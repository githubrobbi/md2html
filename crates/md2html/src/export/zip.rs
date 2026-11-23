// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A zip archive of stored (uncompressed) entries, as EPUB allows; the
//! first entry, `mimetype`, must be stored.

/// The signature of an entry's local header.
const LOCAL: u32 = 0x0403_4B50;
/// The signature of an entry's record in the central directory.
const CENTRAL: u32 = 0x0201_4B50;
/// The signature of the record that closes the archive.
const END: u32 = 0x0605_4B50;
/// The length of a local header without its name.
const LOCAL_LEN: u32 = 30;
/// The length of a central directory record without its name.
const CENTRAL_LEN: u32 = 46;
/// The zip version needed to read a stored entry (1.0).
const NEEDED: u16 = 10;
/// The zip version the entries are written by (2.0).
const MADE_BY: u16 = 20;
/// The flag that says an entry's name is UTF-8.
const UTF8_NAME: u16 = 0x0800;
/// The method of an entry kept as it is.
const STORED: u16 = 0;
/// Every entry's time, 00:00, and date, 1980-01-01: the same book from
/// the same page, byte for byte.
const DOS_TIME: u16 = 0;
/// See [`DOS_TIME`].
const DOS_DATE: u16 = 0x21;

/// A zip archive being written.
#[derive(Default)]
pub(super) struct Zip {
    /// The entries so far, each its local header and its data.
    entries: Vec<u8>,
    /// The central directory so far.
    central: Vec<u8>,
    /// How many entries there are.
    count: u16,
    /// How long `entries` is: where the next entry, and at the end the
    /// central directory, starts.
    offset: u32,
    /// How long `central` is.
    central_len: u32,
}

impl Zip {
    /// Adds a file. A file the archive cannot hold (a zip without its
    /// 64-bit extension counts in 32 bits, its entries in 16) is not
    /// added, and the error says why.
    ///
    /// # Errors
    ///
    /// Returns "it does not fit in a zip archive" when the data is 4 GiB or
    /// longer, the name 64 KiB or longer, or the entries or the central
    /// directory would grow past what 32 bits count; and "a zip archive holds
    /// no more files" when it already holds 65,535. The archive is left as it
    /// was.
    pub(super) fn add(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let too_large = || String::from("it does not fit in a zip archive");
        let (Ok(size), Ok(name_len)) = (u32::try_from(data.len()), u16::try_from(name.len()))
        else {
            return Err(too_large());
        };
        let count = self
            .count
            .checked_add(1)
            .ok_or("a zip archive holds no more files")?;
        let offset = LOCAL_LEN
            .checked_add(u32::from(name_len))
            .and_then(|header| header.checked_add(size))
            .and_then(|entry| self.offset.checked_add(entry))
            .ok_or_else(too_large)?;
        let central_len = CENTRAL_LEN
            .checked_add(u32::from(name_len))
            .and_then(|record| self.central_len.checked_add(record))
            .ok_or_else(too_large)?;
        // The closing record says where the directory ends.
        offset.checked_add(central_len).ok_or_else(too_large)?;
        let crc = crc32(data);
        let flags = if name.is_ascii() { 0 } else { UTF8_NAME };
        let local = &mut self.entries;
        put32(local, &[LOCAL]);
        put16(local, &[NEEDED, flags, STORED, DOS_TIME, DOS_DATE]);
        put32(local, &[crc, size, size]);
        put16(local, &[name_len, 0]);
        local.extend_from_slice(name.as_bytes());
        local.extend_from_slice(data);
        let record = &mut self.central;
        put32(record, &[CENTRAL]);
        put16(record, &[
            MADE_BY, NEEDED, flags, STORED, DOS_TIME, DOS_DATE,
        ]);
        put32(record, &[crc, size, size]);
        // The name's length; no extra field, comment, disk number or
        // internal attributes.
        put16(record, &[name_len, 0, 0, 0, 0]);
        // No external attributes; where the entry starts.
        put32(record, &[0, self.offset]);
        record.extend_from_slice(name.as_bytes());
        self.count = count;
        self.offset = offset;
        self.central_len = central_len;
        Ok(())
    }

    /// The archive, closed.
    pub(super) fn finish(mut self) -> Vec<u8> {
        let mut archive = self.entries;
        archive.append(&mut self.central);
        put32(&mut archive, &[END]);
        // One disk, the first, holding every entry.
        put16(&mut archive, &[0, 0, self.count, self.count]);
        put32(&mut archive, &[self.central_len, self.offset]);
        // No comment.
        put16(&mut archive, &[0]);
        archive
    }
}

/// Appends 16-bit fields, little-endian as zip writes them.
fn put16(out: &mut Vec<u8>, fields: &[u16]) {
    out.extend(fields.iter().flat_map(|field| field.to_le_bytes()));
}

/// Appends 32-bit fields, little-endian as zip writes them.
fn put32(out: &mut Vec<u8>, fields: &[u32]) {
    out.extend(fields.iter().flat_map(|field| field.to_le_bytes()));
}

/// The CRC-32 a zip records for each file.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0_u8..8_u8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1_u32) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::{Zip, crc32};

    /// The CRC-32 is the one zip uses.
    ///
    /// # Panics
    ///
    /// Panics when the CRC-32 of `123456789` is not `0xCBF43926`, the check
    /// value of the CRC zip and gzip share.
    #[test]
    fn crc_of_a_known_text() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    /// The first entry, `mimetype`, is stored where an e-reader looks for it.
    ///
    /// # Panics
    ///
    /// Panics when the archive does not start with a local header, or when the
    /// name `mimetype` and its text `application/epub+zip` are not at bytes 30
    /// and 38, stored and uncompressed.
    #[test]
    fn a_zip_starts_with_the_stored_mimetype() {
        let mut zip = Zip::default();
        zip.add("mimetype", b"application/epub+zip").unwrap();
        let bytes = zip.finish();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        assert_eq!(bytes.get(30..38), Some(&b"mimetype"[..]));
        assert_eq!(bytes.get(38..58), Some(&b"application/epub+zip"[..]));
    }

    /// The closing record counts the entries and says where the directory is
    /// and how long.
    ///
    /// # Panics
    ///
    /// Panics when the last 22 bytes are not the closing record of two entries
    /// with a directory of 96 bytes starting at 66, or when the archive is not
    /// exactly the entries, the directory and that record; a name with `é`
    /// counts its two UTF-8 bytes.
    #[test]
    fn the_closing_record_counts_and_finds_the_directory() {
        let mut zip = Zip::default();
        zip.add("a", b"xy").unwrap();
        zip.add("b\u{e9}", b"").unwrap();
        let bytes = zip.finish();
        // Entries: 30 + 1 + 2 and 30 + 3; directory: 46 + 1 and 46 + 3.
        let (_, end) = bytes.split_last_chunk::<22>().unwrap();
        assert_eq!(end, &[
            0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0, 2, 0, 2, 0, 96, 0, 0, 0, 66, 0, 0, 0, 0, 0
        ]);
        assert_eq!(bytes.len(), 66 + 96 + 22);
    }
}
