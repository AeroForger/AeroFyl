use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::backend::x86_64::elf::ElfError;
use crate::backend::x86_64::emitter::EmitError;
use crate::backend::x86_64::lower::BackendError;
use crate::frontend::diagnostics::Diagnostic;
use crate::frontend::source::SourceMap;
use crate::middle::hir::HirModule;
use crate::middle::ir::IrModule;
use crate::middle::verify::IrVerificationError;
use crate::project::loader::{LoadError, load_source};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Emit {
    #[default]
    Check,
    Elf,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompileOptions {
    pub emit: Emit,
}

impl CompileOptions {
    pub const fn check() -> Self {
        Self { emit: Emit::Check }
    }
    pub const fn elf() -> Self {
        Self { emit: Emit::Elf }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompileOutput {
    pub hir: HirModule,
    pub ir: IrModule,
    pub artifact: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum CompileError {
    Load(LoadError),
    Frontend {
        diagnostics: Vec<Diagnostic>,
        sources: SourceMap,
    },
    Ir(Vec<IrVerificationError>),
    Backend(BackendError),
    Emit(EmitError),
    Elf(ElfError),
    Write {
        path: PathBuf,
        source: io::Error,
    },
}

impl CompileError {
    pub fn render(&self) -> String {
        match self {
            Self::Frontend {
                diagnostics,
                sources,
            } => diagnostics
                .iter()
                .map(|diagnostic| diagnostic.render(sources))
                .collect::<Vec<_>>()
                .join("\n\n"),
            Self::Ir(errors) => errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
            _ => self.to_string(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(formatter, "{error}"),
            Self::Frontend { diagnostics, .. } => write!(
                formatter,
                "compilation failed with {} diagnostic(s)",
                diagnostics.len()
            ),
            Self::Ir(errors) => write!(
                formatter,
                "internal compiler error: IR verification failed with {} error(s)",
                errors.len()
            ),
            Self::Backend(error) => write!(formatter, "{error}"),
            Self::Emit(error) => write!(formatter, "{error}"),
            Self::Elf(error) => write!(formatter, "{error}"),
            Self::Write { path, source } => {
                write!(formatter, "could not write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for CompileError {}

pub fn compile(
    source: &str,
    path: impl Into<PathBuf>,
    options: CompileOptions,
) -> Result<CompileOutput, CompileError> {
    let mut sources = SourceMap::new();
    let file = sources.add(path, source);
    let tokens = crate::frontend::lexer::lex(file, source).map_err(|diagnostics| {
        CompileError::Frontend {
            diagnostics,
            sources: sources.clone(),
        }
    })?;
    let ast =
        crate::frontend::parser::parse(tokens).map_err(|diagnostics| CompileError::Frontend {
            diagnostics,
            sources: sources.clone(),
        })?;
    let hir =
        crate::frontend::semantic::analyze(&ast).map_err(|diagnostics| CompileError::Frontend {
            diagnostics,
            sources: sources.clone(),
        })?;
    if options.emit == Emit::Elf {
        crate::frontend::semantic::validate_executable(&hir).map_err(|diagnostics| {
            CompileError::Frontend {
                diagnostics,
                sources: sources.clone(),
            }
        })?;
    }
    let ir = crate::middle::lower::lower(&hir);
    crate::middle::verify::verify_module(&ir).map_err(CompileError::Ir)?;

    let artifact = match options.emit {
        Emit::Check => None,
        Emit::Elf => {
            let machine =
                crate::backend::x86_64::lower::lower(&ir).map_err(CompileError::Backend)?;
            let code = crate::backend::x86_64::emitter::emit_module(&machine)
                .map_err(CompileError::Emit)?;
            Some(
                crate::backend::x86_64::elf::write_executable(&code.bytes, code.entry_offset)
                    .map_err(CompileError::Elf)?,
            )
        }
    };
    Ok(CompileOutput { hir, ir, artifact })
}

pub fn compile_file(path: &Path, options: CompileOptions) -> Result<CompileOutput, CompileError> {
    let source = load_source(path).map_err(CompileError::Load)?;
    compile(&source, path, options)
}

/// Compiles one source file and writes a directly executable bootstrap artifact.
pub fn compile_file_to_path(
    source_path: &Path,
    output_path: &Path,
) -> Result<CompileOutput, CompileError> {
    let output = compile_file(source_path, CompileOptions::elf())?;
    let artifact = output
        .artifact
        .as_deref()
        .ok_or_else(|| CompileError::Write {
            path: output_path.to_owned(),
            source: io::Error::other("executable compilation produced no artifact"),
        })?;
    fs::write(output_path, artifact).map_err(|source| CompileError::Write {
        path: output_path.to_owned(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(output_path, permissions).map_err(|source| CompileError::Write {
            path: output_path.to_owned(),
            source,
        })?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_pipeline_reaches_ir() {
        let output = compile(
            "private int f() { return 1; }",
            "memory.fyl",
            CompileOptions::check(),
        )
        .unwrap();
        assert_eq!(output.ir.functions.len(), 1);
        assert!(output.artifact.is_none());
    }

    #[test]
    fn executable_pipeline_emits_elf_with_startup_entry() {
        let output = compile("public void main() {}", "memory.fyl", CompileOptions::elf()).unwrap();
        let artifact = output.artifact.unwrap();
        assert_eq!(&artifact[..4], b"\x7fELF");
        assert_eq!(
            u64::from_le_bytes(artifact[24..32].try_into().unwrap()),
            0x401000
        );
    }

    #[test]
    fn carries_source_map_for_diagnostics() {
        let error = compile(
            "private int f() { return missing; }",
            "memory.fyl",
            CompileOptions::check(),
        )
        .unwrap_err();
        assert!(error.render().contains("memory.fyl:1:"));
    }

    #[test]
    fn executable_rejects_invalid_main_signature() {
        let error = compile(
            "public int main(int value) { return value; }",
            "memory.fyl",
            CompileOptions::elf(),
        )
        .unwrap_err();
        let rendered = error.render();
        assert!(rendered.contains("must return `void`"));
        assert!(rendered.contains("must take no parameters"));
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn compiles_and_runs_acceptance_program() {
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "aerofyl-bootstrap-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let source_path = directory.join("program.fyl");
        let output_path = directory.join("program");
        fs::write(
            &source_path,
            "public int add(int x, int y) { return x + y; } public void main() { int result = add(2, 3); }",
        )
        .unwrap();
        compile_file_to_path(&source_path, &output_path).unwrap();
        let status = Command::new(&output_path).status().unwrap();
        assert!(status.success());
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn run_helper_as_exit_status(source: &str, helper_name: &str) -> i32 {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};

        use crate::backend::x86_64::instruction::Instruction;
        use crate::backend::x86_64::register::Register;

        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let output = compile(source, "control-flow.fyl", CompileOptions::check()).unwrap();
        let helper = output
            .ir
            .functions
            .iter()
            .find(|function| function.name == helper_name)
            .unwrap()
            .symbol;
        let mut machine = crate::backend::x86_64::lower::lower(&output.ir).unwrap();
        machine.startup = vec![
            Instruction::Call(helper),
            Instruction::MoveRegister {
                destination: Register::Rdi,
                source: Register::Rax,
            },
            Instruction::MoveImmediate64 {
                destination: Register::Rax,
                value: 60,
            },
            Instruction::Syscall,
        ];
        let code = crate::backend::x86_64::emitter::emit_module(&machine).unwrap();
        let elf =
            crate::backend::x86_64::elf::write_executable(&code.bytes, code.entry_offset).unwrap();
        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aerofyl-control-flow-test-{}-{timestamp}-{unique}",
            std::process::id()
        ));
        fs::write(&path, elf).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let status = Command::new(&path).status().unwrap();
        fs::remove_file(path).unwrap();
        status.code().unwrap()
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_while_loop_and_observes_result() {
        let source = "public int test() { int x = 0; while (x < 5) { x = x + 1; } return x; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 5);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_distinguishable_if_else_branch() {
        let source = "public int test() { int x = 3; if (x > 5) { return 7; } else { return 9; } } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 9);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_short_circuit_without_evaluating_rhs() {
        let source = "public int test() { if (false && (1 / 0 == 0)) { return 1; } if (true || (1 / 0 == 0)) { return 5; } return 2; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 5);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_break_and_continue_targets() {
        let source = "public int test() { int x = 0; while (x < 10) { x = x + 1; if (x < 5) { continue; } if (x == 5) { break; } } return x; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 5);
    }
}
