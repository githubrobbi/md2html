// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What a document may make md2html read: its own folder and below, and
//! the folder `--root` names; never the folder md2html runs in for being
//! that, and nothing from standard input without `--root`.

use super::{Folder, Ran};

/// An image md2html takes for a PNG, with a mark to find its bytes by.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-ROOT-MARK";

/// A stylesheet with a mark to find it by in a page.
const CSS: &str = "body{--root-mark:1}";

/// A document in `docs/` that names a stylesheet and an image in
/// `assets/`, beside its folder.
const PAGE: &str = "---\ncss: ../assets/site.css\n---\n# Page\n\n![logo](../assets/logo.png)\n";

/// A folder with `docs/page.md` ([`PAGE`]) and `assets/site.css` and
/// `assets/logo.png` beside `docs/`.
fn repository(name: &str) -> Folder {
    let folder = Folder::new(name);
    folder.write("docs/page.md", PAGE);
    folder.write("assets/site.css", CSS);
    folder.write("assets/logo.png", PNG);
    folder
}

/// Whether `bytes` hold `mark`.
fn holds(bytes: &[u8], mark: &[u8]) -> bool {
    bytes.windows(mark.len()).any(|window| window == mark)
}

/// Runs md2html with `args` in the folder's `docs/`.
///
/// # Panics
///
/// When the program cannot be started.
fn run_in_docs(folder: &Folder, args: &[&str]) -> Ran {
    Ran::of(folder.md2html(args).current_dir(folder.path().join("docs")))
}

/// A document in a folder, rendered from the folder above it, reads no
/// stylesheet or image beside its folder: not into the page, not into the
/// book. Each file refused is a warning that names `--root`.
///
/// # Panics
///
/// When the stylesheet or the image reaches the page or the book, a run
/// ends otherwise than with 0, or a refusal is not told or does not name
/// `--root`.
#[test]
fn a_document_reads_nothing_above_its_folder_without_root() {
    let folder = repository("no-root");
    let embedded = folder.run(&["--embed", "docs/page.md"]);
    embedded.ends_with(0);
    let page = folder.read("docs/page.html");
    assert!(!page.contains("--root-mark"), "no stylesheet read");
    assert!(!page.contains("data:image/png"), "no image embedded");
    let packed = folder.run(&["--epub", "docs/page.md"]);
    packed.ends_with(0);
    let book = std::fs::read(folder.path().join("docs/page.epub")).expect("the book");
    assert!(!holds(&book, b"ROOT-MARK"), "no image packed");
    for ran in [embedded, packed] {
        let refusals: Vec<&str> = ran
            .err
            .lines()
            .filter(|line| line.contains("outside the document's folder"))
            .collect();
        assert_eq!(refusals.len(), 2, "{}", ran.err);
        assert!(
            refusals
                .iter()
                .all(|line| line.contains("(--root <folder>")),
            "{}",
            ran.err
        );
    }
}

/// `--root` names a folder the document may read from as from its own:
/// `--root .` from the folder above it, `--root ..` from its own folder.
///
/// # Panics
///
/// When the stylesheet does not reach the page, the image is not embedded
/// or not packed into the book, or anything is refused or a run ends
/// otherwise than with 0.
#[test]
fn root_lets_a_document_read_beside_its_folder() {
    let folder = repository("root");
    for asked in ["--embed", "--epub"] {
        for from_docs in [false, true] {
            let ran = if from_docs {
                run_in_docs(&folder, &[asked, "--root=..", "page.md"])
            } else {
                folder.run(&[asked, "--root", ".", "docs/page.md"])
            };
            ran.ends_with(0);
            assert!(!ran.err.contains("outside"), "{}", ran.err);
            let page = folder.read("docs/page.html");
            assert!(page.contains("--root-mark"), "the stylesheet read");
            if asked == "--embed" {
                assert!(page.contains("data:image/png"), "the image embedded");
            } else {
                let book = folder.path().join("docs/page.epub");
                assert!(
                    holds(&std::fs::read(&book).expect("the book"), b"ROOT-MARK"),
                    "the image packed"
                );
                std::fs::remove_file(book).expect("the book removed");
            }
            std::fs::remove_file(folder.path().join("docs/page.html")).expect("the page removed");
        }
    }
}

/// A document cannot widen what it may read: `root` in its front matter is
/// refused, with a warning that names `--root`, and reads nothing.
///
/// # Panics
///
/// When the stylesheet or the image is read with `root: ..` in the front
/// matter, or the refusal of `root` is not told.
#[test]
fn front_matter_cannot_widen_what_a_document_reads() {
    let folder = repository("front-root");
    folder.write("docs/page.md", PAGE.replacen("---\n", "---\nroot: ..\n", 1));
    let ran = folder.run(&["--embed", "docs/page.md"]);
    ran.ends_with(0);
    let page = folder.read("docs/page.html");
    assert!(!page.contains("--root-mark"), "no stylesheet read");
    assert!(!page.contains("data:image/png"), "no image embedded");
    assert!(
        ran.err
            .contains("`root` would widen the folders a document may read from"),
        "{}",
        ran.err
    );
}

/// Standard input has no folder: without `--root` its document reads no
/// file, not even from the folder md2html runs in; `--root .` lets it read
/// there.
///
/// # Panics
///
/// When the stylesheet or the image of the folder md2html runs in reaches
/// the page without `--root`, or does not with `--root .`; or when the
/// refusal is not told or does not name `--root`.
#[test]
fn standard_input_reads_nothing_without_root() {
    let folder = Folder::new("stdin-root");
    folder.write("site.css", CSS);
    folder.write("out/logo.png", PNG);
    let text = "---\ncss: site.css\n---\n# Page\n\n![logo](logo.png)\n";
    let alone = folder.run_with_input(&["--embed", "-o", "out/page.html", "-"], text);
    alone.ends_with(0);
    let page = folder.read("out/page.html");
    assert!(!page.contains("--root-mark"), "no stylesheet read");
    assert!(!page.contains("data:image/png"), "no image embedded");
    assert!(
        alone.err.contains("standard input has none (--root"),
        "{}",
        alone.err
    );
    let rooted = folder.run_with_input(
        &["--embed", "--root", ".", "-o", "out/page.html", "-"],
        text,
    );
    rooted.ends_with(0);
    let widened = folder.read("out/page.html");
    assert!(widened.contains("--root-mark"), "the stylesheet read");
    assert!(widened.contains("data:image/png"), "the image embedded");
}

/// `--root` must name a folder that is there: a missing path, a file, or
/// `root` given with `--set` ends the run with 64 before any page, and
/// says why.
///
/// # Panics
///
/// When one of them ends otherwise, writes a page, or does not name what
/// is wrong.
#[test]
fn root_that_is_no_folder_ends_with_64() {
    let folder = repository("root-usage");
    for (args, said) in [
        (["--root", "missing"], "--root missing: "),
        (
            ["--root", "docs/page.md"],
            "--root docs/page.md: not a folder",
        ),
        (
            ["--set", "root=."],
            "only the command line gives one (--root)",
        ),
    ] {
        let ran = folder.run(&[args[0], args[1], "docs/page.md"]);
        ran.ends_with(64);
        assert!(ran.err.contains(said), "{args:?}: {}", ran.err);
        assert!(!folder.has("docs/page.html"), "{args:?} wrote a page");
    }
}
