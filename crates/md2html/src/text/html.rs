// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! HTML as a type of its own, so that text written into a page is escaped
//! once: never twice, never not at all.

use core::fmt;

use super::esc;

/// HTML: what a page may carry as it is.
///
/// Plain text becomes HTML through [`Html::escape`], which escapes it.
/// Markup that md2html writes itself (from its own tags and from parts it
/// escaped) becomes HTML through [`Html::markup`]. An `Html` is not a
/// `&str`, so it cannot be escaped a second time by mistake, and a plain
/// `String` is not an `Html`, so it cannot stand where markup is expected
/// without one of the two.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Html(String);

impl Html {
    /// Plain text as HTML: `&`, `<`, `>` and `"` written as references.
    #[must_use]
    pub(crate) fn escape(text: &str) -> Self {
        Self(esc(text))
    }

    /// Markup that md2html wrote: its own tags around parts it escaped or
    /// made safe. Plain text goes through [`Html::escape`] instead.
    #[must_use]
    pub(crate) const fn markup(markup: String) -> Self {
        Self(markup)
    }

    /// Whether there is no HTML at all.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `more` after this.
    pub(crate) fn push(&mut self, more: &Self) {
        self.0.push_str(&more.0);
    }
}

impl fmt::Display for Html {
    /// Writes the HTML as it is: it is already fit for the page.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Html;

    /// Text is escaped once, on the way in, and written as it is after.
    ///
    /// # Panics
    ///
    /// When `escape` leaves a `<`, `&` or `"` as it is, when writing an
    /// `Html` escapes it again, or when markup is changed on the way in.
    #[test]
    fn text_is_escaped_once() {
        let html = Html::escape("<b> & \"q\"");
        assert_eq!(format!("{html}"), "&lt;b&gt; &amp; &quot;q&quot;");
        let markup = Html::markup(String::from("<em>x</em>"));
        assert_eq!(format!("{markup}"), "<em>x</em>");
        let mut both = markup;
        both.push(&html);
        assert_eq!(both.to_string(), "<em>x</em>&lt;b&gt; &amp; &quot;q&quot;");
        assert!(Html::default().is_empty());
    }
}
