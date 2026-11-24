// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The command line, read into what it asks for.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::files::is_markdown;
use crate::config::{Settings, Ttl};
use crate::document::Shape;

/// The help text.
pub(super) const HELP: &str = "\
md2html: Markdown in, one self-contained HTML page out.

usage: md2html [options] <input>...
       md2html <in.md> <out.html>

  <input>              a Markdown file, a folder (every Markdown file under it),
                       or - for stdin
  -o, --out, --output <path>
                       the page (one input), a folder (several), or - for stdout

page:
      --title <text>   the title, instead of the first heading
      --lang <code>    the language (else the file name's, notes.de.md, or en);
                       the page's own words are in 17: en de he id nl jv yi nds
                       fr it es ja ja-Hira ja-Latn zh-Hans zh-Hant yue
      --brand <text>   the name in the bar, instead of md2html
      --footer <text>  the fine print at the foot (Markdown)
      --css <file>     add a stylesheet          --head <file>  add to the <head>
      --set key=value  any front matter setting (repeatable)
      --large          larger type, for a page read at half a screen
      --embed, --self-contained
                       put the images inside the page as well
      --no-glossary    leave the shared glossary out
      --no-toc         no outline
      --html, --unsafe-html
                       trust raw HTML in the Markdown (it is shown as text otherwise)
      --csp            a Content-Security-Policy: only the page's own code runs
      --index          let search engines index the page (noindex by default)
      --no-math        read $ as a dollar sign, not maths
      --mermaid <how>  draw diagrams: native (default), js, or code
      --mermaid-js <f> a local mermaid.js for the diagram kinds drawn in the browser
      --nav            link the pages of one run with previous and next
      --fragment       only the HTML of the Markdown, no page around it
      --pdf            also a PDF beside the page, printed by Chrome (MD2HTML_CHROME)
      --epub           also an EPUB beside the page, for e-readers
      --max-input <MB> the largest Markdown file read (64)
      --max-embed <MB> the largest image embedded (32)

run:
      --check          write nothing; report what to fix; exit 1 if anything
      --watch          render again whenever an input changes
      --serve          serve the pages on this computer; they reload on every save
      --port <n>       the port for --serve (8000)
      --live           the page has a time to live (2 minutes): then it reloads
                       if it changed, keeping the reader's place
      --ttl <seconds>  --live with this time to live
  -q, --quiet          no summary; no warnings, except those --check reports
  -h, --help           this text
  -V, --version        the version

Front matter at the top of the Markdown takes the same settings:
  ---
  title: The plans
  lang: de
  brand: The Hillside house
  ---
";

/// The largest Markdown file md2html reads unless `--max-input` says more.
const MAX_INPUT: u64 = 64 << 20;

/// The port `--serve` tries first.
const PORT: u16 = 8000;

/// What the command line asks for.
#[derive(Debug)]
pub(super) enum Command {
    /// `--help`.
    Help,
    /// `--version`.
    Version,
    /// Pages to make.
    Run(Box<Args>),
}

/// Where the pages go: `-o`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Out {
    /// Beside each input, with `.html` for its extension.
    Beside,
    /// `-o -`: standard output.
    Stdout,
    /// `-o <path>`: that file, or that folder for several pages.
    Path(PathBuf),
}

/// What a run does with the pages it renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    /// Writes them.
    Write,
    /// `--check`: writes nothing and reports what to fix.
    Check,
}

/// How long a run lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// Renders once.
    Once,
    /// `--watch`: renders again whenever an input changes.
    Watch,
    /// `--serve`: serves the pages from this port up.
    Serve(u16),
}

/// A run, as the command line describes it.
#[derive(Debug)]
pub(super) struct Args {
    /// The inputs as given.
    pub(super) inputs: Vec<PathBuf>,
    /// Where the pages go.
    pub(super) out: Out,
    /// Settings over the front matter.
    pub(super) settings: Settings,
    /// Write or only check.
    pub(super) action: Action,
    /// Once, watching, or serving.
    pub(super) mode: Mode,
    /// Whole pages or fragments.
    pub(super) shape: Shape,
    /// `--quiet`.
    pub(super) quiet: bool,
    /// `--nav`.
    pub(super) nav: bool,
    /// `--max-input`, in bytes.
    pub(super) max_input: u64,
}

