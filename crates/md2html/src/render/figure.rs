// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Images: in a line of text, or alone in a paragraph as a figure with its
//! caption, and the size an author gives one (`{width=60% #map}`) as a
//! rule of the page's stylesheet.

use core::fmt;

use super::Renderer;
use super::nodes::{Links, title_attr};
use crate::model::{Attrs, Image, Node};
use crate::text::{UrlFor, esc, put};

impl Renderer<'_, '_> {
    /// An image inside a line of text.
    pub(super) fn image(&mut self, image: &Image) -> String {
        let alt_text = esc(&crate::inline::plain(&image.alt));
        let Some(url) = self.href(&image.src, UrlFor::Image) else {
            return alt_text;
        };
        let title = title_attr(image.title.as_deref());
        let id = self
            .size_rule(&image.attrs, Placement::Inline)
            .map(|id| format!(" id=\"{id}\""))
            .unwrap_or_default();
        let classes = if image.attrs.classes.is_empty() {
            String::new()
        } else {
            format!(" class=\"{}\"", esc(&image.attrs.classes.join(" ")))
        };
        format!("<img src=\"{url}\" alt=\"{alt_text}\"{title}{id}{classes} loading=\"lazy\">")
    }

    /// An image on a line of its own: a figure with its caption under it,
    /// enlarged over the page on a click (`page/figure.js`).
    pub(super) fn figure(&mut self, image: &Image) -> String {
        self.figures += 1;
        let caption = self.inline(&image.alt, Links::Glossary);
        let alt_text = esc(&crate::inline::plain(&image.alt));
        let Some(url) = self.href(&image.src, UrlFor::Image) else {
            return format!("<p>{caption}</p>");
        };
        self.art.has_figure = true;
        // An image inside the page is enlarged from its own figure: it
        // needs an id to be linked to.
        let embedded = url.starts_with("data:");
        let mut attrs = image.attrs.clone();
        if embedded && attrs.id.is_none() {
            attrs.id = Some(format!("fig-{}", self.figures));
        }
        let id = self.size_rule(&attrs, Placement::Figure);
        let link = if embedded {
            id.as_deref()
                .map(|name| format!("#{name}"))
                .unwrap_or_default()
        } else {
            url.clone()
        };
        let id_attr = id.map(|name| format!(" id=\"{name}\"")).unwrap_or_default();
        let title = title_attr(image.title.as_deref());
        let classes = if attrs.classes.is_empty() {
            String::new()
        } else {
            format!(" {}", esc(&attrs.classes.join(" ")))
        };
        let figcaption = if caption.is_empty() {
            String::new()
        } else {
            format!("<figcaption>{caption}</figcaption>")
        };
        format!(
            "<figure class=\"fig{classes}\"{id_attr}><a href=\"{link}\" target=\"_blank\" rel=\"noopener\">\
<img src=\"{url}\" alt=\"{alt_text}\"{title} loading=\"lazy\"></a>{figcaption}</figure>"
        )
    }

    /// The rules for an image's `{width=... height=...}`, and the id they
    /// hang on; None when it asks for neither and names no id.
    fn size_rule(&mut self, attrs: &Attrs, placement: Placement) -> Option<String> {
        if attrs.width.is_none() && attrs.height.is_none() && attrs.id.is_none() {
            return None;
        }
        if placement == Placement::Inline {
            self.figures += 1;
        }
        let wanted = attrs
            .id
            .clone()
            .unwrap_or_else(|| format!("fig-{}", self.figures));
        let id = self.unique(&wanted);
        let target = match placement {
            Placement::Figure => format!("#{id} img"),
            Placement::Inline => format!("#{id}"),
        };
        if let Some(width) = attrs.width.as_deref().map(Size::read) {
            match width {
                // The figure's frame adds 22 pixels of its own; a share of
                // the column is a share of what is left.
                Size::Share(percent) if placement == Placement::Figure => {
                    put!(
                        self.art.css,
                        "{target}{{width:calc((100% - 22px)*{} + 22px)}}",
                        percent.fraction
                    );
                }
                Size::Share(_) | Size::Pixels(_) | Size::Css(_) => {
                    put!(self.art.css, "{target}{{width:{width};max-width:100%}}");
                }
            }
        }
        if let Some(height) = attrs.height.as_deref().map(Size::read) {
            put!(self.art.css, "{target}{{height:{height};width:auto}}");
        }
        Some(id)
    }
}

/// Where an image with a size stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    /// In a line of text.
    Inline,
    /// Alone, as a figure.
    Figure,
}

/// A width or height as the author wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Size<'text> {
    /// Bare digits: pixels.
    Pixels(&'text str),
    /// `N%`: a share of the column.
    Share(Percent<'text>),
    /// Anything else, which CSS reads as written.
    Css(&'text str),
}

/// A percentage: `60%`, `33.3%`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Percent<'text> {
    /// As written, the `%` included.
    written: &'text str,
    /// What it is a hundredth of.
    fraction: Fraction,
}

impl<'text> Size<'text> {
    /// The size `text` says.
    fn read(text: &'text str) -> Self {
        if text.chars().all(|ch| ch.is_ascii_digit()) {
            Self::Pixels(text)
        } else if let Some(fraction) = Fraction::of_percent(text) {
            Self::Share(Percent {
                written: text,
                fraction,
            })
        } else {
            Self::Css(text)
        }
    }
}

impl fmt::Display for Size<'_> {
    /// The size as a CSS length.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Pixels(digits) => write!(f, "{digits}px"),
            Self::Share(percent) => f.write_str(percent.written),
            Self::Css(text) => f.write_str(text),
        }
    }
}

