// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's language, the other language versions beside it, and the
//! notice a translation carries.
//!
//! A file named `notes.de.md` is the German version of `notes.md`.

use core::num::NonZeroUsize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::config::{self, Fingerprint, Pair, Settings, Ui, front_matter};
use crate::input::{MAX_BESIDE, read_named, read_named_bytes};
use crate::page::Version;
use crate::render::Warning;
use crate::system;
use crate::text::{self, Html, PathUrl, esc, put};

/// The language a page is in, and its versions.
pub(super) struct Language {
    /// Its code.
    pub(super) lang: String,
    /// The language its file's name gives it.
    suffix: Option<String>,
    /// Its interface words.
    pub(super) ui: &'static Ui,
    /// Every language version beside it, this one included.
    pub(super) versions: Vec<Version>,
}

impl Language {
    /// The language of the page of `source` with `settings`.
    pub(super) fn of(settings: &Settings, source: Option<&Path>) -> Self {
        let suffix = source.and_then(translation_suffix);
        let lang = settings
            .lang
            .clone()
            .or_else(|| suffix.clone())
            .unwrap_or_else(|| String::from("en"));
        let ui = config::ui(&lang);
        Self {
            lang,
            suffix,
            ui,
            versions: source.map(versions_of).unwrap_or_default(),
        }
    }

    /// A translation's notice, which links its original; whether the
    /// translation is older than the original goes to `warnings`.
    pub(super) fn notice(
        &self,
        settings: &Settings,
        source: Option<&Path>,
        warnings: &mut Vec<Warning>,
    ) -> Html {
        let Some(src) = source else {
            return Html::default();
        };
        if settings.translated_from.is_none() && self.suffix.is_none() {
            return Html::default();
        }
        let Some(original) = Original::of(settings, src) else {
            return Html::default();
        };
        if original.path == src {
            warnings.push(Warning {
                line: Some(NonZeroUsize::MIN),
                message: String::from(
                    "translated_from names this page itself; a translation names its original",
                ),
            });
            return Html::default();
        }
        // The original is read only inside the translation's folder (or
        // the folder md2html runs in), where it really is: a file named
        // like the original that links elsewhere is not read.
        let read_from = match text::inside(&original.path, &[folder_of(src)]) {
            Some(real) => real,
            None if original.path.exists() => {
                warnings.push(Warning {
                    line: Some(NonZeroUsize::MIN),
                    message: format!(
                        "the original {} is outside the document's folder; it is not read",
                        original.path.display()
                    ),
                });
                return Html::default();
            }
            None => original.path.clone(),
        };
        let bytes = match read_named_bytes(&read_from, MAX_BESIDE) {
            Ok(bytes) => bytes,
            Err(unread) => {
                warnings.push(Warning {
                    line: Some(NonZeroUsize::MIN),
                    message: format!(
                        "the original {} cannot be read: {}",
                        original.path.display(),
                        unread.describe("")
                    ),
                });
                return Html::default();
            }
        };
        if let Some(message) = staleness(settings, &original.path, &bytes) {
            warnings.push(Warning {
                line: Some(NonZeroUsize::MIN),
                message,
            });
        }
        // The notice links the original it names and was checked against,
        // in that file's own language: not the original the file name
        // points at, which may be another file or this page itself.
        let orig_lang = core::str::from_utf8(&bytes).map_or_else(|_| default_lang(), language_in);
        let orig_name =
            config::lang_info(&orig_lang).map_or(orig_lang.as_str(), |known| known.name);
        let mut notice = String::new();
        put!(
            notice,
            "<p class=\"translated\">{} <a href=\"{}\" hreflang=\"{}\" lang=\"{}\">{}</a></p>",
            esc(&self.ui.lang.notice.fill([orig_name])),
            original.page.attr(),
            esc(&orig_lang),
            esc(&orig_lang),
            esc(self.ui.lang.show_original)
        );
        Html::markup(notice)
    }
}

/// The original a translation names, and its page.
struct Original {
    /// Its Markdown file.
    path: PathBuf,
    /// Its page, relative to the translation's: the name it is given by,
    /// with `.html` for its extension, as a URL.
    page: PathUrl,
}

