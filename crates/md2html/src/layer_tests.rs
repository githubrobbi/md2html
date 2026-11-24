// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! md2html's modules keep to the layers `docs/ARCHITECTURE.md` names.
//!
//! The note's table is the one statement of the layers. This test reads
//! it, then every production source file of md2html, found from
//! `main.rs` the way the compiler finds them (`#[cfg(test)]` items, and
//! the files only they declare, left out). Each use of another top-level
//! module is collected: a `crate::` path, or a `super::` that climbs to
//! the crate's root. A use of a higher layer, a circle of uses, a module
//! the table does not place and a path that names no module all fail it.
//! The same reading holds what only `system` may name: the standard
//! library's standard output, and a `cfg` on a system.

use alloc::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::scan::Scan;

/// Where the layers are written, from the crate's folder.
const NOTE: &str = "../../docs/ARCHITECTURE.md";

/// One row of the note's table.
#[derive(Debug, PartialEq, Eq)]
struct Layer {
    /// Its number, 1 for the lowest.
    number: usize,
    /// Its name.
    name: String,
    /// The top-level modules it holds.
    modules: Vec<String>,
}

/// A production source file.
struct Source {
    /// Its module's path from the crate's root; empty for `main.rs`.
    path: Vec<String>,
    /// Where it is, for messages.
    file: String,
    /// Its code, comments, literals and test items blanked.
    code: String,
}

/// One use of a top-level module, in another one.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Use {
    /// The top-level module the use is written in.
    from: String,
    /// The name the path gives after `crate::`: a top-level module, or not.
    to: String,
    /// The file and line it is written at.
    at: String,
}

