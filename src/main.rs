// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! `md2html`: Markdown in, one self-contained HTML page out.
//!
//! Forked from iv's `iv-handbook` (same owner, MIT OR Apache-2.0), which
//! itself came from docenta's `handbook-html`. Standard library only: no
//! Markdown crate, no network, nothing to vendor.
//!
//! The Markdown is read the way `CommonMark` reads it, with GitHub's tables,
//! task lists, strikethrough, autolinks, footnotes and alerts, and `$maths$`.
//! The page around it has an outline, search, themes, text size, figures
//! that enlarge, and a glossary whose words explain themselves.

extern crate alloc;

mod block;
mod config;
mod glossary;
mod highlight;
mod inline;
mod math;
mod mermaid;
mod page;
mod render;
mod text;

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use config::{Settings, front_matter};
use glossary::Term;
use inline::HtmlMode;
use render::Warning;
pub(crate) use text::esc;

/// The help text.
const HELP: &str = "\
md2html: Markdown in, one self-contained HTML page out.

usage: md2html [options] <input>...
       md2html <in.md> <out.html>

  <input>              a Markdown file, a folder (every .md in it), or - for stdin
  -o, --out <path>     the page (one input), a folder (several), or - for stdout

page:
      --title <text>   the title, instead of the first heading
      --lang <code>    the language: en (default) or de for the page's own words
      --brand <text>   the name in the bar, instead of md2html
      --footer <text>  the fine print at the foot (Markdown)
      --css <file>     add a stylesheet          --head <file>  add to the <head>
      --set key=value  any front matter setting (repeatable)
      --large          larger type, for a page read at half a screen
      --embed          put the images inside the page as well
      --no-glossary    leave the shared glossary out
      --no-toc         no outline
      --html           trust raw HTML in the Markdown (it is shown as text otherwise)
      --csp            a Content-Security-Policy: only the page's own code runs
      --index          let search engines index the page (noindex by default)
      --no-math        read $ as a dollar sign, not maths
      --mermaid <how>  draw diagrams: native (default), js, or code
      --mermaid-js <f> a local mermaid.js for the diagram kinds drawn in the browser
      --nav            link the pages of one run with previous and next
      --fragment       only the HTML of the Markdown, no page around it

run:
      --check          write nothing; report what to fix; exit 1 if anything
      --watch          render again whenever an input changes
  -q, --quiet          no summary, no warnings
  -h, --help           this text
  -V, --version        the version

Front matter at the top of the Markdown takes the same settings:
  ---
  title: The plans
  lang: de
  brand: Maplewood ADU
  ---
";

/// What the command line asked for.
#[derive(Default)]
struct Cli {
    /// The inputs as given.
    inputs: Vec<PathBuf>,
    /// `-o`.
    out: Option<PathBuf>,
    /// Settings over the front matter.
    settings: Settings,
    /// `--check`.
    check: bool,
    /// `--watch`.
    watch: bool,
    /// `-q`.
    quiet: bool,
    /// `--fragment`.
    fragment: bool,
    /// `--nav`.
    nav: bool,
}

fn main() {
    std::process::exit(run());
}

/// The exit status of one run.
fn run() -> i32 {
    let cli = match parse_args(std::env::args().skip(1).collect()) {
        Ok(Some(cli)) => cli,
        Ok(None) => return 0,
        Err(message) => {
            eprintln!("md2html: {message}\nusage: md2html [options] <input>... (md2html --help)");
            return 64;
        }
    };
    let jobs = match plan(&cli) {
        Ok(jobs) => jobs,
        Err((code, message)) => {
            eprintln!("md2html: {message}");
            return code;
        }
    };
    let mut status = render_all(&cli, &jobs);
    if cli.watch {
        status = watch(&cli, &jobs);
    }
    status
}

