//! Jacquard compiler CLI.
//!
//! Usage:
//!   jacquard compile <input.jac>
//!
//! Reads a `.jac` source file, runs the full compilation pipeline, and writes
//! `<module>.jq.h` and `<module>.jq.cpp` output files.

use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_usage(&args[0]);
        process::exit(1);
    }

    match args[1].as_str() {
        "compile" => {
            let input = &args[2];
            if let Err(e) = compile_file(input) {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown subcommand: {}", args[1]);
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

/// Print usage information to stderr.
fn print_usage(prog: &str) {
    eprintln!("Usage: {prog} compile <input.jac>");
}

/// Read a `.jac` file, compile it, and write the output files.
fn compile_file(input_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(input_path);

    // Derive module name from filename sans extension
    let module_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid input filename: {input_path}"))?;

    // Read source
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {input_path}: {e}"))?;

    // Compile
    let output = jacquard::compile(&source, module_name)?;

    // Write output files in the same directory as the input file
    let parent = path.parent().unwrap_or(Path::new("."));
    let header_path = parent.join(format!("{module_name}.jq.h"));
    let source_path = parent.join(format!("{module_name}.jq.cpp"));

    fs::write(&header_path, &output.header)
        .map_err(|e| format!("Failed to write {}: {e}", header_path.display()))?;
    fs::write(&source_path, &output.source)
        .map_err(|e| format!("Failed to write {}: {e}", source_path.display()))?;

    println!(
        "Compiled {input_path} -> {}, {}",
        header_path.display(),
        source_path.display()
    );

    Ok(())
}