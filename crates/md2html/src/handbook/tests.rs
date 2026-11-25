// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Handbooks found among a run's pages, held in memory: the start page,
//! the order, the parts, the pages left over, the translations, the
//! glossary page and what each chapter is given.

use std::path::{Path, PathBuf};

use super::{Asked, Handbooks, Page};
use crate::config::Settings;
use crate::page::{Chapter, Contents, Entry};

/// One page of a run in memory.
struct Held {
    /// Its Markdown file.
    file: PathBuf,
    /// The folder it was found in.
    folder: Option<PathBuf>,
    /// Its page, beside it.
    out: PathBuf,
    /// Its Markdown.
    text: String,
}

/// A run's pages in memory, in the run's order.
struct Pages(Vec<Held>);

impl Pages {
    /// The pages `named` (path under the folder, Markdown) of folder
    /// `folder`, in the order given, which stands for the walk's.
    fn folder(folder: &str, named: &[(&str, &str)]) -> Self {
        Self(
            named
                .iter()
                .map(|(under, text)| {
                    let file = Path::new(folder).join(under);
                    let out = file.with_extension("html");
                    Held {
                        file,
                        folder: Some(PathBuf::from(folder)),
                        out,
                        text: (*text).to_owned(),
                    }
                })
                .collect(),
        )
    }

    /// The pages `named`, each named on the command line itself.
    fn files(named: &[(&str, &str)]) -> Self {
        let mut pages = Self::folder("", named);
        for held in &mut pages.0 {
            held.folder = None;
        }
        pages
    }

    /// The pages as a handbook sees them.
    fn seen(&self) -> Vec<Page<'_>> {
        self.0
            .iter()
            .map(|held| Page {
                file: Some(&held.file),
                folder: held.folder.as_deref(),
                out: Some(&held.out),
                text: Some(&held.text),
            })
            .collect()
    }

    /// The handbooks among the pages, with the command line's `cli` and
    /// `--nav` when `nav`.
    fn find(&self, cli: &Settings, nav: bool) -> Handbooks {
        Handbooks::find(&self.seen(), Asked { cli, nav })
    }

    /// The handbooks among the pages with no option.
    fn handbooks(&self) -> Handbooks {
        self.find(&Settings::default(), false)
    }
}

/// Each handbook's chapters, by title, with the parts' titles and the
/// depth of each (`>` for each step in).
fn titles(found: &Handbooks) -> Vec<Vec<String>> {
    found
        .books
        .iter()
        .map(|book| {
            let mut out = Vec::new();
            for chapter in &book.chapters {
                if let Some(part) = &chapter.part {
                    out.push(format!("# {part}"));
                }
                let steps = ">".repeat(usize::from(chapter.depth));
                out.push(format!("{steps}{}", chapter.title));
            }
            out
        })
        .collect()
}

/// The warnings each chapter of the first handbook is given, by title.
fn warned(found: &Handbooks) -> Vec<(String, String)> {
    found
        .books
        .iter()
        .flat_map(|book| &book.chapters)
        .filter_map(|chapter| Some((chapter.title.clone(), chapter.warning.clone()?)))
        .collect()
}

/// The start page's links order the chapters.
///
/// The first link places a page, a link to the web, in code, out of the folder
/// or to a file that is not a page passes over; a page no link places comes
/// after, with a warning that names the start page, unless it says `orphan:
/// true`.
///
/// # Panics
///
/// When the order is another, a page is warned about that is linked or
/// says it is meant, or the warning is worded otherwise.
#[test]
fn the_start_pages_links_order_the_chapters() {
    let readme = "Read [C](c.md#top), then [A](./a.md?x), [C again](c.md), [web](https://x.test/b.md), \
`[code](b.md)`, [out](../b.md), [missing](nothing.md), [itself](README.md).\n\n```\n[block](b.md)\n```\n";
    let pages = Pages::folder("book", &[
        ("README.md", readme),
        ("a.md", "A page.\n"),
        ("b.md", "Not linked.\n"),
        ("c.md", "C page.\n"),
        ("d.md", "---\norphan: true\n---\nMeant.\n"),
    ]);
    let found = pages.handbooks();
    assert_eq!(titles(&found), [["README", "c", "a", "b", "d"]]);
    assert_eq!(warned(&found), [(
        String::from("b"),
        String::from(
            "not linked from README.md; it comes after the pages README.md links (orphan: true says that is meant)"
        )
    )]);
    assert!(found.notes().is_empty(), "{:?}", found.notes());
}

/// A link to a subfolder's start page, or to the subfolder, opens a part:
/// its own links follow it a step in, before the start page's next link;
/// a chapter's own links reorder nothing.
///
/// # Panics
///
/// When the part's chapters stand elsewhere or at another depth, or a
/// chapter's link moved a page forward.
#[test]
fn a_subfolders_start_page_opens_a_part() {
    let pages = Pages::folder("book", &[
        (
            "README.md",
            "[One](one.md) [Part](part/README.md) [Two](two.md) [Other](other/)\n",
        ),
        ("one.md", "See [two](two.md).\n"),
        ("two.md", "Two.\n"),
        ("other/index.md", "[O](o.md)\n"),
        ("other/o.md", "O.\n"),
        (
            "part/README.md",
            "[Second](second.md) then [First](first.md)\n",
        ),
        ("part/first.md", "First.\n"),
        ("part/second.md", "Second.\n"),
    ]);
    assert_eq!(titles(&pages.handbooks()), [[
        "README", "one", "README", ">second", ">first", "two", "index", ">o"
    ]]);
}

