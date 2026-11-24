// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Images put inside the page as `data:` URIs, their type read from their
//! first bytes, never from their name.
//!
//! [`read_image`] is how md2html reads an image file a document names, for
//! the page ([`data_uri`]) and for the EPUB alike: only a file, through a
//! limit, and only when its bytes are an image.

use std::path::Path;

use crate::input::{Unread, read_named_bytes};

/// The largest image put inside a page unless `--max-embed` says more.
pub(crate) const MAX_EMBED: u64 = 32 << 20;

/// How far into a file an SVG's `<svg` is looked for.
const SVG_HEAD: usize = 512;

/// Bytes at a place in a file.
struct Mark {
    /// Where they start.
    at: usize,
    /// What they are.
    bytes: &'static [u8],
}

/// A kind of image by its marks: a file with every one of them is one.
struct Signature {
    /// The media type.
    mime: &'static str,
    /// The bytes it has, where it has them.
    marks: &'static [Mark],
}

/// The images known by their first bytes, in the order they are tried.
const SIGNATURES: &[Signature] = &[
    Signature {
        mime: "image/png",
        marks: &[Mark {
            at: 0,
            bytes: b"\x89PNG\r\n\x1a\n",
        }],
    },
    Signature {
        mime: "image/jpeg",
        marks: &[Mark {
            at: 0,
            bytes: b"\xFF\xD8\xFF",
        }],
    },
    Signature {
        mime: "image/gif",
        marks: &[Mark {
            at: 0,
            bytes: b"GIF87a",
        }],
    },
    Signature {
        mime: "image/gif",
        marks: &[Mark {
            at: 0,
            bytes: b"GIF89a",
        }],
    },
    Signature {
        mime: "image/webp",
        marks: &[
            Mark {
                at: 0,
                bytes: b"RIFF",
            },
            Mark {
                at: 8,
                bytes: b"WEBP",
            },
        ],
    },
    Signature {
        mime: "image/avif",
        marks: &[
            Mark {
                at: 4,
                bytes: b"ftyp",
            },
            Mark {
                at: 8,
                bytes: b"avif",
            },
        ],
    },
    Signature {
        mime: "image/avif",
        marks: &[
            Mark {
                at: 4,
                bytes: b"ftyp",
            },
            Mark {
                at: 8,
                bytes: b"avis",
            },
        ],
    },
    Signature {
        mime: "image/x-icon",
        marks: &[Mark {
            at: 0,
            bytes: b"\0\0\x01\0",
        }],
    },
    Signature {
        mime: "image/bmp",
        marks: &[Mark {
            at: 0,
            bytes: b"BM",
        }],
    },
];

impl Mark {
    /// Whether `file` has these bytes at this place.
    fn on(&self, file: &[u8]) -> bool {
        file.split_at_checked(self.at)
            .is_some_and(|(_, from)| from.starts_with(self.bytes))
    }
}

/// An image file, read whole.
pub(crate) struct ImageFile {
    /// Its bytes.
    pub(crate) bytes: Vec<u8>,
    /// The media type its first bytes show (`image/png`).
    pub(crate) mime: &'static str,
}

/// Why a file a document names gives no image.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NoImage {
    /// It could not be read: not there, not a file, too large, unreadable.
    Unread(Unread),
    /// It was read, and its bytes are no image md2html knows.
    NotAnImage,
}

impl NoImage {
    /// What to tell the person, after the name of the image.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Unread(Unread::TooLarge(max)) => format!(
                "it is larger than {} MB (--max-embed raises the limit)",
                max >> 20_i32
            ),
            Self::Unread(unread) => unread.describe(""),
            Self::NotAnImage => String::from("its contents are not an image"),
        }
    }
}

/// The image file at `path`: a file (not a folder, a pipe or a device),
/// read through `max` bytes, whose first bytes are an image.
///
/// # Errors
///
/// [`NoImage::Unread`] when it is not there, not a file, larger than `max`
/// or cannot be read; [`NoImage::NotAnImage`] when its first bytes match no
/// known image and it does not open like an SVG.
pub(crate) fn read_image(path: &Path, max: u64) -> Result<ImageFile, NoImage> {
    let bytes = read_named_bytes(path, max).map_err(NoImage::Unread)?;
    // The bytes say what the file is, not its name.
    let mime = image_kind(&bytes).ok_or(NoImage::NotAnImage)?;
    Ok(ImageFile { bytes, mime })
}