/// The command line, or None when it only asked for help or the version.
fn parse_args(args: Vec<String>) -> Result<Option<Cli>, String> {
    let mut cli = Cli::default();
    let mut args = args.into_iter();
    let mut rest_positional = false;
    while let Some(arg) = args.next() {
        if rest_positional || arg == "-" || !arg.starts_with('-') {
            cli.inputs.push(PathBuf::from(arg));
            continue;
        }
        let (name, inline_value) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_owned(), Some(v.to_owned())),
            _ => (arg.clone(), None),
        };
        let mut value = |what: &str| -> Result<String, String> {
            inline_value
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{name} needs {what}"))
        };
        let s = &mut cli.settings;
        match name.as_str() {
            "--" => rest_positional = true,
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("md2html {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-o" | "--out" | "--output" => cli.out = Some(PathBuf::from(value("a path")?)),
            "--large" => s.large = Some(true),
            "--no-glossary" => s.glossary = Some(false),
            "--embed" | "--self-contained" => s.embed = Some(true),
            "--html" | "--unsafe-html" => s.html = Some(true),
            "--csp" => s.csp = Some(true),
            "--index" => s.index = Some(true),
            "--no-toc" => s.toc = Some(false),
            "--no-math" => s.math = Some(false),
            "--mermaid" => s.set("mermaid", &value("native, js or code")?)?,
            "--mermaid-js" => s.mermaid_js = Some(PathBuf::from(value("a file")?)),
            "--title" => s.title = Some(value("a title")?),
            "--lang" => s.lang = Some(value("a language code")?),
            "--brand" => s.brand = Some(value("a name")?),
            "--footer" => s.footer = Some(value("a text")?),
            "--css" => s.css.push(PathBuf::from(value("a file")?)),
            "--head" => s.head.push(PathBuf::from(value("a file")?)),
            "--set" => {
                let pair = value("key=value")?;
                let (key, val) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("--set {pair}: write it as key=value"))?;
                s.set(key, val)?;
            }
            "--check" => cli.check = true,
            "--watch" => cli.watch = true,
            "-q" | "--quiet" => cli.quiet = true,
            "--fragment" => cli.fragment = true,
            "--nav" => cli.nav = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    // The old form: `md2html in.md out.html`.
    if cli.out.is_none() && cli.inputs.len() == 2 {
        let second = &cli.inputs[1];
        if !is_markdown(second) && !second.is_dir() && second.as_os_str() != "-" {
            cli.out = cli.inputs.pop();
        } else if is_markdown(second) && !second.exists() {
            return Err(format!(
                "{} does not exist; the output would be a Markdown file (md2html a.md b.md once overwrote b.md)",
                second.display()
            ));
        }
    }
    if cli.inputs.is_empty() {
        return Err(String::from("no input"));
    }
    Ok(Some(cli))
}

/// Whether a path names a Markdown file.
fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["md", "markdown", "mdown", "mkd"]
                .iter()
                .any(|md| ext.eq_ignore_ascii_case(md))
        })
}

/// One page to make.
#[derive(Clone)]
struct Job {
    /// The Markdown, or None for stdin.
    source: Option<PathBuf>,
    /// The page, or None for stdout.
    out: Option<PathBuf>,
}

/// The pages a run makes.
fn plan(cli: &Cli) -> Result<Vec<Job>, (i32, String)> {
    let mut sources: Vec<Option<PathBuf>> = Vec::new();
    let mut roots: Vec<Option<PathBuf>> = Vec::new();
    for input in &cli.inputs {
        if input.as_os_str() == "-" {
            sources.push(None);
            roots.push(None);
        } else if input.is_dir() {
            let mut found = Vec::new();
            walk(input, &mut found);
            if found.is_empty() {
                return Err((66, format!("{} holds no Markdown file", input.display())));
            }
            for file in found {
                sources.push(Some(file));
                roots.push(Some(input.clone()));
            }
        } else if input.is_file() {
            sources.push(Some(input.clone()));
            roots.push(input.parent().map(Path::to_path_buf));
        } else {
            return Err((66, format!("{}: no such file", input.display())));
        }
    }
    let several = sources.len() > 1;
    let mut jobs = Vec::new();
    for (source, root) in sources.into_iter().zip(roots) {
        let out = match (&cli.out, &source) {
            (Some(out), _) if out.as_os_str() == "-" => None,
            (Some(out), Some(src)) if several || out.is_dir() => {
                let rel = root
                    .as_ref()
                    .and_then(|r| src.strip_prefix(r).ok())
                    .map_or_else(
                        || PathBuf::from(src.file_name().unwrap_or_default()),
                        Path::to_path_buf,
                    );
                Some(out.join(rel).with_extension("html"))
            }
            (Some(out), _) => Some(out.clone()),
            (None, Some(src)) => Some(src.with_extension("html")),
            (None, None) => None,
        };
        if let (Some(src), Some(out)) = (&source, &out) {
            if let Some(reason) = unsafe_output(src, out) {
                return Err((64, format!("refusing to write {}: {reason}", out.display())));
            }
        }
        jobs.push(Job { source, out });
    }
    if jobs.iter().filter(|job| job.out.is_none()).count() > 1 && !cli.check {
        return Err((
            64,
            String::from("several pages cannot all go to stdout; give -o a folder"),
        ));
    }
    Ok(jobs)
}