/// mdBook's `SUMMARY.md` starts the handbook before `README.md`.
///
/// It is no chapter: its nested items stand a step in, a heading after its
/// first link names the part after it, its title does not, and a draft's empty
/// link passes over.
///
/// # Panics
///
/// When the order, a depth or a part's title is another, the summary is a
/// chapter, or the page the draft names is placed.
#[test]
fn a_summary_nests_and_names_its_parts() {
    let summary = "# Summary\n\n[Introduction](README.md)\n\n# Part One\n\n- [A](a.md)\n  - [A1](a1.md)\n- [Draft]()\n\n\
# Part Two\n\n- [B](b.md)\n";
    let pages = Pages::folder("book", &[
        ("README.md", "# Intro\n"),
        ("SUMMARY.md", summary),
        ("a.md", "# A\n"),
        ("a1.md", "# A1\n"),
        ("b.md", "# B\n"),
    ]);
    let found = pages.handbooks();
    assert_eq!(titles(&found), [[
        "Intro",
        "# Part One",
        "A",
        ">A1",
        "# Part Two",
        "B"
    ]]);
    let places = found.places(&pages.seen());
    assert!(
        places.get(1).is_some_and(Option::is_none),
        "the summary is no chapter"
    );
}

/// A folder no link enters stays out of the handbook, said once for its
/// pages; a page of a folder a link enters that no link places is a
/// chapter after the linked ones.
///
/// # Panics
///
/// When a page of a folder no link enters is a chapter, or the notes are
/// other than one for `dev` (two pages) and one for `forge/sub` (one).
#[test]
fn folders_no_link_enters_stay_out() {
    let pages = Pages::folder("docs", &[
        ("README.md", "[A](a.md) [Forge](forge/README.md)\n"),
        ("a.md", "A.\n"),
        ("dev/plan.md", "Internal.\n"),
        ("dev/old/notes.md", "Internal.\n"),
        ("forge/README.md", "Forge.\n"),
        ("forge/guide.md", "Not linked.\n"),
        ("forge/sub/deep.md", "Deeper.\n"),
    ]);
    let found = pages.handbooks();
    assert_eq!(titles(&found), [["README", "a", "README", "guide"]]);
    let dev = Path::new("docs").join("dev");
    let sub = Path::new("docs").join("forge").join("sub");
    let start = Path::new("docs").join("README.md");
    assert_eq!(found.notes(), [
        format!(
            "md2html: 2 pages under {} are not in the handbook (no link from {} enters it)",
            dev.display(),
            start.display()
        ),
        format!(
            "md2html: 1 page under {} is not in the handbook (no link from {} enters it)",
            sub.display(),
            start.display()
        ),
    ]);
}

/// No start page, a start page that links no other page of the run,
/// `handbook: no` on it and `--no-handbook` make no handbook; nor does a
/// folder without a page left to link.
///
/// # Panics
///
/// When any of them makes one.
#[test]
fn without_a_linking_start_page_there_is_no_handbook() {
    let none = Pages::folder("book", &[("a.md", "[b](b.md)\n"), ("b.md", "B.\n")]);
    assert!(none.handbooks().books.is_empty(), "no start page");
    let unlinked = Pages::folder("book", &[
        ("README.md", "[web](https://x.test/) [up](../a.md)\n"),
        ("a.md", "A.\n"),
    ]);
    assert!(
        unlinked.handbooks().books.is_empty(),
        "links nothing of the run"
    );
    let refused = Pages::folder("book", &[
        ("README.md", "---\nhandbook: no\n---\n[A](a.md)\n"),
        ("a.md", "A.\n"),
    ]);
    assert!(refused.handbooks().books.is_empty(), "handbook: no");
    let linked = Pages::folder("book", &[("README.md", "[A](a.md)\n"), ("a.md", "A.\n")]);
    assert_eq!(linked.handbooks().books.len(), 1);
    let cli = Settings {
        handbook: Some(false),
        ..Settings::default()
    };
    assert!(linked.find(&cli, true).books.is_empty(), "--no-handbook");
}

/// `--nav` makes the pages in no handbook one, in the run's order; files
/// named one by one are none without it.
///
/// # Panics
///
/// When the order is not the command line's, or a handbook is made
/// without `--nav`.
#[test]
fn nav_makes_the_run_one_handbook() {
    let pages = Pages::files(&[("z.md", "# Zed\n"), ("a.md", "# A\n"), ("m.md", "M.\n")]);
    assert!(pages.handbooks().books.is_empty());
    let found = pages.find(&Settings::default(), true);
    assert_eq!(titles(&found), [["Zed", "A", "m"]]);
}

