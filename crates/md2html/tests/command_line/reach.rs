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

/// A handbook whose chapter shows images from three places.
///
/// Its start page links a chapter in a subfolder, which shows an image of the
/// handbook's `images/`, one above the handbook, and a file that is no image,
/// and names a stylesheet of the handbook's folder.
fn handbook(name: &str) -> Folder {
    let folder = Folder::new(name);
    folder.write(
        "manual/README.md",
        "# Manual\n\n[Chapter](part/chapter.md)\n",
    );
    folder.write(
        "manual/part/chapter.md",
        "---\ncss: ../site.css\n---\n# Chapter\n\n![in](../images/in.png) ![above](../../above.png) ![text](../notes.png)\n",
    );
    folder.write("manual/images/in.png", PNG);
    folder.write("manual/notes.png", "not an image, a secret\n");
    folder.write("manual/site.css", CSS);
    folder.write("above.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-ABOVE-MARK");
    folder
}

/// A chapter of a handbook embeds and packs an image from anywhere in the
/// handbook's folder.
///
/// Never one above it without `--root` (each such image
/// a warning on the chapter, on every run), never a file that is no image,
/// and never a stylesheet outside its own folder.
///
/// # Panics
///
/// When the handbook's image is not in the page or the book, the image
/// above it or the file that is no image is, the stylesheet is read, a
/// run ends otherwise than with 0, or the image above is not named.
#[test]
fn a_chapter_reads_images_from_its_handbooks_folder_only() {
    let folder = handbook("handbook-images");
    let ran = folder.run(&["--embed", "--epub", "manual"]);
    ran.ends_with(0);
    let page = std::fs::read(folder.path().join("manual/part/chapter.html")).expect("the page");
    assert!(
        holds(&page, b"data:image/png;base64,"),
        "the handbook's image"
    );
    assert!(
        !holds(&page, b"--root-mark"),
        "no stylesheet above the chapter"
    );
    let book = std::fs::read(folder.path().join("manual/part/chapter.epub")).expect("the book");
    assert!(holds(&book, b"ROOT-MARK"), "the handbook's image is packed");
    assert!(
        !holds(&book, b"ABOVE-MARK"),
        "the image above is not packed"
    );
    assert!(!holds(&book, b"a secret"), "no file that is not an image");
    let chapter = std::path::Path::new("manual")
        .join("part")
        .join("chapter.md");
    assert!(
        ran.err.contains(&format!(
            "{}:6: `../../above.png` lies outside the handbook (manual): it is not put inside the pages or a book",
            chapter.display()
        )),
        "{}",
        ran.err
    );
    assert!(
        ran.err.contains("`../notes.png` is not embedded"),
        "{}",
        ran.err
    );
    let checked = folder.run(&["--check", "manual"]);
    checked.ends_with(1);
    assert!(
        checked.err.contains("lies outside the handbook"),
        "{}",
        checked.err
    );
}

/// The folder md2html runs in is no handbook's folder for being that.
///
/// `md2html .` there is a handbook still, but its chapter reads images
/// from its own folder only, the image beside it is no warning of the
/// handbook's, and `--root .` allows the rest.
///
/// # Panics
///
/// When the folder is no handbook, the image beside the chapter's folder
/// is embedded without `--root .` or named as outside the handbook, or it
/// is not embedded with `--root .`.
#[test]
fn the_folder_md2html_runs_in_is_no_handbooks_for_being_that() {
    let folder = handbook("handbook-here");
    let here = folder.path().join("manual");
    let page = here.join("part/chapter.html");
    let ran = Ran::of(folder.md2html(&["--embed", "."]).current_dir(&here));
    ran.ends_with(0);
    assert!(
        !ran.err.contains("lies outside the handbook"),
        "{}",
        ran.err
    );
    let alone = std::fs::read(&page).expect("the page");
    assert!(holds(&alone, b"<ol class=\"book\">"), "a handbook still");
    assert!(
        !holds(&alone, b"data:image/png;base64,"),
        "not for being here"
    );
    Ran::of(
        folder
            .md2html(&["--embed", "--root", ".", "."])
            .current_dir(&here),
    )
    .ends_with(0);
    let rooted = std::fs::read(&page).expect("the page");
    assert!(
        holds(&rooted, b"data:image/png;base64,"),
        "--root . allows it"
    );
}