impl Original {
    /// The original of the translation at `src`: the one `translated_from`
    /// names, else the one its file name points at (`notes.de.md` to
    /// `notes.md`); none when neither names one, or the name is not
    /// Unicode or not relative.
    fn of(settings: &Settings, src: &Path) -> Option<Self> {
        let (path, name) = if let Some(name) = &settings.translated_from {
            (
                src.parent().unwrap_or_else(|| Path::new(".")).join(name),
                name.clone(),
            )
        } else {
            let path = original_of(src)?;
            let name = path.file_name()?.to_str()?.to_owned();
            (path, name)
        };
        let stem = name
            .rsplit_once('.')
            .map_or(name.as_str(), |(stem, _)| stem);
        let page = system::relative_url(Path::new(&format!("{stem}.html")))?;
        Some(Self { path, page })
    }
}

/// The language of a page whose front matter gives none.
fn default_lang() -> String {
    String::from("en")
}

/// The language a Markdown text's front matter gives, by any name of the
/// setting and as its page reads it (`lang` or `language`), else English.
fn language_in(text: &str) -> String {
    front_matter(text)
        .pairs
        .iter()
        .rev()
        .find_map(|Pair { key, value }| {
            let mut one = Settings::default();
            one.set(key, value).ok()?;
            one.lang
        })
        .unwrap_or_else(default_lang)
}

/// What is wrong with a translation's record of its original, if anything.
fn staleness(settings: &Settings, original: &Path, bytes: &[u8]) -> Option<String> {
    let now = Fingerprint::of(bytes);
    match &settings.source_sha256 {
        Some(then) if *then == now => None,
        Some(_) => Some(format!(
            "this translation is older than {}: the original changed since it was translated (source_sha256 now: {now})",
            original.display()
        )),
        None => Some(format!(
            "a translation should record its source's fingerprint: source_sha256: {now}"
        )),
    }
}

/// The language a file's name gives it: `report.de.md` is German.
fn translation_suffix(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let (_, code) = stem.rsplit_once('.')?;
    config::LANGS
        .iter()
        .find(|lang| lang.code.eq_ignore_ascii_case(code))
        .map(|lang| lang.code.to_owned())
}

/// The original a translation's file name points at: `notes.de.md` to
/// `notes.md`; none when the name is not Unicode.
fn original_of(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let base = stem.rsplit_once('.').map_or(stem, |(base, _)| base);
    let ext = path.extension().map_or(Some("md"), OsStr::to_str)?;
    Some(path.with_file_name(format!("{base}.{ext}")))
}

/// Every language version of a page that exists beside it: the original
/// (`notes.md`) and its translations (`notes.de.md`, ...), each with the
/// name its page will have.
fn versions_of(path: &Path) -> Vec<Version> {
    // The versions are linked by name, and a link names a file by text: a
    // name that is not Unicode has no versions (the page says so).
    let Some(original) = (if translation_suffix(path).is_some() {
        original_of(path)
    } else {
        Some(path.to_path_buf())
    }) else {
        return Vec::new();
    };
    let Some(stem) = original.file_stem().and_then(OsStr::to_str) else {
        return Vec::new();
    };
    let Some(ext) = original.extension().map_or(Some("md"), OsStr::to_str) else {
        return Vec::new();
    };
    let here = path.file_name();
    let mut versions = Vec::new();
    if original.is_file() {
        // Its language is read only when it lies inside the folder, where
        // it really is; one that links elsewhere is taken for English.
        let text = text::inside(&original, &[folder_of(&original)])
            .and_then(|real| read_named(&real, MAX_BESIDE).ok())
            .unwrap_or_default();
        versions.push(Version {
            code: language_in(&text),
            href: PathUrl::name(&format!("{stem}.html")),
            current: original.file_name() == here,
            original: true,
        });
    }
    for code in config::LANGS.iter().map(|lang| lang.code) {
        let name = format!("{stem}.{code}.{ext}");
        if original.with_file_name(&name).is_file() {
            versions.push(Version {
                code: code.to_owned(),
                href: PathUrl::name(&format!("{stem}.{code}.html")),
                current: here == Some(OsStr::new(&name)),
                original: false,
            });
        }
    }
    versions
}

/// The folder a file is in, `.` for a bare name.
fn folder_of(file: &Path) -> &Path {
    file.parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::language_in;

    /// The language of another page is read as that page reads it: by any
    /// name of the setting.
    ///
    /// # Panics
    ///
    /// When `lang` or `language` is not read, when the later of the two
    /// (as the page takes it) is not the one given, or when a text without
    /// one is not English.
    #[test]
    fn a_language_is_read_by_any_of_its_names() {
        assert_eq!(language_in("---\nlang: de\n---\n"), "de");
        assert_eq!(language_in("---\nlanguage: fr\n---\n"), "fr");
        assert_eq!(language_in("---\nlang: de\nlanguage: fr\n---\n"), "fr");
        assert_eq!(language_in("# No front matter\n"), "en");
    }
}
