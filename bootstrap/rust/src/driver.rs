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
    let verified = crate::middle::verify::verify_module(&ir).map_err(CompileError::Ir)?;

    let artifact = match options.emit {
        Emit::Check => None,
        Emit::Elf => {
            let machine =
                crate::backend::x86_64::lower::lower(&verified).map_err(CompileError::Backend)?;
            let code = crate::backend::x86_64::emitter::emit_module(&machine)
                .map_err(CompileError::Emit)?;
            Some(
                crate::backend::x86_64::elf::write_executable(
                    &code.bytes,
                    code.entry_offset,
                    &code.read_only_data,
                    code.data_offset,
                )
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

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    static NATIVE_EXECUTION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

        let _execution = NATIVE_EXECUTION_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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

        let _execution = NATIVE_EXECUTION_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let output = compile(source, "control-flow.fyl", CompileOptions::check()).unwrap();
        let helper = output
            .ir
            .functions
            .iter()
            .find(|function| function.name == helper_name)
            .unwrap()
            .symbol;
        let verified = crate::middle::verify::verify_module(&output.ir).unwrap();
        let mut machine = crate::backend::x86_64::lower::lower(&verified).unwrap();
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
        let elf = crate::backend::x86_64::elf::write_executable(
            &code.bytes,
            code.entry_offset,
            &code.read_only_data,
            code.data_offset,
        )
        .unwrap();
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
    fn run_main_with_arguments(source: &str, arguments: &[&str]) -> i32 {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;
        use std::sync::atomic::{AtomicU64, Ordering};

        let _execution = NATIVE_EXECUTION_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let output = compile(source, "arguments.fyl", CompileOptions::elf()).unwrap();
        let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "aerofyl-arguments-test-{}-{unique}",
            std::process::id()
        ));
        fs::write(&path, output.artifact.unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let status = Command::new(&path).args(arguments).status().unwrap();
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

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_struct_acceptance_program() {
        let source = "struct Point { int x; int y; } public int test() { Point point = Point { x: 10, y: 20 }; point.x = point.x + 5; return point.x; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 15);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_enum_acceptance_program() {
        let source = "enum State { idle, running, stopped } public int test() { State state = State.running; if (state == State.running) { return 1; } return 0; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 1);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_struct_with_enum_acceptance_program() {
        let source = "enum TokenKind { identifier, integer, eof } struct Token { TokenKind kind; int line; } public int test() { Token token = Token { kind: TokenKind.identifier, line: 1 }; if (token.kind == TokenKind.identifier) { token.line = 42; } return token.line; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 42);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_independent_whole_struct_copies() {
        let source = "struct Point { int x; int y; } public int test() { Point a = Point { x: 10, y: 20 }; Point b = a; b.x = 50; return a.x + b.x; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 60);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_independent_whole_struct_assignment() {
        let source = "struct Point { int x; } public int test() { Point a = Point { x: 10 }; Point b = Point { x: 1 }; b = a; b.x = 50; return a.x + b.x; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 60);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_list_of_structs_acceptance_program() {
        let source = "enum tokenKind { identifier, integer, eof } struct Token { tokenKind kind; int line; } public int test() { list Token tokens = []; Token a = Token { kind: tokenKind.identifier, line: 10 }; Token b = Token { kind: tokenKind.integer, line: 20 }; tokens.push(a); tokens.push(b); Token first = tokens[0]; Token last = tokens.pop(); return first.line + last.line + tokens.length; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 31);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_token_struct_with_sliced_string() {
        let source = "enum tokenKind { identifier, integer, eof } struct Token { tokenKind kind; string lexeme; int line; } public int test() { string source = \"hello 123\"; list Token tokens = []; Token token = Token { kind: tokenKind.identifier, lexeme: source.slice(0, 5), line: 1 }; tokens.push(token); Token first = tokens[0]; if (first.lexeme == \"hello\") { return first.lexeme.length; } return 0; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 5);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_byte_oriented_string_slices() {
        let source = "public int test() { string source = \"Aerofyl\"; string a = source.slice(0, 4); string b = source.slice(4, 7); string empty = source.slice(3, 3); string full = source.slice(0, source.length); string utf8 = \"é\".slice(0, 2); string byte = utf8.slice(0, 1); if (a == \"Aero\" && b == \"fyl\" && empty == \"\" && full == source && utf8 == \"é\" && byte.length == 1 && byte.byte(0) == 195) { return a.length + b.length; } return 0; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 7);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn struct_list_growth_and_indexed_store_preserve_complete_values() {
        let source = "struct Pair { int x; string name; int y; } public int test() { list Pair values = []; Pair item = Pair { x: 1, name: \"kept\", y: 2 }; values.push(item); values.push(item); values.push(item); values.push(item); values.push(item); Pair replacement = Pair { x: 10, name: \"new\", y: 20 }; values[0] = replacement; Pair loaded = values[0]; loaded.x = 99; Pair stored = values[0]; Pair last = values.pop(); if (stored.name == \"new\" && last.name == \"kept\") { return stored.x + stored.y + last.x + last.y + values.length; } return 0; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 37);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_fixed_array_acceptance_program() {
        let source = "public int test() { int[4] values = [10, 20, 30, 40]; values[1] = 25; return values[1] + values.length; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 29);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_list_acceptance_program() {
        let source = "public int test() { list int values = [1, 2, 3]; values.push(4); values[0] = 10; int last = values.pop(); return values[0] + last + values.length; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 17);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_scalar_collection_element_types() {
        let source = "enum State { idle, ready } public int test() { bool[2] flags = [false, true]; flags[0] = flags[1]; list State states = [State.idle]; states.push(State.ready); State last = states.pop(); char[1] chars = ['x']; list char letters = [chars[0]]; letters.push('y'); char final = letters.pop(); if (flags[0] && last == State.ready) { return chars.length + states.length + letters.length; } return 0; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 3);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_string_equality_acceptance_program() {
        let source = "public int test() { string a = \"aero\"; string b = \"aero\"; if (a == b && a != \"other\") { return 1; } return 0; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 1);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_string_concat_acceptance_program() {
        let source = "public int test() { string a = \"Aero\"; string b = \"fyl\"; string c = a + b; if (c == \"Aerofyl\") { return c.length; } return 0; } public void main() { int result = test(); }";
        assert_eq!(run_helper_as_exit_status(source, "test"), 7);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn array_bounds_failure_exits_with_status_70() {
        let source =
            "public int test() { int[1] values = [1]; return values[1]; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn negative_array_index_exits_with_status_70() {
        let source =
            "public int test() { int[1] values = [1]; return values[-1]; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn list_bounds_failure_exits_with_status_70() {
        let source =
            "public int test() { list int values = [1]; return values[1]; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn empty_list_pop_exits_with_status_70() {
        let source = "public int test() { list int values = []; return values.pop(); } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn list_growth_preserves_elements() {
        let source = "public int test() { list int values = [1, 2, 3, 4]; values.push(5); return values[0] + values[4] + values.length; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 11);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_ascii_string_byte_access() {
        let source = "public int test() { string text = \"Aerofyl\"; return text.byte(0); } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 65);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn string_byte_access_uses_raw_utf8_bytes() {
        let source = "public int first() { string text = \"é\"; return text.byte(0); } public int second() { string text = \"é\"; return text.byte(1); } public int length() { string text = \"é\"; return text.length; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "first"), 195);
        assert_eq!(run_helper_as_exit_status(source, "second"), 169);
        assert_eq!(run_helper_as_exit_status(source, "length"), 2);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn string_byte_bounds_fail_with_status_70() {
        let negative = "public int test() { string text = \"a\"; return text.byte(-1); } public void main() {}";
        let at_length = "public int test() { string text = \"a\"; return text.byte(text.length); } public void main() {}";
        assert_eq!(run_helper_as_exit_status(negative, "test"), 70);
        assert_eq!(run_helper_as_exit_status(at_length, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn string_slice_bounds_fail_with_status_70() {
        let negative = "public int test() { string text = \"abc\"; return text.slice(-1, 2).length; } public void main() {}";
        let reversed = "public int test() { string text = \"abc\"; return text.slice(3, 2).length; } public void main() {}";
        let too_high = "public int test() { string text = \"abc\"; return text.slice(0, text.length + 1).length; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(negative, "test"), 70);
        assert_eq!(run_helper_as_exit_status(reversed, "test"), 70);
        assert_eq!(run_helper_as_exit_status(too_high, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn main_observes_zero_one_and_multiple_user_arguments() {
        let zero = "public void main(string[] args) { int[1] trap = [0]; if (args.length != 0) { int fail = trap[1]; } }";
        let one = "public void main(string[] args) { int[1] trap = [0]; if (args.length != 1 || args[0].byte(0) != 111) { int fail = trap[1]; } }";
        let three = "public void main(string[] args) { int[1] trap = [0]; if (args.length != 3 || args[0].byte(0) != 111) { int fail = trap[1]; } }";
        assert_eq!(run_main_with_arguments(zero, &[]), 0);
        assert_eq!(run_main_with_arguments(one, &["one"]), 0);
        assert_eq!(run_main_with_arguments(three, &["one", "two", "three"]), 0);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn reads_files_and_handles_empty_and_multiple_reads() {
        use std::os::unix::fs::PermissionsExt;

        let directory =
            std::env::temp_dir().join(format!("aerofyl-read-file-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let fixture = directory.join("fixture.txt");
        let empty = directory.join("empty.txt");
        fs::write(&fixture, b"abc").unwrap();
        fs::write(&empty, b"").unwrap();
        fs::set_permissions(&fixture, fs::Permissions::from_mode(0o444)).unwrap();
        let success = format!(
            "public int test() {{ string source = readFile(\"{}\"); return source.byte(0) + source.length; }} public void main() {{}}",
            fixture.display()
        );
        let empty_source = format!(
            "public int test() {{ string source = readFile(\"{}\"); return source.length; }} public void main() {{}}",
            empty.display()
        );
        let multiple = format!(
            "public int test() {{ string a = readFile(\"{}\"); string b = readFile(\"{}\"); return a.length + b.length; }} public void main() {{}}",
            fixture.display(),
            fixture.display()
        );
        assert_eq!(run_helper_as_exit_status(&success, "test"), 100);
        assert_eq!(run_helper_as_exit_status(&empty_source, "test"), 0);
        assert_eq!(run_helper_as_exit_status(&multiple, "test"), 6);
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn missing_file_fails_with_status_70() {
        let path =
            std::env::temp_dir().join(format!("aerofyl-definitely-missing-{}", std::process::id()));
        let source = format!(
            "public int test() {{ string source = readFile(\"{}\"); return source.length; }} public void main() {{}}",
            path.display()
        );
        assert_eq!(run_helper_as_exit_status(&source, "test"), 70);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn main_reads_source_path_from_command_line() {
        let fixture = std::env::temp_dir().join(format!(
            "aerofyl-source-argument-{}.fyl",
            std::process::id()
        ));
        fs::write(&fixture, b"abc").unwrap();
        let source = "public void main(string[] args) { int[1] trap = [0]; string source = readFile(args[0]); if (args.length != 1 || source.length != 3 || source.byte(0) != 97) { int fail = trap[1]; } }";
        assert_eq!(
            run_main_with_arguments(source, &[fixture.to_str().unwrap()]),
            0
        );
        fs::remove_file(fixture).unwrap();
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn executes_compiler_representative_byte_scan() {
        let source = "enum tokenKind { character, eof } struct token { tokenKind kind; int value; } public int test() { string source = \"abc\"; list int bytes = []; int i = 0; while (i < source.length) { bytes.push(source.byte(i)); i = i + 1; } return bytes.length; } public void main() {}";
        assert_eq!(run_helper_as_exit_status(source, "test"), 3);
    }
}