/// Reads the command line (without the program's name).
///
/// # Errors
///
/// An unknown option, an option without its value, a value it cannot
/// take, no input, or the old two-file form naming a Markdown file as
/// the output.
pub(super) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut parser = Parser {
        args: args.into_iter(),
        run: Args {
            inputs: Vec::new(),
            out: Out::Beside,
            settings: Settings::default(),
            action: Action::Write,
            mode: Mode::Once,
            shape: Shape::Page,
            quiet: false,
            nav: false,
            max_input: MAX_INPUT,
        },
    };
    let mut only_inputs = false;
    while let Some(arg) = parser.args.next() {
        // An option starts with the byte `-`; anything else, whatever
        // its bytes, is an input.
        if only_inputs || arg == "-" || arg.as_encoded_bytes().first() != Some(&b'-') {
            parser.run.inputs.push(PathBuf::from(arg));
            continue;
        }
        let Some(option) = arg.to_str() else {
            return Err(format!("{}: not valid Unicode", Path::new(&arg).display()));
        };
        let (name, inline) = match option.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value)),
            _ => (option, None),
        };
        match parser.option(name, inline)? {
            Next::Go => {}
            Next::OnlyInputs => only_inputs = true,
            Next::Help => return Ok(Command::Help),
            Next::Version => return Ok(Command::Version),
        }
    }
    let mut run = parser.run;
    old_form(&mut run)?;
    if run.inputs.is_empty() {
        return Err(String::from("no input"));
    }
    Ok(Command::Run(Box::new(run)))
}

/// The old form, `md2html in.md out.html`: a second input that is not
/// Markdown is the output.
///
/// # Errors
///
/// When the second of two inputs is a Markdown file that does not exist:
/// the old form would make it the output, so the message names the file
/// and says the output would be a Markdown file.
fn old_form(run: &mut Args) -> Result<(), String> {
    let (Out::Beside, [_, second]) = (&run.out, run.inputs.as_slice()) else {
        return Ok(());
    };
    if !is_markdown(second) && !second.is_dir() && second.as_os_str() != "-" {
        if let Some(out) = run.inputs.pop() {
            run.out = Out::Path(out);
        }
    } else if is_markdown(second) && !second.exists() {
        return Err(format!(
            "{} does not exist; the output would be a Markdown file (md2html a.md b.md once overwrote b.md)",
            second.display()
        ));
    }
    Ok(())
}

/// What the parser does after an option.
enum Next {
    /// Reads on.
    Go,
    /// `--`: everything after it is an input.
    OnlyInputs,
    /// `--help`: nothing else matters.
    Help,
    /// `--version`: nothing else matters.
    Version,
}

/// The arguments not yet read, and the run they describe so far.
struct Parser<Rest> {
    /// What is left of the command line.
    args: Rest,
    /// The run so far.
    run: Args,
}

impl<Rest: Iterator<Item = OsString>> Parser<Rest> {
    /// The value of option `name`: what followed its `=`, or the next
    /// argument.
    ///
    /// # Errors
    ///
    /// When the option has no `=` and is the last argument; the message
    /// is `<name> needs <what>`, for example `--out needs a path`.
    fn value(&mut self, name: &str, inline: Option<&str>, what: &str) -> Result<OsString, String> {
        inline
            .map(OsString::from)
            .or_else(|| self.args.next())
            .ok_or_else(|| format!("{name} needs {what}"))
    }

    /// The value of option `name`, as text.
    ///
    /// # Errors
    ///
    /// When the value is missing (`<name> needs <what>`), or when it is not
    /// valid Unicode (`<name> <value>: not valid Unicode`).
    fn text(&mut self, name: &str, inline: Option<&str>, what: &str) -> Result<String, String> {
        self.value(name, inline, what)?
            .into_string()
            .map_err(|value| format!("{name} {}: not valid Unicode", Path::new(&value).display()))
    }

    /// The value of option `name`, as a path.
    ///
    /// # Errors
    ///
    /// When the value is missing: `<name> needs <what>`. Any bytes make a
    /// path, so a value that is there is always taken.
    fn path(&mut self, name: &str, inline: Option<&str>, what: &str) -> Result<PathBuf, String> {
        self.value(name, inline, what).map(PathBuf::from)
    }