/// A file as a `data:` URI, its type from its first bytes.
///
/// # Errors
///
/// Returns `it is outside the page's folder and the folder md2html runs in`
/// when `path` (links resolved) lies in none of `bases` nor the working folder,
/// or does not exist; otherwise what [`NoImage::describe`] says when
/// [`read_image`] refuses it (`it is larger than <n> MB (--max-embed raises the
/// limit)`, `it is not a file`, `its contents are not an image`, the system's
/// message).
pub(super) fn data_uri(path: &Path, bases: &[&Path], max: u64) -> Result<String, String> {
    if !crate::text::inside(path, bases) {
        return Err(String::from(
            "it is outside the page's folder and the folder md2html runs in",
        ));
    }
    let image = read_image(path, max).map_err(|refused| refused.describe())?;
    let mut url = format!("data:{};base64,", image.mime);
    crate::text::base64_into(&mut url, &image.bytes);
    Ok(url)
}

/// An image's media type from its first bytes.
fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    SIGNATURES
        .iter()
        .find(|signature| signature.marks.iter().all(|mark| mark.on(bytes)))
        .map(|signature| signature.mime)
        .or_else(|| is_svg(bytes).then_some("image/svg+xml"))
}

/// Whether a file is SVG: it opens with `<svg`, or with an XML
/// declaration or a comment and has `<svg` in its first bytes. A byte
/// order mark and white space before the opening do not count.
fn is_svg(bytes: &[u8]) -> bool {
    let head = bytes.chunks(SVG_HEAD).next().unwrap_or_default();
    // The opening is read as text up to the first byte that is not UTF-8;
    // every opening looked for is ASCII.
    let text = head
        .utf8_chunks()
        .next()
        .map(|chunk| chunk.valid())
        .unwrap_or_default();
    let opening = text.trim_start_matches('\u{feff}').trim_start();
    opening.starts_with("<svg")
        || (opening.starts_with("<?xml") || opening.starts_with("<!--"))
            && head.windows(4).any(|word| word == b"<svg")
}

#[cfg(test)]
mod tests {
    use super::image_kind;

    /// An image's type is read from the marks at the start of its bytes.
    ///
    /// # Panics
    ///
    /// Panics when PNG, JPEG, GIF, WebP, AVIF (both brands), icon or BMP bytes
    /// are not given their media type; when a RIFF file that is a WAVE, or a
    /// bare `RIFF` too short to say, is taken for WebP; or when no bytes at all
    /// are taken for an image.
    #[test]
    fn an_image_is_known_by_its_first_bytes() {
        assert_eq!(image_kind(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(image_kind(b"\xFF\xD8\xFF\xE0"), Some("image/jpeg"));
        assert_eq!(image_kind(b"GIF89a..."), Some("image/gif"));
        assert_eq!(image_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(image_kind(b"RIFF\0\0\0\0WAVE"), None);
        assert_eq!(image_kind(b"\0\0\0\x1cftypavif"), Some("image/avif"));
        assert_eq!(image_kind(b"\0\0\0\x1cftypavis"), Some("image/avif"));
        assert_eq!(image_kind(b"\0\0\x01\0\x01\0"), Some("image/x-icon"));
        assert_eq!(image_kind(b"BM...."), Some("image/bmp"));
        assert_eq!(image_kind(b"RIFF"), None);
        assert_eq!(image_kind(b""), None);
    }

    /// An SVG is known by `<svg` at its start, or soon after an XML declaration
    /// or a comment.
    ///
    /// # Panics
    ///
    /// Panics when `<svg` at the start, or after a byte order mark, white space
    /// and an XML declaration, or after a comment (one with a byte that is not
    /// UTF-8 included), is not taken for SVG; or when an XML declaration before
    /// `<html>`, `<svg>` inside `<html>`, a stray byte before `<svg`, or `<svg`
    /// past the first 512 bytes is.
    #[test]
    fn svg_is_known_by_its_opening() {
        assert_eq!(image_kind(b"<svg xmlns=\"\"/>"), Some("image/svg+xml"));
        assert_eq!(
            image_kind(b"\xEF\xBB\xBF \n<?xml version=\"1.0\"?><svg/>"),
            Some("image/svg+xml")
        );
        assert_eq!(
            image_kind(b"<!-- a drawing --><svg/>"),
            Some("image/svg+xml")
        );
        assert_eq!(image_kind(b"<!-- \xFF --><svg/>"), Some("image/svg+xml"));
        assert_eq!(image_kind(b"<?xml version=\"1.0\"?><html/>"), None);
        assert_eq!(image_kind(b"<html><svg/></html>"), None);
        assert_eq!(image_kind(b"\xFF<svg/>"), None);
        let mut late = b"<!--".to_vec();
        late.resize(600, b' ');
        late.extend_from_slice(b"<svg/>");
        assert_eq!(image_kind(&late), None);
    }
}