/// Every Markdown file under `dir`, folders sorted, an index or README first.
fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort_by_key(|path| {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let first = name.starts_with("index.") || name.starts_with("readme.");
        (path.is_dir(), !first, name)
    });
    for path in entries {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk(&path, found);
        } else if is_markdown(&path) {
            found.push(path);
        }
    }
}

/// Why `out` must not be written: it is the source itself, or Markdown.
fn unsafe_output(source: &Path, out: &Path) -> Option<&'static str> {
    let same = match (std::fs::canonicalize(source), std::fs::canonicalize(out)) {
        (Ok(a), Ok(b)) => a == b,
        _ => source == out,
    };
    if same {
        return Some("it is the input file");
    }
    is_markdown(out).then_some("the output would be a Markdown file")
}

/// Renders every job; the exit status.
fn render_all(cli: &Cli, jobs: &[Job]) -> i32 {
    let mut texts: Vec<Result<String, String>> = Vec::with_capacity(jobs.len());
    for job in jobs {
        texts.push(if let Some(path) = &job.source {
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
        } else {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map(|_| buf)
                .map_err(|e| format!("stdin: {e}"))
        });
    }
    // The run's pages in order, for --nav.
    let titles: Vec<String> = jobs
        .iter()
        .zip(&texts)
        .map(|(job, text)| {
            text.as_ref()
                .map(|t| quick_title(t, job))
                .unwrap_or_default()
        })
        .collect();
    let mut status = 0;
    let mut problems = 0usize;
    for (at, (job, text)) in jobs.iter().zip(&texts).enumerate() {
        let text = match text {
            Ok(text) => text,
            Err(message) => {
                eprintln!("md2html: {message}");
                status = 66;
                continue;
            }
        };
        let nav = if cli.nav && jobs.len() > 1 {
            let link = |other: usize| -> Option<(String, String)> {
                let target = jobs.get(other)?.out.as_ref()?;
                let here = job.out.as_ref()?;
                Some((
                    relative(here, target),
                    titles.get(other).cloned().unwrap_or_default(),
                ))
            };
            let prev = at.checked_sub(1).and_then(link);
            let next = link(at + 1);
            Some((prev, next))
        } else {
            None
        };
        let page = render_doc(
            text,
            job.source.as_deref(),
            job.out.as_deref(),
            &cli.settings,
            cli.fragment,
            nav,
        );
        let shown = job
            .source
            .as_ref()
            .map_or_else(|| String::from("<stdin>"), |p| p.display().to_string());
        if !cli.quiet || cli.check {
            for warning in &page.warnings {
                if warning.line > 0 {
                    eprintln!("{shown}:{}: {}", warning.line, warning.message);
                } else {
                    eprintln!("{shown}: {}", warning.message);
                }
            }
        }
        problems += page.warnings.len();
        if cli.check {
            continue;
        }
        match &job.out {
            None => {
                let mut stdout = std::io::stdout().lock();
                if stdout.write_all(page.html.as_bytes()).is_err() {
                    status = 73;
                }
            }
            Some(out) => {
                if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(error) = write_atomic(out, page.html.as_bytes()) {
                    eprintln!("md2html: writing {}: {error}", out.display());
                    status = 73;
                    continue;
                }
                if !cli.quiet {
                    let warned = if page.warnings.is_empty() {
                        String::new()
                    } else {
                        format!(", {} to look at", page.warnings.len())
                    };
                    println!(
                        "md2html: {shown} -> {} ({} bytes, {} glossary terms, {} of them shared{warned})",
                        out.display(),
                        page.html.len(),
                        page.terms,
                        page.shared
                    );
                }
            }
        }
    }
    if cli.check && status == 0 {
        if problems > 0 {
            if !cli.quiet {
                eprintln!("md2html: {problems} to look at");
            }
            return 1;
        }
        if !cli.quiet {
            println!("md2html: nothing to fix");
        }
    }
    status
}

