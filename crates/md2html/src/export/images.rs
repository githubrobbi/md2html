// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The page's images, moved into the book.

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use crate::scan::Scan;
use crate::system;
use crate::text::{self, base64_decode, percent_decode, percent_decode_path};

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
///
/// # Errors
///
/// Returns the errors of [`from_data`] for a `data:` URL and of [`from_file`]
/// for anything else: a short reason the image cannot be packed, which the book
/// lists as `epub: an image is left out: <reason>`.
fn load(src: &str, bases: &[&Path]) -> Result<Picture, String> {
    src.strip_prefix("data:")
        .map_or_else(|| from_file(src, bases), from_data)
}

/// An image from the part of a `data:` URL after `data:`.
///
/// # Errors
///
/// Returns `a data: URL without data` when there is no comma, `<type> is not an
/// image an e-reader shows` when the media type is not one [`Media`] knows, and
/// `a data: URL that is not base64` when a `;base64` payload does not decode.
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
///
/// The address must be names only, read with the system's separators (no
/// root, no drive, no `..`; on Windows `\x.png` and `C:x.png` lead off the
/// folder), and where it leads, links followed, must lie in one of `bases`
/// or the folder md2html runs in ([`text::inside`]), as for `--embed`.
///
/// # Errors
///
/// Returns `<path> is not an image an e-reader shows` when its extension is not
/// one [`Media`] knows, `<path> is outside the page's folder` when it is not
/// names only, or when a folder of `bases` has it but it resolves outside
/// them (and none has it inside), `<path> cannot be read: <reason>` when a
/// folder of `bases` has the file but it cannot be read (and none has a
/// readable one), and `<path> was not found` when no folder has it, or when
/// the address decodes to bytes that are not UTF-8.
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
    let outside = || format!("{path} is outside the page's folder");
    if system::names_below(Path::new(&path)).is_none() {
        return Err(outside());
    }
    // The first folder with the file inside the page's folders gives it; a
    // file that is there but cannot be read is named as such, not as
    // missing, and one that leads outside as outside.
    let mut unread = None;
    let mut led_out = false;
    for base in bases {
        let full = base.join(&path);
        if full.exists() && !text::inside(&full, bases) {
            led_out = true;
            continue;
        }
        match std::fs::read(&full) {
            Ok(bytes) => return Ok(Picture { media, bytes }),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                unread.get_or_insert(error);
            }
        }
    }
    Err(match unread {
        Some(error) => format!("{path} cannot be read: {error}"),
        None if led_out => outside(),
        None => format!("{path} was not found"),
    })
}

#[cfg(test)]
mod tests {
    use super::{Media, load, pack};

    /// A percent-encoded `data:` image keeps its bytes exactly, even those that
    /// are not UTF-8.
    ///
    /// # Panics
    ///
    /// Panics when the image is not packed as a PNG, or when its bytes differ
    /// from the eleven written, as they would if `%ff`, `%fe` or `%00` became
    /// replacement characters.
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

    /// A file address with a broken escape before a wide character is refused,
    /// not read in half.
    ///
    /// # Panics
    ///
    /// Panics when any of six addresses (`%` before `é` or an emoji, a `%` at
    /// the end, `%%%`, `%+f`) is loaded as an image instead of giving an error;
    /// a panic inside the escape reader fails it too.
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

    /// An address whose escapes decode to bytes that are not UTF-8 names no
    /// file.
    ///
    /// # Panics
    ///
    /// Panics when the file named by the escaped `U+FFFD` cannot be loaded,
    /// when `%ff.png`, `%C3.png` or `%80%80.png` loads that same file (as it
    /// would if bad bytes became replacement characters) or fails with another
    /// message than "was not found", or when the test's folder in the temporary
    /// directory cannot be made, written or removed.
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

