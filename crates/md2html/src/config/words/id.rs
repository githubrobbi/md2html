// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `id`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `id`.
pub(super) static WORDS: Ui = Ui {
    skip: "Langsung ke isi",
    search: "Cari",
    search_page: "Cari di halaman ini",
    keys: SearchKeys {
        step: "pindah",
        open: "buka",
        close: "tutup",
    },
    none: "Tidak ada yang cocok di halaman ini.",
    results: Template {
        lead: "",
        tails: [" hasil"],
    },
    result: "1 hasil",
    glossary: "Istilah di halaman ini",
    glossary_intro: "Setiap kata bergaris titik di dalam bab menampilkan artinya saat penunjuk berada di atasnya, dan melompat ke sini saat diklik. Setiap entri memiliki tombol <strong>kembali</strong> ke baris yang sedang Anda baca.",
    glossary_shared: Template {
        lead: " Singkatan diambil dari <a href=\"",
        tails: [
            "\">glosarium bersama</a>; <strong>semua istilah</strong> membukanya pada entri tersebut.",
        ],
    },
    back: "kembali",
    all_terms: "semua istilah",
    keep_outline: "&lsaquo; biarkan kerangka tetap terbuka",
    outline: "Kerangka",
    outline_tips: OutlineTips {
        hide: "Sembunyikan kerangka",
        show: "Tampilkan kerangka",
        keys: " (⌘\\, Control+\\ di Windows)",
    },
    notes: "Catatan",
    note_n: Template {
        lead: "Catatan ",
        tails: [""],
    },
    link_section: "Tautan ke bagian ini",
    copy: CopyButton {
        label: "salin",
        copied: "tersalin",
    },
    callouts: Callouts {
        note: "Catatan",
        tip: "Tips",
        important: "Penting",
        warning: "Peringatan",
        caution: "Perhatian",
    },
    footer: Template {
        lead: "Dibuat dari <code>",
        tails: [
            "</code> oleh <code>md2html</code>. Berkas Markdown adalah sumbernya; buat ulang setelah menyuntingnya.",
        ],
    },
    months: [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ],
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Bagian atas halaman",
    figure: EnlargedFigure {
        label: "Gambar yang diperbesar",
        close: "Tutup",
        close_key: "Tutup (Escape)",
        position: Template {
            lead: "",
            tails: [" dari ", ""],
        },
        enlarge: "Perbesar diagram",
    },
    size: TextSize {
        group: "Ukuran teks",
        smaller: SizeStep {
            words: "Teks lebih kecil",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Teks lebih besar",
            joint: ", ",
            unit: "%",
        },
        as_written: "Ukuran teks: 100%, halaman seperti yang ditulis.",
        scaled: Template {
            lead: "Ukuran teks: ",
            tails: ["%. Klik untuk kembali ke 100%."],
        },
        keys: " Command (Control di Windows) dengan + atau - mengubahnya di sini, 0 mengatur ulang; zoom peramban sendiri tidak diubah.",
        smallest: "Teks sudah paling kecil",
        largest: "Teks sudah paling besar",
    },
    theme: ThemeSwitch {
        button: "Tema",
        tip: Template {
            lead: "Tema: ",
            tails: [". Klik untuk beralih ke ", ", lalu ke ", "."],
        },
        system_now: Template {
            lead: "Sistem, saat ini ",
            tails: [""],
        },
        system: "Sistem",
        light: "Terang",
        dark: "Gelap",
        dark_mode: "gelap",
        light_mode: "terang",
    },
    table: "Tabel",
    pages: PageLinks {
        previous: "Sebelumnya",
        next: "Berikutnya",
        label: "Halaman",
    },
    task: TaskBoxes {
        done: "Selesai",
        open: "Belum",
    },
    guide_heading: "Cara membaca halaman ini",
    glossary_heading: "Glosarium",
    meaning_heads: &["Arti", "Makna", "Definisi", "Keterangan"],
    contents_heading: "Daftar isi",
    lang: LanguageSwitch {
        label: "Bahasa",
        original: "Asli",
        notice: Template {
            lead: "Ini adalah terjemahan. Versi yang berlaku adalah aslinya (",
            tails: [")."],
        },
        on_device: "Di perangkat ini",
        machine: "Terjemahan mesin oleh peramban ini, di perangkat ini: tidak ada yang keluar darinya.",
        show_original: "Tampilkan aslinya",
        translating: "Menerjemahkan…",
        cannot: Template {
            lead: "Peramban ini tidak dapat menerjemahkan ke ",
            tails: ["."],
        },
        more: "Bahasa lainnya",
    },
    export: ExportMenu {
        label: "Simpan atau cetak",
        html: "Halaman ini, untuk dibaca offline (HTML)",
        print: "Cetak, atau simpan sebagai PDF",
        pdf: "PDF",
        epub: "EPUB, untuk pembaca e-book",
    },
    prefs: "Pengaturan baca",
    credit: Template {
        lead: "Dibuat dengan ",
        tails: [""],
    },
    share: Sharing {
        copied: "Tautan disalin",
        passage: "Tautan ke bagian teks ini",
        row: "Tautan ke baris ini",
        thing: "Tautan ke sini",
    },
    live: "Diperbarui ke versi terbaru",
};
