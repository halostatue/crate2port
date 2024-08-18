use std::fmt;
use std::io::{self, Cursor, Read};

use cargo2port::{lockfile_from_path, lockfile_from_stdin, lockfile_from_str};
use cargo_lock::{self, Lockfile};
use flate2::read::GzDecoder;
use tar::Archive;

// Result type with the crate's [`Error`] type.
type Result<T> = std::result::Result<T, Error>;

/// Error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Errors from cargo_lock
    CargoLock(cargo_lock::Error),

    /// Errors related to crate download
    Download(reqwest::Error),

    /// Errors related to crate lockfile extraction
    Tar(io::ErrorKind),

    /// Missing lockfile in tarball
    MissingLockfile,

    /// Could not parse the crate specification
    Spec(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CargoLock(error) => error.fmt(f),
            Error::Download(error) => error.fmt(f),
            Error::Tar(error) => error.fmt(f),
            Error::Spec(err) => write!(f, "invalid crate specifier: {}", err),
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

pub fn resolve_lockfile(name: &str) -> Result<Lockfile> {
    if name == "-" {
        Ok(lockfile_from_stdin()?)
    } else if name.contains("@") {
        lockfile_from_crates_io(name)
    } else {
        Ok(lockfile_from_path(name)?)
    }
}

/// Retrieve a lockfile from the crate spec provided.
pub fn lockfile_from_crates_io(crate_spec: &str) -> Result<Lockfile> {
    let parts: Vec<&str> = crate_spec.split('@').collect();

    if parts.len() >= 2 {
        let pkg = download_crate(parts[0], parts[1])?;
        let cargo_lock = extract_cargo_lock_from_pkg(&pkg)?;

        return Ok(lockfile_from_str(&cargo_lock)?);
    };

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
    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}/download",
        name, version
    );
    let response = reqwest::blocking::get(url)?.bytes()?;
    Ok(response.to_vec())
}
