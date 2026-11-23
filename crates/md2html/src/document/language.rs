// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's language, the other language versions beside it, and the
//! notice a translation carries.
//!
//! A file named `notes.de.md` is the German version of `notes.md`.

use std::path::{Path, PathBuf};

use crate::config::{self, Settings, Ui, front_matter};
use crate::input::{MAX_BESIDE, read_bytes, read_file};
use crate::page::Version;
use crate::render::Warning;
use crate::text::{self, esc, put};

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
    ) -> String {
        let Some(src) = source else {
            return String::new();
        };
        if settings.translated_from.is_none() && self.suffix.is_none() {
            return String::new();
        }
        let original = settings.translated_from.as_ref().map_or_else(
            || original_of(src),
            |name| src.parent().unwrap_or_else(|| Path::new(".")).join(name),
        );
        let bytes = match read_bytes(&original, MAX_BESIDE) {
            Ok(bytes) => bytes,
            Err(unread) => {
                warnings.push(Warning {
                    line: 1,
                    message: format!(
                        "the original {} cannot be read: {}",
                        original.display(),
                        unread.describe("")
                    ),
                });
                return String::new();
            }
        };
        if let Some(message) = staleness(settings, &original, &bytes) {
            warnings.push(Warning { line: 1, message });
        }
        let first = self.versions.iter().find(|version| version.original);
        let orig_lang = first.map_or("en", |version| version.code.as_str());
        let orig_name = config::lang_info(orig_lang).map_or(orig_lang, |known| known.name);
        let href = first
            .map(|version| version.href.clone())
            .unwrap_or_default();
        let mut notice = String::new();
        put!(
            notice,
            "<p class=\"translated\">{} <a href=\"{}\" hreflang=\"{}\" lang=\"{}\">{}</a></p>",
            esc(&self.ui.lang.notice.fill([orig_name])),
            text::url_attr(&href),
            esc(orig_lang),
            esc(orig_lang),
            esc(self.ui.lang.show_original)
        );
        notice
    }
}

/// What is wrong with a translation's record of its original, if anything.
fn staleness(settings: &Settings, original: &Path, bytes: &[u8]) -> Option<String> {
    let now = text::hex(&text::sha256(bytes));
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
/// `notes.md`.
fn original_of(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let base = stem.rsplit_once('.').map_or(stem, |(base, _)| base);
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("md");
    path.with_file_name(format!("{base}.{ext}"))
}

/// Every language version of a page that exists beside it: the original
/// (`notes.md`) and its translations (`notes.de.md`, ...), each with the
/// name its page will have.
fn versions_of(path: &Path) -> Vec<Version> {
    let original = if translation_suffix(path).is_some() {
        original_of(path)
    } else {
        path.to_path_buf()
    };
    let stem = original
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let ext = original
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("md");
    let here = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut versions = Vec::new();
    if original.is_file() {
        let text = read_file(&original, MAX_BESIDE).unwrap_or_default();
        let (map, _, _) = front_matter(&text);
        let code = map
            .get("lang")
            .cloned()
            .unwrap_or_else(|| String::from("en"));
        let name = original
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        versions.push(Version {
            code,
            href: format!("{stem}.html"),
            current: name == here,
            original: true,
        });
    }
    for code in config::LANGS.iter().map(|lang| lang.code) {
        let name = format!("{stem}.{code}.{ext}");
        if original.with_file_name(&name).is_file() {
            versions.push(Version {
                code: code.to_owned(),
                href: format!("{stem}.{code}.html"),
                current: name == here,
                original: false,
            });
        }
    }
    versions
}