/// Whether `ch` can be part of an identifier.
fn is_ident(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// The layers the note's "Layers" table lists, in its order.
fn layers(note: &str) -> Vec<Layer> {
    note.lines()
        .skip_while(|line| *line != "## Layers")
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter_map(layer_row)
        .collect()
}

/// A row of the table, `| 1. Name | `a`, `b` | ... |`; None for its head
/// and any other line.
fn layer_row(line: &str) -> Option<Layer> {
    let mut cells = line.strip_prefix('|')?.split('|').map(str::trim);
    let (number, name) = cells.next()?.split_once(". ")?;
    let modules = cells
        .next()?
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    Some(Layer {
        number: number.parse().ok()?,
        name: name.to_owned(),
        modules,
    })
}

/// Characters as blanks: each a space, a line break kept, so the lines
/// keep their numbers.
fn hide(text: &str, out: &mut String) {
    out.extend(text.chars().map(|ch| if ch == '\n' { '\n' } else { ' ' }));
}

/// Source with its comments and literals blanked, so a path in a string
/// or a comment is not a use.
fn blank(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut scan = Scan::new(source);
    while !scan.is_done() {
        let start = scan.mark();
        let in_word = scan.prev().is_some_and(is_ident);
        let literal = if scan.eat_str("//") {
            scan.take_while(|ch| ch != '\n');
            true
        } else if scan.eat_str("/*") {
            block_comment(&mut scan);
            true
        } else {
            !in_word && (raw_string(&mut scan) || string(&mut scan) || char_literal(&mut scan))
        };
        if literal {
            hide(scan.since(start), &mut out);
        } else {
            out.extend(scan.bump());
        }
    }
    out
}

/// Reads the rest of a block comment, the `/*` read; they nest.
fn block_comment(scan: &mut Scan<'_>) {
    let mut depth = 1_usize;
    while depth > 0 {
        if scan.eat_str("/*") {
            depth = depth.saturating_add(1);
        } else if scan.eat_str("*/") {
            depth = depth.saturating_sub(1);
        } else if scan.bump().is_none() {
            return;
        }
    }
}

/// Reads a raw string, `r"..."`, `r#"..."#` or `br"..."`, if one is next.
fn raw_string(scan: &mut Scan<'_>) -> bool {
    let mut ahead = scan.clone();
    ahead.eat('b');
    if !ahead.eat('r') {
        return false;
    }
    let hashes = ahead.take_while(|ch| ch == '#');
    if !ahead.eat('"') {
        return false;
    }
    let end = format!("\"{hashes}");
    if ahead.take_until(&end).is_none() {
        ahead.take_rest();
    }
    ahead.eat_str(&end);
    *scan = ahead;
    true
}

/// Reads a string, `"..."` or `b"..."`, if one is next.
fn string(scan: &mut Scan<'_>) -> bool {
    let mut ahead = scan.clone();
    ahead.eat('b');
    if !ahead.eat('"') {
        return false;
    }
    while let Some(ch) = ahead.bump() {
        if ch == '"' {
            break;
        }
        if ch == '\\' {
            ahead.bump();
        }
    }
    *scan = ahead;
    true
}

/// Reads a character, `'x'`, `'\n'`, `'\u{e9}'` or `b'x'`, if one is
/// next; a lifetime (`'a`, `'static`) is none.
fn char_literal(scan: &mut Scan<'_>) -> bool {
    let mut ahead = scan.clone();
    ahead.eat('b');
    if !ahead.eat('\'') {
        return false;
    }
    if ahead.eat('\\') {
        ahead.bump();
        ahead.take_while(|ch| ch != '\'' && ch != '\n');
    } else {
        ahead.bump();
    }
    if !ahead.eat('\'') {
        return false;
    }
    *scan = ahead;
    true
}

/// Code with each `#[cfg(test)]` item blanked: test modules, and imports
/// and methods only the tests use.
fn drop_tests(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut scan = Scan::new(code);
    while !scan.is_done() {
        let start = scan.mark();
        if scan.eat_str("#[cfg(test)]") {
            skip_item(&mut scan);
            hide(scan.since(start), &mut out);
        } else {
            out.extend(scan.bump());
        }
    }
    out
}

/// Reads one item: up to its `;`, or through its braces.
fn skip_item(scan: &mut Scan<'_>) {
    let mut depth = 0_usize;
    while let Some(ch) = scan.bump() {
        match ch {
            '(' | '[' => depth = depth.saturating_add(1),
            ')' | ']' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return,
            '{' if depth == 0 => {
                braces(scan);
                return;
            }
            _ => {}
        }
    }
}

/// Reads to the `}` that closes a `{` just read.
fn braces(scan: &mut Scan<'_>) {
    let mut depth = 1_usize;
    while depth > 0 {
        match scan.bump() {
            Some('{') => depth = depth.saturating_add(1),
            Some('}') => depth = depth.saturating_sub(1),
            Some(_) => {}
            None => return,
        }
    }
}

/// The modules `code` declares from files: `mod name;`.
fn declared(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut scan = Scan::new(code);
    while !scan.is_done() {
        let in_word = scan.prev().is_some_and(is_ident);
        if !in_word && scan.eat_str("mod ") {
            scan.take_while(char::is_whitespace);
            let name = scan.take_while(is_ident);
            scan.take_while(char::is_whitespace);
            if !name.is_empty() && scan.eat(';') {
                out.push(name.to_owned());
            }
        } else {
            scan.bump();
        }
    }
    out
}

/// Every production source file under `src`, found from `main.rs` as the
/// compiler finds them.
///
/// # Panics
///
/// Panics when a file a module declaration names cannot be read.
fn sources(src: &Path) -> Vec<Source> {
    let mut out = Vec::new();
    let mut todo: Vec<(Vec<String>, PathBuf)> = vec![(Vec::new(), src.join("main.rs"))];
    while let Some((path, file)) = todo.pop() {
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let code = drop_tests(&blank(&text));
        let dir = if path.is_empty() || file.ends_with("mod.rs") {
            file.parent().unwrap_or(src).to_path_buf()
        } else {
            file.with_extension("")
        };
        for name in declared(&code) {
            let flat = dir.join(format!("{name}.rs"));
            let next = if flat.exists() {
                flat
            } else {
                dir.join(&name).join("mod.rs")
            };
            let mut inner = path.clone();
            inner.push(name);
            todo.push((inner, next));
        }
        let shown = file.strip_prefix(src).unwrap_or(&file).display();
        out.push(Source {
            path,
            file: format!("src/{shown}"),
            code,
        });
    }
    out
}

/// The names after `crate::`: one, or the first of each part of a
/// `{...}` group.
fn names(scan: &mut Scan<'_>) -> Vec<String> {
    if !scan.eat('{') {
        return vec![scan.take_while(is_ident).to_owned()];
    }
    let mut out = Vec::new();
    let mut depth = 1_usize;
    let mut part = true;
    while depth > 0 {
        scan.take_while(char::is_whitespace);
        if part {
            let name = scan.take_while(is_ident);
            if !name.is_empty() {
                out.push(name.to_owned());
            }
            part = false;
        }
        match scan.bump() {
            Some('{') => depth = depth.saturating_add(1),
            Some('}') => depth = depth.saturating_sub(1),
            Some(',') => part = depth == 1,
            Some(_) => {}
            None => return out,
        }
    }
    out
}

/// The uses of other top-level modules in one source file. The crate's
/// root may use every module and is not looked at.
fn uses(source: &Source) -> Vec<Use> {
    let Some(from) = source.path.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut scan = Scan::new(&source.code);
    while !scan.is_done() {
        let at_start = !scan.prev().is_some_and(|ch| is_ident(ch) || ch == ':');
        let mut climbs = 0_usize;
        while at_start && scan.eat_str("super::") {
            climbs = climbs.saturating_add(1);
        }
        let to_root = (climbs > 0 && climbs >= source.path.len())
            || (climbs == 0 && at_start && scan.eat_str("crate::"));
        if !to_root {
            if climbs == 0 {
                scan.bump();
            }
            continue;
        }
        let line = scan.seen().matches('\n').count().saturating_add(1);
        out.extend(
            names(&mut scan)
                .into_iter()
                .filter(|to| to != from)
                .map(|to| Use {
                    from: from.clone(),
                    to,
                    at: format!("{}:{line}", source.file),
                }),
        );
    }
    out
}

/// What breaks the layering: each module the table does not place or
/// places but does not exist, each path that names no module, each use of
/// a higher layer, and a circle of uses.
fn findings(layers: &[Layer], modules: &BTreeSet<String>, all: &[Use]) -> Vec<String> {
    let mut out = Vec::new();
    let mut layer_of: BTreeMap<&str, &Layer> = BTreeMap::new();
    for (place, layer) in layers.iter().enumerate() {
        if layer.number != place.saturating_add(1) {
            out.push(format!(
                "layer {} \"{}\" is numbered out of order",
                layer.number, layer.name
            ));
        }
        for module in &layer.modules {
            if layer_of.insert(module, layer).is_some() {
                out.push(format!("`{module}` stands in two layers"));
            }
            if !modules.contains(module) {
                out.push(format!(
                    "the table places `{module}`, which md2html does not declare"
                ));
            }
        }
    }
    for module in modules {
        if !layer_of.contains_key(module.as_str()) {
            out.push(format!(
                "`{module}` stands in no layer: add it to the table"
            ));
        }
    }
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for one in all {
        match (
            layer_of.get(one.from.as_str()),
            layer_of.get(one.to.as_str()),
        ) {
            (_, None) if !modules.contains(&one.to) => out.push(format!(
                "{}: `crate::{}` names no module; name the module the item lives in",
                one.at, one.to
            )),
            (Some(from), Some(to)) if to.number > from.number => out.push(format!(
                "{}: `{}` ({}. {}) uses `{}` ({}. {}), a higher layer",
                one.at, one.from, from.number, from.name, one.to, to.number, to.name
            )),
            _ => {}
        }
        edges
            .entry(one.from.clone())
            .or_default()
            .insert(one.to.clone());
    }
    if let Some(found) = circle(&edges) {
        let steps: Vec<String> = found
            .windows(2)
            .filter_map(|pair| match pair {
                [from, to] => all
                    .iter()
                    .find(|one| one.from == *from && one.to == *to)
                    .map(|one| format!("`{from}` uses `{to}` at {}", one.at)),
                _ => None,
            })
            .collect();
        out.push(format!("uses close a circle: {}", steps.join(", ")));
    }
    out
}

/// A circle of uses, as the modules on it with the first repeated at the
/// end; None when there is none.
fn circle(edges: &BTreeMap<String, BTreeSet<String>>) -> Option<Vec<String>> {
    let mut done = BTreeSet::new();
    edges
        .keys()
        .find_map(|start| visit(start, edges, &mut Vec::new(), &mut done))
}

/// Follows the uses from `node` depth first; a module met again on the
/// way closes a circle.
fn visit<'graph>(
    node: &'graph str,
    edges: &'graph BTreeMap<String, BTreeSet<String>>,
    path: &mut Vec<&'graph str>,
    done: &mut BTreeSet<&'graph str>,
) -> Option<Vec<String>> {
    if let Some(at) = path.iter().position(|seen| *seen == node) {
        let mut found: Vec<String> = path
            .iter()
            .skip(at)
            .map(|seen| (*seen).to_owned())
            .collect();
        found.push(node.to_owned());
        return Some(found);
    }
    if done.contains(node) {
        return None;
    }
    path.push(node);
    for next in edges.get(node).into_iter().flatten() {
        if let Some(found) = visit(next, edges, path, done) {
            return Some(found);
        }
    }
    path.pop();
    done.insert(node);
    None
}

