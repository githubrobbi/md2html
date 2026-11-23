// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The page's images, moved into the book.

use std::ffi::OsStr;
use std::path::Path;

use crate::scan::Scan;
use crate::text::{base64_decode, percent_decode, percent_decode_path};

/// The kinds of image an e-reader shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Media {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
    /// GIF.
    Gif,
    /// WebP.
    Webp,
    /// SVG.
    Svg,
}

impl Media {
    /// The kind a media type names.
    fn of_type(media_type: &str) -> Option<Self> {
        match media_type {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/gif" => Some(Self::Gif),
            "image/webp" => Some(Self::Webp),
            "image/svg+xml" => Some(Self::Svg),
            _ => None,
        }
    }

    /// The kind a file's extension names, in either case.
    fn of_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::Webp),
            "svg" => Some(Self::Svg),
            _ => None,
        }
    }

    /// Its media type.
    pub(super) const fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Svg => "image/svg+xml",
        }
    }

    /// The extension its file in the book gets.
    const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Svg => "svg",
        }
    }
}

/// An image's bytes and their kind.
pub(super) struct Picture {
    /// What kind of image it is.
    pub(super) media: Media,
    /// The image.
    pub(super) bytes: Vec<u8>,
}

/// An image packed into the book.
pub(super) struct Image {
    /// Where the page had it.
    src: String,
    /// Its file in the book.
    pub(super) name: String,
    /// The image itself.
    pub(super) picture: Picture,
}

/// An article with its images moved into the book.
pub(super) struct Packed {
    /// The article: `src` and links to the packed images point into the
    /// book.
    pub(super) html: String,
    /// The images, in the order the article first shows them.
    pub(super) images: Vec<Image>,
    /// Where the page had the images that are left out.
    pub(super) left_out: Vec<String>,
    /// The images that could not be packed, and why.
    pub(super) problems: Vec<String>,
}

/// The article's images moved into the book. An image that cannot be,
/// one on the web among them (a book holds its images itself), is left
/// out, its text alternative in its place.
pub(super) fn pack(html: &str, bases: &[&Path]) -> Packed {
    let mut packed = Packed {
        html: String::with_capacity(html.len()),
        images: Vec::new(),
        left_out: Vec::new(),
        problems: Vec::new(),
    };
    let mut scan = Scan::new(html);
    while let Some(before) = scan.take_until("<img") {
        packed.html.push_str(before);
        let start = scan.mark();
        if scan.take_until(">").is_some() {
            scan.eat('>');
        } else {
            scan.take_rest();
        }
        packed.img(scan.since(start), bases);
    }
    packed.html.push_str(scan.take_rest());
    packed.relink();
    packed
}

impl Packed {
    /// Writes one `<img>` tag, pointing at its packed copy; an image that
    /// is not packed gives way to its text alternative.
    fn img(&mut self, tag: &str, bases: &[&Path]) {
        let Some(src) = super::between(tag, " src=\"", "\"") else {
            self.html.push_str(tag);
            return;
        };
        let packed = if src.starts_with("http://") || src.starts_with("https://") {
            self.problem(format!(
                "epub: an image is left out: {src} is on the web, and a book holds its images itself"
            ));
            None
        } else {
            self.name_for(src, bases)
        };
        if let Some(name) = packed {
            self.html.push_str(&tag.replacen(
                &format!(" src=\"{src}\""),
                &format!(" src=\"{name}\""),
                1,
            ));
        } else {
            self.html
                .push_str(super::between(tag, " alt=\"", "\"").unwrap_or_default());
            if !self.left_out.iter().any(|seen| seen == src) {
                self.left_out.push(src.to_owned());
            }
        }
    }

    /// The name in the book of the image at `src`, packing it when it is
    /// not yet. An image that cannot be packed has none, and is a problem.
    fn name_for(&mut self, src: &str, bases: &[&Path]) -> Option<String> {
        if let Some(image) = self.images.iter().find(|image| image.src == src) {
            return Some(image.name.clone());
        }
        match load(src, bases) {
            Ok(picture) => {
                let name = format!(
                    "images/img-{}.{}",
                    self.images.len() + 1,
                    picture.media.extension()
                );
                self.images.push(Image {
                    src: src.to_owned(),
                    name: name.clone(),
                    picture,
                });
                Some(name)
            }
            Err(why) => {
                self.problem(format!("epub: an image is left out: {why}"));
                None
            }
        }
    }

    /// Notes a problem, once.
    fn problem(&mut self, problem: String) {
        if !self.problems.contains(&problem) {
            self.problems.push(problem);
        }
    }

    /// Points links to a packed image at its copy in the book.
    fn relink(&mut self) {
        for image in &self.images {
            self.html = self.html.replace(
                &format!("href=\"{}\"", image.src),
                &format!("href=\"{}\"", image.name),
            );
        }
    }
}

/// An image from a `data:` URL or a file beside the page.
fn load(src: &str, bases: &[&Path]) -> Result<Picture, String> {
    src.strip_prefix("data:")
        .map_or_else(|| from_file(src, bases), from_data)
}