/// Writes a file whole or not at all: a reader never sees half a page.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.md2html-tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// The path from page `from` to page `to`, for a link between them.
fn relative(from: &Path, to: &Path) -> String {
    let abs = |p: &Path| {
        std::fs::canonicalize(p.parent().unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| p.parent().unwrap_or(Path::new(".")).to_path_buf())
    };
    let a = abs(from);
    let b = abs(to);
    let common = a
        .components()
        .zip(b.components())
        .take_while(|(x, y)| x == y)
        .count();
    let mut out = PathBuf::new();
    for _ in a.components().skip(common) {
        out.push("..");
    }
    for part in b.components().skip(common) {
        out.push(part);
    }
    out.push(to.file_name().unwrap_or_default());
    out.to_string_lossy().replace('\\', "/")
}

/// A page's title without rendering it: front matter, the first heading,
/// or the file's name.
fn quick_title(text: &str, job: &Job) -> String {
    let (map, body, _) = front_matter(text);
    if let Some(title) = map.get("title") {
        return title.clone();
    }
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map_or_else(
            || stem(job.source.as_deref()),
            |t| t.trim().replace(['`', '*'], ""),
        )
}

/// A file's name without its extension.
fn stem(source: Option<&Path>) -> String {
    source.and_then(|p| p.file_stem()).map_or_else(
        || String::from("stdin"),
        |s| s.to_string_lossy().into_owned(),
    )
}

/// A page's neighbours in a run, as (link, title): the previous, the next.
type Neighbours = (Option<(String, String)>, Option<(String, String)>);

/// A rendered page and what came of it.
pub(crate) struct Rendered {
    /// The HTML.
    pub(crate) html: String,
    /// What the author should know.
    pub(crate) warnings: Vec<Warning>,
    /// Glossary terms on the page.
    pub(crate) terms: usize,
    /// Of them, from the shared glossary.
    pub(crate) shared: usize,
}