/// A number from 0 to 429,496.7295 in ten-thousandths: a percentage the
/// author wrote, divided by a hundred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fraction {
    /// The number of ten-thousandths: hundredths of the percentage.
    ten_thousandths: u32,
}

impl Fraction {
    /// The fraction `N%` stands for, `N` in decimal digits with an
    /// optional point. Digits past the hundredths of the percentage are
    /// rounded, a tie to the even neighbour. None for anything else, and
    /// for a percentage too large for the type.
    fn of_percent(text: &str) -> Option<Self> {
        let number = text.strip_suffix('%')?;
        let (whole, after_point) = number.split_once('.').unwrap_or((number, ""));
        if whole.is_empty() && after_point.is_empty() {
            return None;
        }
        let mut decimals = after_point.chars();
        // The digits kept: the whole percentage and its first two decimals,
        // a missing decimal being a zero.
        let kept = whole.chars().chain([
            decimals.next().unwrap_or('0'),
            decimals.next().unwrap_or('0'),
        ]);
        let mut ten_thousandths: u32 = 0;
        for digit in kept {
            let value = digit.to_digit(10)?;
            ten_thousandths = ten_thousandths.checked_mul(10)?.checked_add(value)?;
        }
        let dropped: Vec<u32> = decimals.map(|ch| ch.to_digit(10)).collect::<Option<_>>()?;
        let up = match dropped.split_first() {
            None => false,
            Some((&first, later)) => {
                first > 5
                    || first == 5
                        && (later.iter().any(|&digit| digit > 0) || ten_thousandths % 2 == 1)
            }
        };
        Some(Self {
            ten_thousandths: ten_thousandths.checked_add(u32::from(up))?,
        })
    }
}

impl fmt::Display for Fraction {
    /// The fraction with four decimals: `0.6000`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{:04}",
            self.ten_thousandths / 10_000,
            self.ten_thousandths % 10_000
        )
    }
}

/// The image of a paragraph that holds nothing else.
pub(super) fn lone_image(nodes: &[Node]) -> Option<&Image> {
    let mut shown = nodes.iter().filter(|node| !is_blank(node));
    match (shown.next(), shown.next()) {
        (Some(Node::Image(image)), None) => Some(image),
        _ => None,
    }
}

/// A node that shows nothing: a line break in the source, or spaces.
fn is_blank(node: &Node) -> bool {
    match node {
        Node::SoftBreak => true,
        Node::Text(text) => text.trim().is_empty(),
        Node::Code(_)
        | Node::Html(_)
        | Node::HardBreak
        | Node::Em(_)
        | Node::Strong(_)
        | Node::Del(_)
        | Node::Link(_)
        | Node::Image(_)
        | Node::Note(_)
        | Node::Math(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{Fraction, Size};

    /// The fraction `text` stands for, written as the stylesheet has it.
    fn fraction(text: &str) -> Option<String> {
        Fraction::of_percent(text).map(|found| found.to_string())
    }

    /// A percentage becomes the fraction it stands for, to four decimal places.
    ///
    /// # Panics
    ///
    /// Panics when `60%`, `150%`, `007%`, `.5%` or `5.%` is not the fraction it
    /// stands for; when digits past the fourth place are not rounded, a tie to
    /// the even neighbour (`3.125%` to `0.0312`, `3.135%` to `0.0314`) and
    /// anything past a tie up; when the largest percentage the type holds,
    /// `42949672.95%`, is refused or the next one up is not; or when an
    /// exponent, a bare point, a bare `%`, a number without `%`, or two points
    /// are read as a fraction.
    #[test]
    fn a_percentage_is_a_fraction_to_four_places() {
        assert_eq!(fraction("60%").as_deref(), Some("0.6000"));
        assert_eq!(fraction("150%").as_deref(), Some("1.5000"));
        assert_eq!(fraction("33.333%").as_deref(), Some("0.3333"));
        assert_eq!(fraction("33.3366%").as_deref(), Some("0.3334"));
        assert_eq!(fraction("007%").as_deref(), Some("0.0700"));
        assert_eq!(fraction(".5%").as_deref(), Some("0.0050"));
        assert_eq!(fraction("5.%").as_deref(), Some("0.0500"));
        assert_eq!(fraction("3.125%").as_deref(), Some("0.0312"));
        assert_eq!(fraction("3.135%").as_deref(), Some("0.0314"));
        assert_eq!(fraction("3.12501%").as_deref(), Some("0.0313"));
        assert_eq!(fraction("429496.7295%").as_deref(), Some("4294.9673"));
        assert_eq!(fraction("42949672.95%").as_deref(), Some("429496.7295"));
        assert_eq!(fraction("42949672.96%"), None);
        assert_eq!(fraction("1e2%"), None);
        assert_eq!(fraction(".%"), None);
        assert_eq!(fraction("%"), None);
        assert_eq!(fraction("60"), None);
        assert_eq!(fraction("6.0.0%"), None);
    }

    /// An image size is read as pixels, a share of the width, or a CSS length
    /// kept as written.
    ///
    /// # Panics
    ///
    /// Panics when a bare `40` does not become `40px`, when `60%` is not a
    /// share written back as `60%`, when `12em` is not kept as written, or when
    /// `inf%`, which is no number, is read as a share instead of kept as CSS.
    #[test]
    fn a_size_is_pixels_a_share_or_css() {
        assert_eq!(Size::read("40").to_string(), "40px");
        assert_eq!(Size::read("60%").to_string(), "60%");
        assert_eq!(Size::read("12em").to_string(), "12em");
        assert!(matches!(Size::read("60%"), Size::Share(_)));
        assert!(matches!(Size::read("inf%"), Size::Css("inf%")));
    }
}