    /// Reads one option.
    ///
    /// # Errors
    ///
    /// When an option that takes a value has none or one that is not valid
    /// Unicode; when `--port` is not a number from 0 to 65535 (the message
    /// gives the reason the number was refused); when `--max-input` is not
    /// a whole number of megabytes from 1 to 1,048,576; and whatever
    /// [`Self::setting`] reports for the options it reads, an unknown
    /// option among them.
    fn option(&mut self, name: &str, inline: Option<&str>) -> Result<Next, String> {
        match name {
            "--" => return Ok(Next::OnlyInputs),
            "-h" | "--help" => return Ok(Next::Help),
            "-V" | "--version" => return Ok(Next::Version),
            "-o" | "--out" | "--output" => {
                let out = self.path(name, inline, "a path")?;
                self.run.out = if out.as_os_str() == "-" {
                    Out::Stdout
                } else {
                    Out::Path(out)
                };
            }
            "--check" => self.run.action = Action::Check,
            "--watch" => {
                if self.run.mode == Mode::Once {
                    self.run.mode = Mode::Watch;
                }
            }
            "--serve" => {
                if !matches!(self.run.mode, Mode::Serve(_)) {
                    self.run.mode = Mode::Serve(PORT);
                }
            }
            "--port" => {
                let port = self.text(name, inline, "a port number")?;
                let number = port
                    .parse()
                    .map_err(|error| format!("--port {port}: a port number ({error})"))?;
                self.run.mode = Mode::Serve(number);
            }
            "-q" | "--quiet" => self.run.quiet = true,
            "--fragment" => self.run.shape = Shape::Fragment,
            "--nav" => self.run.nav = true,
            "--max-input" => {
                self.run.max_input = megabytes(&self.text(name, inline, "a size in MB")?)?;
            }
            _ => self.setting(name, inline)?,
        }
        Ok(Next::Go)
    }

    /// Reads an option that sets one of the page's settings.
    ///
    /// # Errors
    ///
    /// `unknown option <name>` for an option md2html does not have; a
    /// missing or non-Unicode value; `--set` without a `=` in its value;
    /// a `--set` key that is not a setting or a value that setting cannot
    /// take; `--ttl` that is neither a number of seconds nor yes or no,
    /// `--mermaid` that is not native, js or code, and
    /// `--max-embed` that is not a whole number of megabytes from 1 to
    /// 1,048,576.
    fn setting(&mut self, name: &str, inline: Option<&str>) -> Result<(), String> {
        match name {
            "--large" => self.run.settings.large = Some(true),
            "--no-glossary" => self.run.settings.glossary = Some(false),
            "--embed" | "--self-contained" => self.run.settings.embed = Some(true),
            "--html" | "--unsafe-html" => self.run.settings.html = Some(true),
            "--csp" => self.run.settings.csp = Some(true),
            "--pdf" => self.run.settings.pdf = Some(true),
            "--live" => self.run.settings.live = Some(Ttl::DEFAULT),
            "--epub" => self.run.settings.epub = Some(true),
            "--index" => self.run.settings.index = Some(true),
            "--no-toc" => self.run.settings.toc = Some(false),
            "--no-math" => self.run.settings.math = Some(false),
            "--ttl" => {
                let seconds = self.text(name, inline, "a number of seconds")?;
                self.run.settings.set("live", &seconds)?;
            }
            "--max-embed" => {
                let size = megabytes(&self.text(name, inline, "a size in MB")?)?;
                self.run.settings.max_embed = Some(size);
            }
            "--mermaid" => {
                let how = self.text(name, inline, "native, js or code")?;
                self.run.settings.set("mermaid", &how)?;
            }
            "--mermaid-js" => {
                self.run.settings.mermaid_js = Some(self.path(name, inline, "a file")?);
            }
            "--title" => self.run.settings.title = Some(self.text(name, inline, "a title")?),
            "--lang" => {
                let code = self.text(name, inline, "a language code")?;
                self.run.settings.lang = Some(code);
            }
            "--brand" => self.run.settings.brand = Some(self.text(name, inline, "a name")?),
            "--footer" => self.run.settings.footer = Some(self.text(name, inline, "a text")?),
            "--css" => {
                let file = self.path(name, inline, "a file")?;
                self.run.settings.css.push(file);
            }
            "--head" => {
                let file = self.path(name, inline, "a file")?;
                self.run.settings.head.push(file);
            }
            "--set" => {
                let pair = self.text(name, inline, "key=value")?;
                let (key, value) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("--set {pair}: write it as key=value"))?;
                self.run.settings.set(key, value)?;
            }
            other => return Err(format!("unknown option {other}")),
        }
        Ok(())
    }
}

/// A size given in megabytes, in bytes.
///
/// # Errors
///
/// Not a whole number from 1 to 1,048,576.
pub(super) fn megabytes(text: &str) -> Result<u64, String> {
    text.trim()
        .parse::<u64>()
        .ok()
        .filter(|size| (1..=1 << 20_i32).contains(size))
        .map(|size| size << 20_i32)
        .ok_or_else(|| format!("{text}: give a size in megabytes, 1 or more"))
}