/// One Markdown text as a page.
pub(crate) fn render_doc(
    text: &str,
    source: Option<&Path>,
    out: Option<&Path>,
    cli: &Settings,
    fragment: bool,
    nav: Option<Neighbours>,
) -> Rendered {
    let mut warnings = Vec::new();
    let (map, body, first_line) = front_matter(text);
    let mut own = Settings::default();
    for (key, value) in &map {
        if let Err(message) = own.set(key, value) {
            warnings.push(Warning {
                line: 1,
                message: format!("front matter: {message}"),
            });
        }
    }
    let s = own.overlay(cli);
    let math = s.math.unwrap_or(true);
    let html_mode = if s.html == Some(true) {
        HtmlMode::Trusted
    } else {
        HtmlMode::Safe
    };
    let (blocks, defs) = block::parse(body, first_line, math, false);
    if fragment {
        return Rendered {
            html: render::fragment(&blocks, &defs, html_mode, math),
            warnings,
            terms: 0,
            shared: 0,
        };
    }
    let folder = |p: &Path| -> PathBuf {
        p.parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    };
    let out_dir = out.map(folder);
    let src_dir = source.map(folder);
    let base = src_dir.as_deref();
    let mut bases: Vec<&Path> = Vec::new();
    for dir in [out_dir.as_deref(), src_dir.as_deref()]
        .into_iter()
        .flatten()
    {
        if !bases.contains(&dir) {
            bases.push(dir);
        }
    }
    let own_terms = glossary::terms(&blocks, &defs);
    let shared = if s.glossary == Some(false) {
        None
    } else {
        source.and_then(shared_glossary)
    };
    let glossary_url = shared.as_ref().map(|(path, _)| glossary_url(path));
    let terms = join(
        own_terms,
        shared.map(|(_, terms)| terms).unwrap_or_default(),
    );
    let lang = s.lang.clone().unwrap_or_else(|| String::from("en"));
    let ui = config::ui(&lang);
    let name = source.and_then(|p| p.file_name()).map_or_else(
        || String::from("stdin"),
        |n| n.to_string_lossy().into_owned(),
    );
    let title = s
        .title
        .clone()
        .unwrap_or_else(|| first_title(&blocks, &defs).unwrap_or_else(|| stem(source)));
    let date = match &s.date {
        Some(date) => text::iso_date(date).map_or_else(|| date.clone(), |d| ui.date(d)),
        None => ui.date(text::civil(text::now_secs())),
    };
    let eyebrow = match &s.eyebrow {
        Some(eyebrow) => esc(eyebrow),
        None => match &s.version {
            Some(version) => format!(
                "{} &middot; {} &middot; {}",
                esc(&name),
                esc(version),
                esc(&date)
            ),
            None => format!("{} &middot; {}", esc(&name), esc(&date)),
        },
    };
    let inputs = render::Inputs {
        defs: &defs,
        terms: &terms,
        ui,
        html: html_mode,
        math,
        title: title.clone(),
        eyebrow,
        bases,
        embed: s.embed.unwrap_or(false),
        mermaid: match s.mermaid.as_deref() {
            Some("code") => render::Diagrams::Code,
            Some("js") if s.mermaid_js.is_some() => render::Diagrams::Js,
            _ => render::Diagrams::Native,
        },
        mermaid_js: s.mermaid_js.is_some(),
    };
    let article = render::article(&blocks, &inputs);
    warnings.extend(article.warnings.iter().cloned());
    let description = s
        .description
        .clone()
        .unwrap_or_else(|| shorten(&article.lead, 200));
    let footer = match &s.footer {
        Some(footer) => {
            let env = inline::Env {
                defs: &defs,
                html: html_mode,
                math: false,
            };
            render::bare_inline(&inline::parse(footer, &env))
        }
        None => ui.footer.replace("{source}", &esc(&name)),
    };
    let mut css = String::new();
    for file in &s.css {
        match read_beside(base, file) {
            Ok(text) => css.push_str(&text),
            Err(message) => warnings.push(Warning { line: 0, message }),
        }
    }
    let mut head = String::new();
    for file in &s.head {
        match read_beside(base, file) {
            Ok(text) => head.push_str(&text),
            Err(message) => warnings.push(Warning { line: 0, message }),
        }
    }
    // mermaid.js goes into a page only when one of its diagrams needs it.
    let mermaid_js = match (&s.mermaid_js, article.mermaid_js) {
        (Some(file), true) => match read_beside(base, file) {
            Ok(js) => Some(js.replace("</script", "<\\/script")),
            Err(message) => {
                warnings.push(Warning {
                    line: 0,
                    message: format!("mermaid.js: {message}"),
                });
                None
            }
        },
        _ => None,
    };
    if s.mermaid.as_deref() == Some("js") && s.mermaid_js.is_none() {
        warnings.push(Warning {
            line: 0,
            message: String::from(
                "mermaid: js needs --mermaid-js FILE; the diagrams are drawn natively",
            ),
        });
    }
    let nav_html = nav
        .map(|(prev, next)| {
            page::pages_nav(
                prev.as_ref().map(|(h, t)| (h.as_str(), t.as_str())),
                next.as_ref().map(|(h, t)| (h.as_str(), t.as_str())),
                ui,
            )
        })
        .unwrap_or_default();
    let meta = page::Meta {
        title,
        source: name.clone(),
        large: s.large.unwrap_or(false),
        glossary_url,
        ui,
        lang,
        brand: s
            .brand
            .as_deref()
            .map_or_else(|| String::from("md2html"), esc),
        brand_link: s.brand_link.clone().unwrap_or_else(|| String::from("#top")),
        kicker: s.kicker.as_deref().map_or_else(|| esc(&name), esc),
        footer,
        description,
        author: s.author.clone(),
        index: s.index.unwrap_or(false),
        toc: s.toc.unwrap_or(true),
        csp: s.csp.unwrap_or(false),
        css: css.replace("</style", "<\\/style"),
        head,
        nav: nav_html,
        mermaid_js,
    };
    let html = page::page(&article, &terms, &meta);
    for missing in broken_anchors(&html) {
        warnings.push(Warning {
            line: 0,
            message: format!("a link points at #{missing}, which is nowhere on the page"),
        });
    }
    let shared_count = terms.iter().filter(|t| t.shared).count();
    Rendered {
        html,
        warnings,
        terms: terms.len(),
        shared: shared_count,
    }
}

