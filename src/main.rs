use std::env;
use std::path::Path;
use std::process;

mod download;

use crate::download::resolve_lockfile;
use cargo2port::{format_cargo_crates, resolve_lockfile_packages, AlignmentMode};
use cargo_lock::Lockfile;

// use crate::lockfile_from_crates_io;

fn main() {
    let mut mode = AlignmentMode::Justify;
    let mut files: Vec<String> = vec![];

    for arg in env::args().skip(1) {
        match &arg[..] {
            "" => continue,
            "--help" => print_usage(0),
            "-?" => print_usage(0),
            "-h" => print_usage(0),
            "--align=plain" => mode = AlignmentMode::Normal,
            "--align=maxlen" => mode = AlignmentMode::Maxlen,
            "--align=multiline" => mode = AlignmentMode::Multiline,
            "--align=justify" => mode = AlignmentMode::Justify,
            _ => match check_path(&arg[..]) {
                Some(path) => files.push(path),
                None => process::exit(1),
            },
        }
    }

    if files.is_empty() {
        files.push("Cargo.lock".to_string())
    }

    let mut lockfiles: Vec<Lockfile> = vec![];

    for name in files {
        match resolve_lockfile(&name) {
            Ok(lockfile) => lockfiles.push(lockfile),
            Err(error) => {
                eprintln!("{}", error);
                process::exit(1)
            }
        }
    }

    match resolve_lockfile_packages(&lockfiles) {
        Ok(packages) => {
            if packages.is_empty() {
                eprintln!("No packages with checksums found.");
                process::exit(0);
            }

            println!("{}", format_cargo_crates(packages, mode));
        }
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1)
        }
    }
}

fn check_path(arg: &str) -> Option<String> {
    if arg == "-" || arg.contains("@") {
        return Some(arg.to_string());
    }

    let path = Path::new(&arg);
    match path.try_exists() {
        Ok(true) => {
            if path.is_file() {
                match path.to_str() {
                    Some(path_str) => Some(path_str.to_string()),
                    None => process::exit(1),
                }
            } else {
                match path.join("Cargo.lock").to_str() {
                    Some(file_path) => check_path(file_path),
                    None => {
                        eprintln!("Error: failure appending Cargo.lock to {arg}");
                        process::exit(1);
                    }
                }
            }
        }
        Ok(false) => {
            eprintln!("Error: cannot find file {arg}");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage(code: i32) {
    let arg0 = env::args().next().unwrap_or("cargo2port".to_owned());
    eprintln!(
        "Usage: {} [--align=plain|maxlen|multiline|justify] <path/to/Cargo.lock>...",
        arg0
    );
    process::exit(code);
}
