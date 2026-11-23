// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Images put inside the page as `data:` URIs, their type read from their
//! first bytes, never from their name.

use std::path::Path;

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

/// A file as a `data:` URI, its type from its first bytes.
pub(super) fn data_uri(path: &Path, bases: &[&Path], max: u64) -> Result<String, String> {
    if !crate::text::inside(path, bases) {
        return Err(String::from(
            "it is outside the page's folder and the folder md2html runs in",
        ));
    }
    let size = std::fs::metadata(path)
        .map_err(|err| err.to_string())?
        .len();
    if size > max {
        return Err(format!(
            "it is larger than {} MB (--max-embed raises the limit)",
            max >> 20
        ));
    }
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    // The bytes say what the file is, not its name: only an image is embedded.
    let mime = image_kind(&bytes).ok_or("its contents are not an image")?;
    Ok(format!(
        "data:{mime};base64,{}",
        crate::text::base64(&bytes)
    ))
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
