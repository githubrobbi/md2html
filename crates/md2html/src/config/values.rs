// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The values of the settings that are more than words or yes and no,
//! each read once from what the front matter or the command line says.
//!
//! What the setting may be is the type's to say: a page reads the value,
//! it does not check the words again.

use core::fmt;

use crate::text::{hex, sha256};

/// How Mermaid diagrams are drawn: the `mermaid` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Diagrams {
    /// Drawn here as SVG; what cannot be is left to mermaid.js, if given.
    Native,
    /// Every diagram left to mermaid.js.
    Js,
    /// Shown as code.
    Code,
}

impl Diagrams {
    /// The setting's value: `native`, `js` or `code`, in any case.
    ///
    /// # Errors
    ///
    /// ``mermaid: `<value>` is not native, js or code`` for any other value.
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "native" => Ok(Self::Native),
            "js" => Ok(Self::Js),
            "code" => Ok(Self::Code),
            _ => Err(format!("mermaid: `{value}` is not native, js or code")),
        }
    }
}

/// What a page's language switch offers: the `translations` setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Translations {
    /// `false`: a switch only when versions are beside the page, listing
    /// only them; the reader's browser is not offered to translate.
    VersionsOnly,
    /// Any other value (`device`), and the default: a switch on every
    /// page, offering the versions beside it and the browser's translation;
    /// on a page with no version beside it, shown only by a browser that
    /// translates.
    Device,
}

impl Translations {
    /// The setting's value: `false`, in any case, or anything else for the
    /// default.
    #[must_use]
    pub(crate) fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("false") {
            Self::VersionsOnly
        } else {
            Self::Device
        }
    }
}

/// The SHA-256 of a file, as lowercase hexadecimal: the `source_sha256`
/// a translation records of its original.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fingerprint(String);

impl Fingerprint {
    /// The fingerprint as an author writes it, spaces around it taken
    /// off and its letters lowercased, so `ABC` and `abc` are the same.
    #[must_use]
    pub(crate) fn parse(value: &str) -> Self {
        Self(value.trim().to_ascii_lowercase())
    }

    /// The fingerprint of `bytes`.
    #[must_use]
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(hex(&sha256(bytes)))
    }
}

impl fmt::Display for Fingerprint {
    /// The hexadecimal digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A live page's time to live: the `live` setting, in seconds, from five
/// seconds to a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ttl(u32);

impl Ttl {
    /// Two minutes: `live: yes` and `--live`.
    pub(crate) const DEFAULT: Self = Self(120);
    /// The shortest: five seconds.
    const SHORTEST: u32 = 5;
    /// The longest: a day.
    const LONGEST: u32 = 86_400;

    /// `seconds`, held between five seconds and a day.
    #[must_use]
    pub(crate) const fn seconds(seconds: u32) -> Self {
        if seconds < Self::SHORTEST {
            Self(Self::SHORTEST)
        } else if seconds > Self::LONGEST {
            Self(Self::LONGEST)
        } else {
            Self(seconds)
        }
    }

    /// The setting's value: yes is two minutes, no is off (None), a number
    /// is seconds (with or without an `s`), held between five seconds and a
    /// day.
    ///
    /// # Errors
    ///
    /// When the value is none of these, or a number too large for a `u32`:
    /// `live: <value>: true, false or a number of seconds (<why>)`.
    pub(crate) fn parse(value: &str) -> Result<Option<Self>, String> {
        let word = value.trim().to_ascii_lowercase();
        match word.as_str() {
            "true" | "yes" | "on" => Ok(Some(Self::DEFAULT)),
            "false" | "no" | "off" => Ok(None),
            number => number
                .trim_end_matches('s')
                .parse::<u32>()
                .map(|seconds| Some(Self::seconds(seconds)))
                .map_err(|err| {
                    format!("live: {value}: true, false or a number of seconds ({err})")
                }),
        }
    }
}

impl fmt::Display for Ttl {
    /// The seconds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagrams, Fingerprint, Translations, Ttl};

    /// Each value is read once, in any case and with spaces around it.
    ///
    /// # Panics
    ///
    /// When a word is read as another value than the one it names, when
    /// `mermaid` takes a word it does not know or refuses with another
    /// message, when a fingerprint keeps its case or spaces, or when a time
    /// to live is not held between five seconds and a day.
    #[test]
    fn each_value_is_read_once() {
        assert_eq!(Diagrams::parse(" JS "), Ok(Diagrams::Js));
        assert_eq!(Diagrams::parse("code"), Ok(Diagrams::Code));
        assert_eq!(Diagrams::parse("Native"), Ok(Diagrams::Native));
        assert_eq!(
            Diagrams::parse("svg"),
            Err(String::from("mermaid: `svg` is not native, js or code"))
        );
        assert_eq!(Translations::parse("False"), Translations::VersionsOnly);
        assert_eq!(Translations::parse(" device"), Translations::Device);
        assert_eq!(Translations::parse("true"), Translations::Device);
        assert_eq!(Fingerprint::parse(" ABC0 "), Fingerprint::parse("abc0"));
        assert_eq!(Fingerprint::of(b"").to_string().len(), 64);
        assert_eq!(Ttl::seconds(1), Ttl::seconds(5));
        assert_eq!(Ttl::seconds(100_000).to_string(), "86400");
        assert_eq!(Ttl::parse("30s"), Ok(Some(Ttl::seconds(30))));
        assert_eq!(Ttl::parse("no"), Ok(None));
    }
}