/// The document's own title: its first heading of level one.
fn first_title(blocks: &[block::Block], defs: &block::Defs) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        block::Block::Heading { level: 1, text, .. } => {
            let env = inline::Env {
                defs,
                html: HtmlMode::Safe,
                math: false,
            };
            Some(inline::plain(&inline::parse(text, &env)))
        }
        _ => None,
    })
}

/// Text cut at a word to about `max` characters.
fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end_matches([',', ';', ':', '.']))
}

/// A file named relative to the Markdown's folder.
fn read_beside(base: Option<&Path>, file: &Path) -> Result<String, String> {
    let path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        base.unwrap_or(Path::new(".")).join(file)
    };
    std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The in-page links of a page that point at no anchor on it.
fn broken_anchors(html: &str) -> Vec<String> {
    let mut ids = alloc::collections::BTreeSet::new();
    let mut rest = html;
    while let Some(at) = rest.find(" id=\"") {
        rest = &rest[at + 5..];
        if let Some(end) = rest.find('"') {
            ids.insert(rest[..end].to_owned());
        }
    }
    let mut missing = Vec::new();
    let body = html.find("<article").map_or(html, |at| &html[at..]);
    let mut rest = body;
    while let Some(at) = rest.find("href=\"#") {
        rest = &rest[at + 7..];
        let Some(end) = rest.find('"') else { break };
        let id = inline::decode_entities(&rest[..end]);
        if !id.is_empty()
            && !ids.contains(&id)
            && !ids.contains(&text::url_attr(&id))
            && !missing.contains(&id)
        {
            missing.push(id);
        }
    }
    missing
}

/// The shared glossary: `MD2HTML_GLOSSARY`, else `~/.config/md2html/glossary.md`.
fn shared_glossary(source: &Path) -> Option<(PathBuf, Vec<Term>)> {
    let path = std::env::var_os("MD2HTML_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/md2html/glossary.md"))
        })?;
    let text = std::fs::read_to_string(&path).ok()?;
    let same = match (std::fs::canonicalize(&path), std::fs::canonicalize(source)) {
        (Ok(shared), Ok(own)) => shared == own,
        _ => false,
    };
    if same {
        return None;
    }
    let (_, body, first) = front_matter(&text);
    let (blocks, defs) = block::parse(body, first, false, false);
    let terms = glossary::terms(&blocks, &defs)
        .into_iter()
        .map(|term| Term {
            shared: true,
            ..term
        })
        .collect();
    Some((path, terms))
}

/// Where a page sends the reader for the full shared glossary.
fn glossary_url(path: &Path) -> String {
    if let Some(url) = std::env::var_os("MD2HTML_GLOSSARY_URL") {
        return url.to_string_lossy().into_owned();
    }
    let twin = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .with_extension("html");
    format!("file://{}", twin.display())
}

/// The page's own terms, then the shared ones it does not define itself,
/// longest first as the matcher wants them.
fn join(own: Vec<Term>, shared: Vec<Term>) -> Vec<Term> {
    let mut terms = own;
    for term in shared {
        if !terms
            .iter()
            .any(|have| have.word.eq_ignore_ascii_case(&term.word))
        {
            terms.push(term);
        }
    }
    terms.sort_by_key(|term| core::cmp::Reverse(term.word.len()));
    terms
}

/// Renders again whenever an input, a stylesheet or the shared glossary
/// changes. Runs until interrupted.
fn watch(cli: &Cli, jobs: &[Job]) -> i32 {
    let mut watched: Vec<PathBuf> = jobs.iter().filter_map(|j| j.source.clone()).collect();
    for file in cli.settings.css.iter().chain(&cli.settings.head) {
        watched.push(file.clone());
    }
    if let Some(home) = std::env::var_os("MD2HTML_GLOSSARY")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/md2html/glossary.md"))
        })
    {
        watched.push(home);
    }
    if watched.is_empty() {
        eprintln!("md2html: --watch needs files, not stdin");
        return 64;
    }
    let stamp = |paths: &[PathBuf]| -> Vec<Option<std::time::SystemTime>> {
        paths
            .iter()
            .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
            .collect()
    };
    let mut last = stamp(&watched);
    if !cli.quiet {
        eprintln!(
            "md2html: watching {} file(s); Ctrl-C to stop",
            watched.len()
        );
    }
    loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let now = stamp(&watched);
        if now != last {
            last = now;
            render_all(cli, jobs);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod spec_tests;
