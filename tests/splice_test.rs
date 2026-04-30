use std::io::Write;

use crate2port::splice_cargo_crates;
use goldenfile::Mint;

#[test]
fn test_splice_replaces_existing_block() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_replace").unwrap();

    let portfile = "\
PortSystem 1.0
PortGroup cargo 1.0

cargo.crates \\
    foo 1.0.0 abc123 \\
    bar 2.0.0 def456

checksums rmd160 abc
";

    let new_block = "cargo.crates \\
    baz 3.0.0 aaa111";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_preserves_surrounding_content() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_surrounding").unwrap();

    let portfile = "\
# header
cargo.crates \\
    old 1.0.0 aaa
# footer
";

    let new_block = "cargo.crates \\
    new 2.0.0 bbb";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_returns_none_when_no_block() {
    let portfile = "\
PortSystem 1.0
PortGroup cargo 1.0
";

    let new_block = "cargo.crates \\
    foo 1.0.0 abc123";

    assert!(splice_cargo_crates(portfile, new_block).is_none());
}

#[test]
fn test_splice_block_at_end_of_file() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_end_of_file").unwrap();

    let portfile = "\
PortSystem 1.0

cargo.crates \\
    old 1.0.0 aaa
";

    let new_block = "cargo.crates \\
    new 2.0.0 bbb";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_single_line_block_no_continuation() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_single_line").unwrap();

    let portfile = "\
before
cargo.crates
after
";

    let new_block = "cargo.crates \\
    foo 1.0.0 abc123";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_preserves_content_order() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_content_order").unwrap();

    let portfile = "\
line1
line2
cargo.crates \\
    old 1.0.0 aaa \\
    old2 2.0.0 bbb
line3
line4
";

    let new_block = "cargo.crates \\
    new 3.0.0 ccc";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_matches_multiple_spaces_before_continuation() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_multi_space").unwrap();

    let portfile = "\
before
cargo.crates     \\
    old 1.0.0 aaa
after
";

    let new_block = "cargo.crates \\
    new 2.0.0 bbb";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_preserves_original_indentation() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_indentation").unwrap();

    let portfile = "\
before
    cargo.crates \\
        old 1.0.0 aaa
after
";

    let new_block = "cargo.crates \\
    new 2.0.0 bbb";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}

#[test]
fn test_splice_preserves_subport_indentation() {
    let mut mint = Mint::new("tests/support");
    let mut file = mint.new_goldenfile("splice_subport").unwrap();

    let portfile = "\
PortSystem 1.0

subport foo-bar {
    cargo.crates \\
        old 1.0.0 aaa \\
        old2 2.0.0 bbb
}
";

    let new_block = "cargo.crates \\
    new 3.0.0 ccc";

    let result = splice_cargo_crates(portfile, new_block).unwrap();
    write!(file, "{}", result).unwrap();
}