/// An image from the part of a `data:` URL after `data:`.
fn from_data(data: &str) -> Result<Picture, String> {
    let (head, payload) = data.split_once(',').ok_or("a data: URL without data")?;
    let media_type = head
        .split_once(';')
        .map_or(head, |(media_type, _)| media_type);
    let media = Media::of_type(media_type)
        .ok_or_else(|| format!("{media_type} is not an image an e-reader shows"))?;
    let bytes = if head.ends_with(";base64") {
        base64_decode(payload).ok_or("a data: URL that is not base64")?
    } else {
        percent_decode(payload)
    };
    Ok(Picture { media, bytes })
}

/// An image from a file in the page's folder: the first of `bases` that
/// has it.
fn from_file(src: &str, bases: &[&Path]) -> Result<Picture, String> {
    let address = src
        .split_once(['?', '#'])
        .map_or(src, |(address, _)| address);
    // A path that decodes to bytes which are not UTF-8 names no file.
    let path = percent_decode_path(address).ok_or_else(|| format!("{address} was not found"))?;
    let media = Path::new(&path)
        .extension()
        .and_then(OsStr::to_str)
        .and_then(Media::of_extension)
        .ok_or_else(|| format!("{path} is not an image an e-reader shows"))?;
    if Path::new(&path).is_absolute() || path.split(['/', '\\']).any(|part| part == "..") {
        return Err(format!("{path} is outside the page's folder"));
    }
    bases
        .iter()
        .find_map(|base| std::fs::read(base.join(&path)).ok())
        .map(|bytes| Picture { media, bytes })
        .ok_or_else(|| format!("{path} was not found"))
}

#[cfg(test)]
mod tests {
    use super::{Media, load, pack};

    #[test]
    fn a_data_image_keeps_its_bytes() {
        // Percent-encoded, not base64: the bytes are not UTF-8 and must
        // arrive as they are, not as replacement characters.
        let picture = load("data:image/png,%89PNG%0D%0A%1A%0A%ff%fe%00", &[]).unwrap();
        assert_eq!(picture.media, Media::Png);
        assert_eq!(picture.bytes, [
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0xFE, 0x00
        ]);
    }

    #[test]
    fn an_image_address_cannot_split_a_character() {
        // `%a` followed by a two-byte character: reading the escape must not
        // cut the character in half (0.7.6 stopped here with a panic).
        for src in [
            "%a\u{e9}.png",
            "%\u{e9}",
            "a%",
            "%%%",
            "%+f.png",
            "\u{1f600}%f",
        ] {
            assert!(load(src, &[]).is_err(), "{src} names no image, and says so");
        }
    }

    #[test]
    fn an_address_that_is_not_utf8_names_no_image() {
        let dir = std::env::temp_dir().join(format!("md2html-images-utf8-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A file whose name is what replacing the bytes would make.
        std::fs::write(dir.join("\u{fffd}.png"), b"\x89PNG").unwrap();
        let bases = [dir.as_path()];
        assert_eq!(load("%EF%BF%BD.png", &bases).unwrap().bytes, b"\x89PNG");
        for src in ["%ff.png", "%C3.png", "%80%80.png"] {
            assert_eq!(
                load(src, &bases).err().as_deref(),
                Some(format!("{src} was not found").as_str())
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_image_is_packed_once_and_linked_to() {
        let src = "data:image/gif;base64,R0lGODlhAQABAAAAACw=";
        let html = format!(
            "<a href=\"{src}\"><img src=\"{src}\" alt=\"a\"></a><img src=\"{src}\"><img src=\"https://example.com/b.png\"><img src=\"c.bmp\">"
        );
        let packed = pack(&html, &[]);
        assert_eq!(packed.images.len(), 1);
        assert_eq!(packed.left_out, ["https://example.com/b.png", "c.bmp"]);
        assert_eq!(
            packed.html,
            "<a href=\"images/img-1.gif\"><img src=\"images/img-1.gif\" alt=\"a\"></a><img src=\"images/img-1.gif\">"
        );
    }

    #[test]
    fn an_image_not_in_the_book_gives_way_to_its_text() {
        // A book holds its images itself: an image on the web or one that
        // cannot be read would be a reference to nothing in it.
        let html = "<p><img src=\"https://example.com/b.png\" alt=\"the badge\"> \
<img src=\"https://example.com/b.png\" alt=\"again\"> <img src=\"missing.png\" alt=\"gone &amp; lost\"> <img src=\"c.bmp\"></p>";
        let packed = pack(html, &[]);
        assert_eq!(packed.images.len(), 0);
        assert_eq!(packed.html, "<p>the badge again gone &amp; lost </p>");
        assert_eq!(packed.left_out, [
            "https://example.com/b.png",
            "missing.png",
            "c.bmp"
        ]);
        assert_eq!(packed.problems, [
            "epub: an image is left out: https://example.com/b.png is on the web, and a book holds its images itself",
            "epub: an image is left out: missing.png was not found",
            "epub: an image is left out: c.bmp is not an image an e-reader shows",
        ]);
    }
}