/// The module declarations of `source` that are not private: `pub`, in
/// any of its forms, before `mod`. Each as `file:line`.
fn open_modules(source: &Source) -> Vec<String> {
    let mut out = Vec::new();
    let mut scan = Scan::new(&source.code);
    while !scan.is_done() {
        let in_word = scan.prev().is_some_and(is_ident);
        if !in_word && scan.eat_str("pub") && !scan.peek().is_some_and(is_ident) {
            let line = scan.seen().matches('\n').count().saturating_add(1);
            scan.take_while(char::is_whitespace);
            if scan.eat('(') {
                scan.take_while(|ch| ch != ')');
                scan.bump();
                scan.take_while(char::is_whitespace);
            }
            if scan.eat_str("mod ") {
                out.push(format!("{}:{line}", source.file));
            }
        } else {
            scan.bump();
        }
    }
    out
}

/// A source at `path` holding `code`, read as the test reads a file.
fn source(path: &[&str], code: &str) -> Source {
    Source {
        path: path.iter().map(|part| (*part).to_owned()).collect(),
        file: String::from("src/x.rs"),
        code: drop_tests(&blank(code)),
    }
}

/// The modules `code` at `path` names, in order.
fn named(path: &[&str], code: &str) -> Vec<String> {
    uses(&source(path, code))
        .into_iter()
        .map(|one| one.to)
        .collect()
}

