use std::collections::BTreeSet;
use std::io::{self, Read};
use std::str::FromStr;

use cargo_lock::{self, Lockfile, Package};

/// Result type with the `cargo2port` crate's [`Error`] type.
pub type Result<T> = std::result::Result<T, cargo_lock::Error>;

// The amount of space that will always be put between the name and version when in
// AlignmentMode::Justify, in addition to any other amount calculated.
const JUSTIFIED_BASE_WIDTH: usize = 5;

#[derive(PartialEq)]
pub enum AlignmentMode {
    Normal,
    Maxlen,
    Multiline,
    Justify,
}

/// Load a Cargo.lock file from the filename provided.
/// This is a thin wrapper around cargo_lockfile::Lockfile::load.
pub fn lockfile_from_path(filename: &str) -> Result<Lockfile> {
    Lockfile::load(filename)
}

/// Parse a Cargo.lock file from the contents provided.
/// This is a thin wrapper around cargo_lockfile::Lockfile::from_str.
pub fn lockfile_from_str(contents: &str) -> Result<Lockfile> {
    Lockfile::from_str(contents)
}

/// Load Cargo.lock data from stdin and parse it from the resulting string.
pub fn lockfile_from_stdin() -> Result<Lockfile> {
    let mut stdin = io::stdin().lock();
    let mut contents = String::new();
    stdin.read_to_string(&mut contents)?;
    lockfile_from_str(&contents)
}

/// Resolve packages from a vector of Lockfile entries to a de-duplicated sorted vector of
/// Packages.
///
/// Packages without a checksum are omitted (this usually happens for the package with the
/// Cargo.lock file or files being processed).
pub fn resolve_lockfile_packages(lockfiles: &Vec<Lockfile>) -> Result<Vec<Package>> {
    let mut packageset: BTreeSet<&Package> = BTreeSet::new();

    for lockfile in lockfiles {
        for package in &lockfile.packages {
            if package.checksum.is_none() {
                continue;
            }

            packageset.insert(package);
        }
    }

    let mut packages = Vec::new();

    for package in packageset {
        packages.push(package.clone())
    }

    packages.sort();

    Ok(packages)
}

/// Return the portfile `cargo.crates` block given a vector of packages and AlignmentMode.
/// It is assumed that the package vector is already sorted and deduplicated.
pub fn format_cargo_crates(packages: Vec<Package>, mode: AlignmentMode) -> String {
    let mut output = String::new();

    let mut name_min_width = 0;
    let mut version_min_width = 0;
    let mut package_max_width = 0;

    if mode == AlignmentMode::Maxlen {
        for package in &packages {
            let name_len = package.name.as_str().len();
            if name_len > name_min_width {
                name_min_width = name_len;
            }

            let version_len = package.version.to_string().len();
            if version_len > version_min_width {
                version_min_width = version_len;
            }
        }
    } else if mode == AlignmentMode::Justify {
        for package in &packages {
            let len = package.name.as_str().len() + package.version.to_string().len();
            if len > package_max_width {
                package_max_width = len;
            }
        }
    }

    output.push_str("cargo.crates");

    for package in packages {
        if let Some(checksum) = &package.checksum {
            output.push_str(" \\\n");

            let line = match mode {
                AlignmentMode::Maxlen => format!(
                    "    {:<name_width$}  {:<version_width$}  {}",
                    package.name,
                    package.version,
                    checksum,
                    name_width = name_min_width,
                    version_width = version_min_width
                ),
                AlignmentMode::Multiline => format!(
                    "    {} \\\n    {} \\\n    {}",
                    package.name, package.version, checksum
                ),
                AlignmentMode::Normal => format!(
                    "    {:<name_width$}  {:>version_width$}  {}",
                    package.name,
                    package.version,
                    checksum,
                    name_width = 28,
                    version_width = 8
                ),
                AlignmentMode::Justify => {
                    let version_len = package.version.to_string().len();
                    let space_width = package_max_width - package.name.as_str().len() - version_len
                        + JUSTIFIED_BASE_WIDTH;

                    format!(
                        "    {}{:space_width$}{:>version_width$}  {}",
                        package.name,
                        " ",
                        package.version,
                        checksum,
                        space_width = space_width,
                        version_width = version_len,
                    )
                }
            };

            output.push_str(&line);
        }
    }

    output
}

/// Returns true if `line` ends with a single backslash (Tcl line continuation).
fn is_continuation_line(line: &str) -> bool {
    line.ends_with('\\') && !line.ends_with("\\\\")
}

/// Splice a new `cargo.crates` block into a Portfile's contents,
/// replacing the existing block.
///
/// Returns `None` if no existing `cargo.crates` block is found.
pub fn splice_cargo_crates(portfile_contents: &str, cargo_crates_block: &str) -> Option<String> {
    let lines: Vec<&str> = portfile_contents.lines().collect();

    // Find the start of the cargo.crates block.
    let start = lines.iter().position(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with("cargo.crates") {
            return false;
        }
        let rest = trimmed["cargo.crates".len()..].trim();
        rest.is_empty() || is_continuation_line(rest)
    })?;

    // Find the end of the block: continuation lines end with '\'
    let mut end = start;
    while end < lines.len() && is_continuation_line(lines[end]) {
        end += 1;
    }
    // `end` is now the last line of the block (the one without trailing \)

    // Detect indentation from the original cargo.crates line
    let original_line = lines[start];
    let indent = &original_line[..original_line.len() - original_line.trim_start().len()];

    // Detect whether the original uses tabs so we can convert the 4-space indent
    // in format_cargo_crates output to match
    let uses_tabs = indent.contains('\t');

    let mut output = String::new();

    // Everything before the block
    for line in &lines[..start] {
        output.push_str(line);
        output.push('\n');
    }

    // The new block, with original indentation applied
    for (i, line) in cargo_crates_block.lines().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        if !line.is_empty() {
            output.push_str(indent);
            if uses_tabs {
                output.push_str(&line.replace("    ", "\t"));
            } else {
                output.push_str(line);
            }
        }
    }
    output.push('\n');

    // Everything after the block
    if end + 1 < lines.len() {
        for line in &lines[end + 1..] {
            output.push_str(line);
            output.push('\n');
        }
    }

    Some(output)
}
