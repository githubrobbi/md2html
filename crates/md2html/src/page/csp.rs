// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A Content-Security-Policy that allows exactly a page's own inline
//! styles and scripts, by their hashes, and no connection out.

use crate::scan::Scan;
use crate::text::{base64, sha256};

/// What a policy lets run: the hashes of the page's scripts and styles.
#[derive(Default)]
struct Hashes {
    /// One per different script that runs.
    scripts: Vec<String>,
    /// One per different style.
    styles: Vec<String>,
}

impl Hashes {
    /// The hashes of the scripts and styles in `html`. A script that is
    /// data (JSON) runs nothing and needs none. An element left open at
    /// the end ends the reading.
    fn of(html: &str) -> Self {
        let mut hashes = Self::default();
        let mut scan = Scan::new(html);
        while scan.take_until("<").is_some() {
            let (list, close) = if scan.starts_with("<script") {
                (&mut hashes.scripts, "</script>")
            } else if scan.starts_with("<style") {
                (&mut hashes.styles, "</style>")
            } else {
                scan.bump();
                continue;
            };
            let Some(tag) = scan.take_until(">") else {
                break;
            };
            scan.bump();
            let Some(body) = scan.take_until(close) else {
                break;
            };
            scan.eat_str(close);
            let data_only = tag.contains("application/json") || tag.contains("application/ld+json");
            if !data_only {
                list.push(format!("'sha256-{}'", base64(&sha256(body.as_bytes()))));
            }
        }
        hashes.scripts.dedup();
        hashes.styles.dedup();
        hashes
    }
}

/// The policy's `<meta>` for a page whose scripts and styles are all in
/// `html`.
///
/// `inline_styles` allows any inline style (mermaid.js writes its own into
/// each drawing); `live` lets the page ask its own server whether it
/// changed.
pub(super) fn policy_tag(html: &str, inline_styles: bool, live: bool) -> String {
    let hashes = Hashes::of(html);
    let scripts = if hashes.scripts.is_empty() {
        String::from("'none'")
    } else {
        hashes.scripts.join(" ")
    };
    let styles = if inline_styles {
        String::from("'unsafe-inline'")
    } else if hashes.styles.is_empty() {
        String::from("'none'")
    } else {
        hashes.styles.join(" ")
    };
    let connect = if live { "'self'" } else { "'none'" };
    format!(
        "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src {scripts}; style-src {styles}; \
img-src * data: blob:; media-src * data: blob:; font-src data:; \
connect-src {connect}; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'\">"
    )
}
