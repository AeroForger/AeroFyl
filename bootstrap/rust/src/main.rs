use std::path::Path;

use aerofyl_bootstrap::driver::{CompileOptions, compile_file, compile_file_to_path};

fn main() {
    let mut arguments = std::env::args_os();
    let program = arguments
        .next()
        .unwrap_or_else(|| "aerofyl-bootstrap".into());
    let remaining: Vec<_> = arguments.collect();
    if remaining.is_empty() {
        print_usage(Path::new(&program));
        std::process::exit(2);
    }

    let result = if remaining.len() == 1 {
        compile_file(Path::new(&remaining[0]), CompileOptions::check())
            .map(|_| "check succeeded".to_owned())
    } else if remaining.len() == 4 && remaining[0] == "compile" && remaining[2] == "-o" {
        let output_path = Path::new(&remaining[3]);
        compile_file_to_path(Path::new(&remaining[1]), output_path)
            .map(|_| format!("wrote {}", output_path.display()))
    } else {
        print_usage(Path::new(&program));
        std::process::exit(2);
    };

    match result {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("{}", error.render());
            std::process::exit(1);
        }
    }
}

fn print_usage(program: &Path) {
    eprintln!("bootstrap usage:");
    eprintln!("  {} <source.fyl>", program.display());
    eprintln!(
        "  {} compile <source.fyl> -o <executable>",
        program.display()
    );
}