/// md2html's modules keep to the layers `docs/ARCHITECTURE.md` names.
///
/// # Panics
///
/// Panics when the note or a source file cannot be read, when the note
/// has no layer table, when no use is found at all (the reading would be
/// broken), or with every finding: a module the table does not place or
/// one it places that does not exist, a path that names no module, a use
/// of a higher layer, a circle of uses.
#[test]
fn modules_keep_to_their_layers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let note = std::fs::read_to_string(root.join(NOTE)).expect("docs/ARCHITECTURE.md");
    let layers = layers(&note);
    assert!(
        !layers.is_empty(),
        "docs/ARCHITECTURE.md has no layer table"
    );
    let sources = sources(&root.join("src"));
    let modules: BTreeSet<String> = sources
        .iter()
        .filter_map(|one| match one.path.as_slice() {
            [module] => Some(module.clone()),
            _ => None,
        })
        .collect();
    let all: Vec<Use> = sources.iter().flat_map(uses).collect();
    assert!(all.len() > modules.len(), "only {} uses found", all.len());
    let found = findings(&layers, &modules, &all);
    assert!(
        found.is_empty(),
        "md2html's modules break docs/ARCHITECTURE.md:\n{}",
        found.join("\n")
    );
}

/// Each module's root is what it offers the rest of md2html: its
/// submodules are private, and what other modules use is declared or
/// re-exported in the root.
///
/// # Panics
///
/// Panics when a source file cannot be read, when too few are found for
/// the reading to be right, or with every module declared `pub`,
/// `pub(crate)`, `pub(super)` or `pub(in ...)`.
#[test]
fn every_module_root_is_its_storefront() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources = sources(&root.join("src"));
    assert!(
        sources.len() > 20,
        "only {} source files found",
        sources.len()
    );
    let open: Vec<String> = sources.iter().flat_map(open_modules).collect();
    assert!(
        open.is_empty(),
        "modules declared public; keep them private and offer their items from the parent's root: {open:?}"
    );
}