    /// An image that is there but cannot be read is named as such, not as
    /// missing; a later folder that has a readable one still gives it.
    ///
    /// # Panics
    ///
    /// Panics when a folder named `pic.png` is reported as not found, or as
    /// anything but `pic.png cannot be read: <reason>`; when the readable copy
    /// in the second folder is not loaded; or when the test's folders in the
    /// temporary directory cannot be made, written or removed.
    #[test]
    fn an_image_that_cannot_be_read_is_named_as_such() {
        let dir =
            std::env::temp_dir().join(format!("md2html-images-unread-{}", std::process::id()));
        let (first, second) = (dir.join("first"), dir.join("second"));
        std::fs::create_dir_all(first.join("pic.png")).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let refused = load("pic.png", &[first.as_path()])
            .err()
            .unwrap_or_default();
        assert!(refused.starts_with("pic.png cannot be read: "), "{refused}");
        std::fs::write(second.join("pic.png"), b"\x89PNG").unwrap();
        let bases = [first.as_path(), second.as_path()];
        assert_eq!(load("pic.png", &bases).unwrap().bytes, b"\x89PNG");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A fresh folder for one test, with `page` inside it and `outside.png`
    /// beside it.
    ///
    /// # Panics
    ///
    /// When the folder of an earlier run cannot be removed, or the folders or
    /// the image cannot be made.
    fn beside(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("md2html-images-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(dir.join("page")).unwrap();
        std::fs::write(dir.join("outside.png"), b"\x89PNG").unwrap();
        dir
    }

    /// An address that leaves the page's folder, by its root or by `..`, is
    /// not read, on every system.
    ///
    /// # Panics
    ///
    /// When `../outside.png`, `page/../../outside.png` or the absolute path of
    /// the image is packed, or refused with another message; or when the
    /// scratch folder cannot be made or removed.
    #[test]
    fn an_image_outside_the_folder_is_not_packed() {
        let dir = beside("outside");
        let page = dir.join("page");
        let absolute = dir.join("outside.png").display().to_string();
        for src in [
            "../outside.png",
            "page/../../outside.png",
            absolute.as_str(),
        ] {
            assert_eq!(
                load(src, &[page.as_path()]).err(),
                Some(format!("{src} is outside the page's folder")),
                "{src}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A link in the page's folder to an image outside it is not followed.
    ///
    /// # Panics
    ///
    /// When the image behind the link is packed, or refused with another
    /// message than that it is outside; or when the link or the scratch
    /// folder cannot be made or removed.
    #[cfg(unix)]
    #[test]
    fn on_unix_a_link_out_of_the_folder_is_not_followed() {
        let dir = beside("link");
        let page = dir.join("page");
        std::os::unix::fs::symlink(dir.join("outside.png"), page.join("in.png")).unwrap();
        assert_eq!(
            load("in.png", &[page.as_path()]).err().as_deref(),
            Some("in.png is outside the page's folder")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// On Windows a drive's root (`\x.png`, `/x.png`), a drive's current
    /// folder (`C:x.png`) and a share lead off the page's folder.
    ///
    /// # Panics
    ///
    /// When any of them is read instead of refused as outside the folder.
    #[cfg(windows)]
    #[test]
    fn on_windows_a_drive_or_its_root_is_outside_the_folder() {
        let here = std::env::current_dir().unwrap();
        for src in [r"\x.png", "/x.png", "C:x.png", r"\\server\share\x.png"] {
            assert_eq!(
                load(src, &[here.as_path()]).err(),
                Some(format!("{src} is outside the page's folder")),
                "{src}"
            );
        }
    }

    /// An image used three times is packed once, and every use and link points
    /// at the copy.
    ///
    /// # Panics
    ///
    /// Panics when the same `data:` image is packed more than once or not at
    /// all, when the image on the web and the BMP are not listed as left
    /// out, or when the links and sources are not rewritten to
    /// `images/img-1.gif` while the web and BMP images give way to their
    /// (empty) text.
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

    /// An image on the web, or one that cannot be read, gives way to its text
    /// alternative.
    ///
    /// # Panics
    ///
    /// Panics when such an image stays in the article, when its text
    /// alternative is not in its place, when its address is not listed as left
    /// out, or when the reasons are not reported once each.
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
