use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::process;

use cargo_lock::{self, Lockfile};
use crate2port::{
    format_cargo_crates, lockfile_from_path, lockfile_from_stdin, lockfile_from_str,
    resolve_lockfile_packages, splice_cargo_crates, AlignmentMode,
};
use flate2::read::GzDecoder;
use tar::Archive;

// Result type with the crate's [`Error`] type.
type Result<T> = std::result::Result<T, Error>;

/// Error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    CargoLock(cargo_lock::Error),
    Download(reqwest::Error),
    Tar(io::ErrorKind),
    MissingLockfile,
    Spec(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CargoLock(error) => error.fmt(f),
            Error::Download(error) => error.fmt(f),
            Error::Tar(error) => error.fmt(f),
            Error::Spec(err) => write!(f, "invalid crate specifier: {err}"),
            Error::MissingLockfile => write!(f, "crate missing Cargo.lock file"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Tar(err.kind())
    }
}

impl From<cargo_lock::Error> for Error {
    fn from(err: cargo_lock::Error) -> Self {
        Error::CargoLock(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Download(err)
    }
}

impl std::error::Error for Error {}

fn main() {
    let mut mode = AlignmentMode::Justify;
    let mut files: Vec<String> = vec![];
    let mut portfile_path: Option<String> = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match &arg[..] {
            "" => continue,
            "--help" | "-?" | "-h" => print_usage(0),
            "--align=plain" => mode = AlignmentMode::Normal,
            "--align=maxlen" => mode = AlignmentMode::Maxlen,
            "--align=multiline" => mode = AlignmentMode::Multiline,
            "--align=justify" => mode = AlignmentMode::Justify,
            "-P" | "--portfile" => {
                portfile_path = Some(args.next().unwrap_or_else(|| {
                    eprintln!("Error: -P requires a path to a Portfile");
                    process::exit(1);
                }));
            }
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
                eprintln!("{error}");
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

            let block = format_cargo_crates(packages, mode);

            match portfile_path {
                Some(ref path) => update_portfile(path, &block),
                None => println!("{block}"),
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(1)
        }
    }
}

fn resolve_lockfile(name: &str) -> Result<Lockfile> {
    if name == "-" {
        Ok(lockfile_from_stdin()?)
    } else if name.contains("@") {
        lockfile_from_crates_io(name)
    } else {
        Ok(lockfile_from_path(name)?)
    }
}

fn lockfile_from_crates_io(crate_spec: &str) -> Result<Lockfile> {
    let parts: Vec<&str> = crate_spec.split('@').collect();

    if parts.len() >= 2 {
        let pkg = download_crate(parts[0], parts[1])?;
        let cargo_lock = extract_cargo_lock_from_pkg(&pkg)?;
        return Ok(lockfile_from_str(&cargo_lock)?);
    }

    Err(Error::Spec(crate_spec.to_string()))
}

fn extract_cargo_lock_from_pkg(pkg: &[u8]) -> Result<String> {
    let gzip = GzDecoder::new(Cursor::new(pkg));
    let mut archive = Archive::new(gzip);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.to_path_buf();

        if path.ends_with("Cargo.lock") {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            return Ok(contents);
        }
    }

    Err(Error::MissingLockfile)
}

fn download_crate(name: &str, version: &str) -> Result<Vec<u8>> {
    let url = format!("https://crates.io/api/v1/crates/{name}/{version}/download");
    let response = reqwest::blocking::get(url)?.bytes()?;
    Ok(response.to_vec())
}

fn update_portfile(path: &str, cargo_crates_block: &str) {
    let portfile_path = Path::new(path);

    let contents = fs::read_to_string(portfile_path).unwrap_or_else(|e| {
        eprintln!("Error reading Portfile '{path}': {e}");
        process::exit(1);
    });

    let updated = splice_cargo_crates(&contents, cargo_crates_block).unwrap_or_else(|| {
        eprintln!("Error: no cargo.crates block found in '{path}'");
        process::exit(1);
    });

    // Write to a temporary file in the same directory, then rename into place
    // so the Portfile is never left in a partially-written state.
    let dir = portfile_path.parent().unwrap_or(Path::new("."));
    let tmp_path = dir.join(format!(".Portfile.cargo2port.{}.tmp", process::id()));

    fs::write(&tmp_path, &updated).unwrap_or_else(|e| {
        eprintln!(
            "Error writing temporary file '{}': {}",
            tmp_path.display(),
            e
        );
        process::exit(1);
    });

    fs::rename(&tmp_path, portfile_path).unwrap_or_else(|e| {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("Error replacing Portfile '{path}': {e}");
        process::exit(1);
    });

    eprintln!("Updated {path}");
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
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn print_usage(code: i32) {
    let arg0 = env::args().next().unwrap_or("crate2port".to_owned());
    eprintln!(
        "Usage: {arg0} [options] <path/to/Cargo.lock | crate@version>...

Generate a cargo.crates block for a MacPorts Portfile from one or more
Cargo.lock files, or download from crates.io via crate@version.

Options:
  -h, --help                          Print this help message
  --align=plain|maxlen|multiline|justify  Set alignment mode (default: justify)
  -P, --portfile <path>               Update the cargo.crates block in <path> in place"
    );
    process::exit(code);
}