/// What production code names only in `system`.
///
/// The standard library's standard output, which md2html writes through
/// `system::StandardOutput` alone, and a `cfg` on anything but tests
/// (`docs/policies/platform_policy.md`).
const SYSTEM_ONLY: [&str; 6] = [
    "stdout()",
    "io::stdout",
    "cfg(",
    "cfg!(",
    "target_os",
    "target_family",
];

/// Where a production file names what only `system` may, with its line.
fn outside_system(source: &Source) -> Vec<String> {
    if source.path.first().is_some_and(|module| module == "system") {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (at, line) in source.code.lines().enumerate() {
        for name in SYSTEM_ONLY.iter().filter(|name| line.contains(*name)) {
            found.push(format!("{}:{}: {name}", source.file, at.saturating_add(1)));
        }
    }
    found
}

/// Standard output is written through `system` alone, and only `system`
/// has a `cfg` on a system.
///
/// # Panics
///
/// Panics when a source file cannot be read, when too few are found for
/// the reading to be right, or with every place outside `system` that
/// names `std::io::stdout` or a `cfg` other than `cfg(test)`.
#[test]
fn only_system_names_standard_output_and_the_systems() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources = sources(&root.join("src"));
    assert!(
        sources.len() > 20,
        "only {} source files found",
        sources.len()
    );
    let found: Vec<String> = sources.iter().flat_map(outside_system).collect();
    assert!(
        found.is_empty(),
        "write standard output through system::StandardOutput and keep a cfg on a system in system:\n{}",
        found.join("\n")
    );
}

/// A name only `system` may use is found outside it, with its line, and
/// not inside it, in a test item or in a comment.
///
/// # Panics
///
/// Panics when `std::io::stdout()`, `io::stdout` as a function or a
/// `cfg(unix)` outside `system` is missed, or when one in `system`, in a
/// `#[cfg(test)]` item or in a comment, or a child's `.stdout(...)`, is
/// taken for one.
#[test]
fn names_only_system_may_use_are_found() {
    let code = "fn a() { std::io::stdout(); }\n// std::io::stdout()\n#[cfg(unix)]\nfn b() { f(io::stdout) }\n\
        #[cfg(test)]\nmod tests { fn c() { std::io::stdout(); } }\nfn d() { x.stdout(Stdio::null()); }\n";
    assert_eq!(outside_system(&source(&["report"], code)), vec![
        String::from("src/x.rs:1: stdout()"),
        String::from("src/x.rs:1: io::stdout"),
        String::from("src/x.rs:3: cfg("),
        String::from("src/x.rs:4: io::stdout"),
    ]);
    assert_eq!(
        outside_system(&source(&["system", "output"], code)),
        Vec::<String>::new()
    );
}

/// A module declared public in any form is found, with its line; a
/// private one, one in a comment, and other public items are not.
///
/// # Panics
///
/// Panics when `pub(crate) mod` or `pub(in crate::x) mod` is missed or put
/// on another line, or when `mod`, a comment, `pub fn modulus` or
/// `pub struct Module` is taken for one.
#[test]
fn open_modules_are_found() {
    let code = "mod a;\npub(crate) mod b;\n// pub mod c;\npub(in crate::x) mod d;\n\
        pub fn modulus() {}\npub struct Module;\nrepub mod e;\n";
    assert_eq!(
        open_modules(&source(&["x"], code)),
        vec![String::from("src/x.rs:2"), String::from("src/x.rs:4")],
        "the open modules of {code:?}"
    );
}