/// Each language of the translations is a handbook of its own.
///
/// By its own start page when that links its pages, else in the original's
/// order; a language with one page is none, and a translation is never a
/// chapter of the original.
///
/// # Panics
///
/// When a language's order is another, the language of one page makes a
/// handbook, or a translation stands among the original's chapters.
#[test]
fn each_language_is_a_handbook_of_its_own() {
    let pages = Pages::folder("book", &[
        (
            "README.md",
            "# Book\n\n[B](b.md) [A](a.md) [A in German](a.de.md)\n",
        ),
        ("README.de.md", "# Buch\n\n[A](a.de.md) [B](b.de.md)\n"),
        ("a.md", "# A\n"),
        ("a.de.md", "# A de\n"),
        ("a.fr.md", "# A fr\n"),
        ("b.md", "# B\n"),
        ("b.de.md", "# B de\n"),
        ("b.nl.md", "# B nl\n"),
        ("a.nl.md", "# A nl\n"),
        ("c.nl.md", "# C nl\n"),
    ]);
    assert_eq!(titles(&pages.handbooks()), [
        vec!["Book", "B", "A"],
        vec!["Buch", "A de", "B de"],
        vec!["B nl", "A nl", "C nl"],
    ]);
}

/// The handbook's glossary page is the one named `glossary` at its top,
/// in any case; else the first chapter titled Glossary. Every chapter is
/// lent its terms, their links written from the chapter's folder.
///
/// # Panics
///
/// When another page is the glossary, a chapter is not lent its terms, or
/// a meaning's link from a subfolder's chapter does not lead up.
#[test]
fn the_glossary_page_lends_every_chapter_its_terms() {
    let glossary = "# Words\n\n## Glossary\n\n**Cluster** - see [A](a.md).\n";
    let named = Pages::folder("book", &[
        ("README.md", "[A](a.md) [Sub](sub/README.md)\n"),
        ("Glossary.MD", glossary),
        ("a.md", "# Glossar\n\n**Other** - not the glossary page.\n"),
        ("sub/README.md", "Sub.\n"),
    ]);
    let found = named.handbooks();
    let places = found.places(&named.seen());
    let lent = |at: usize| {
        places
            .get(at)
            .and_then(Option::as_ref)
            .and_then(|place| place.glossary.as_ref())
            .map(|lent| {
                let words: Vec<&str> = lent.terms.iter().map(|term| term.word.as_str()).collect();
                let said: Vec<&str> = lent.terms.iter().map(|term| term.said.as_str()).collect();
                (
                    lent.page.as_str().to_owned(),
                    words.join(","),
                    said.join(","),
                )
            })
    };
    assert_eq!(
        lent(0),
        Some((
            String::from("Glossary.html"),
            String::from("Cluster"),
            String::from("see <a href=\"a.html\">A</a>.")
        ))
    );
    assert_eq!(
        lent(3),
        Some((
            String::from("../Glossary.html"),
            String::from("Cluster"),
            String::from("see <a href=\"../a.html\">A</a>.")
        ))
    );
    let titled = Pages::folder("book", &[
        ("README.md", "[A](a.md) [G](g.md)\n"),
        ("a.md", "A.\n"),
        ("g.md", "# Glossar\n\n**Wort** - ein Wort.\n"),
    ]);
    let found_titled = titled.handbooks();
    let glossary_page = found_titled
        .books
        .first()
        .and_then(|book| book.glossary.as_ref())
        .map(|it| it.page);
    assert_eq!(glossary_page, Some(super::PageNo(2)));
}

/// Each chapter links its neighbours in the handbook's order and shows the
/// contents with itself marked, every link relative to it.
///
/// # Panics
///
/// When a neighbour, an entry's link or the marked chapter is another.
#[test]
fn each_chapter_is_given_its_neighbours_and_the_contents() {
    let pages = Pages::folder("book", &[
        ("README.md", "# Book\n\n[Two](part/two.md) [One](one.md)\n"),
        ("one.md", "# One\n"),
        ("part/two.md", "# Two\n"),
    ]);
    let found = pages.handbooks();
    let places = found.places(&pages.seen());
    let Some(Some(two)) = places.get(2) else {
        panic!("part/two.md is a chapter");
    };
    let prev = two.neighbours.prev.as_ref().map(|link| link.href.as_str());
    let next = two.neighbours.next.as_ref().map(|link| link.href.as_str());
    assert_eq!((prev, next), (Some("../README.html"), Some("../one.html")));
    let entry = |href: &str, title: &str, current: bool| {
        Entry::Chapter(Chapter {
            href: crate::system::relative_url(Path::new(href)).unwrap(),
            title: title.to_owned(),
            depth: 0,
            current,
        })
    };
    assert_eq!(
        two.contents,
        Some(Contents::new(vec![
            entry("../README.html", "Book", false),
            entry("two.html", "Two", true),
            entry("../one.html", "One", false),
        ]))
    );
}