/// A path in a comment or a literal is no use, and the lines keep their
/// numbers.
///
/// # Panics
///
/// Panics when a path inside a string, a raw string, a line or nested
/// block comment is taken for a use, when a character or a lifetime
/// derails the reading, or when the one real use is missed or put on
/// another line.
#[test]
fn comments_and_literals_hold_no_uses() {
    let code = "let a = \"crate::one\"; // crate::two\n\
        /* crate::three /* crate::four */ crate::five */ let b = '{'; let c = '\\'';\n\
        fn f<'x>(s: &'x str) {} let d = r#\"crate::six\"#; let e = b\"crate::seven\";\n\
        crate::real::thing();\n";
    let found = uses(&source(&["here"], code));
    assert_eq!(found, vec![Use {
        from: String::from("here"),
        to: String::from("real"),
        at: String::from("src/x.rs:4"),
    }]);
}

/// Test items are left out; groups and climbs to the root name their
/// modules.
///
/// # Panics
///
/// Panics when a use inside a `#[cfg(test)]` module or import is counted,
/// when a module of a `{...}` group is missed or an inner name counted,
/// when `super::super::` from two levels down does not reach the root, or
/// when a `super::` that stays inside the module is counted.
#[test]
fn groups_climbs_and_test_items() {
    let code = "#[cfg(test)]\nmod tests {\n    use crate::cli;\n}\n#[cfg(test)]\nuse crate::serve;\n\
        use crate::{text::{esc, put}, model::Node};\nuse super::super::scan::Scan;\nuse super::Target;\n";
    assert_eq!(named(&["inline", "link"], code), ["text", "model", "scan"]);
    assert_eq!(
        named(&["inline"], "use super::block::x;\nuse crate::inline::y;\n"),
        ["block"]
    );
    assert_eq!(
        declared("mod a;\n#[cfg(test)]\nmod b;\nmod c {}\npub(crate) mod d;"),
        ["a", "b", "d"]
    );
    assert_eq!(declared(&drop_tests("mod a;\n#[cfg(test)]\nmod b;\n")), [
        "a"
    ]);
}

/// The table is read row by row, and every way of breaking it is found.
///
/// # Panics
///
/// Panics when the table's rows are not read as their layers, when a
/// clean set of uses gives a finding, or when an unplaced module, a module
/// that does not exist, a path that names no module, a use of a higher
/// layer or a circle inside a layer is not found.
#[test]
fn every_break_is_found() {
    let note = "# T\n\n## Layers\n\n| Layer | Modules | What |\n| --- | --- | --- |\n\
        | 1. Low | `low` | x |\n| 2. High | `high`, `side` | y |\n\n## Next\n\n| 3. No | `no` | z |\n";
    let read = layers(note);
    assert_eq!(read, vec![
        Layer {
            number: 1,
            name: String::from("Low"),
            modules: vec![String::from("low")],
        },
        Layer {
            number: 2,
            name: String::from("High"),
            modules: vec![String::from("high"), String::from("side")],
        },
    ]);
    let one = |from: &str, to: &str| Use {
        from: from.to_owned(),
        to: to.to_owned(),
        at: format!("src/{from}.rs:1"),
    };
    let modules: BTreeSet<String> = ["low", "high", "side"].map(String::from).into();
    let clean = [one("high", "low"), one("high", "side"), one("side", "low")];
    assert_eq!(findings(&read, &modules, &clean), Vec::<String>::new());
    let more: BTreeSet<String> = ["low", "high", "side", "lost"].map(String::from).into();
    let broken = [
        one("low", "high"),
        one("high", "side"),
        one("side", "high"),
        one("side", "esc"),
    ];
    let found = findings(&read, &more, &broken).join("\n");
    for expected in [
        "`lost` stands in no layer",
        "src/side.rs:1: `crate::esc` names no module",
        "src/low.rs:1: `low` (1. Low) uses `high` (2. High), a higher layer",
        "uses close a circle: `high` uses `side` at src/high.rs:1, `side` uses `high` at src/side.rs:1",
    ] {
        assert!(found.contains(expected), "{expected} not in:\n{found}");
    }
    let missing = ["low", "high"].map(String::from).into();
    assert!(
        findings(&read, &missing, &[]).contains(&String::from(
            "the table places `side`, which md2html does not declare"
        )),
        "a module that does not exist is not found"
    );
}
