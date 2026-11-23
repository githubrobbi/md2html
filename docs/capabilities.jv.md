---
title: md2html, kabèh sing bisa dilakoni
description: md2html ngowahi Markdown dadi siji kaca wèb sing jangkep lan mandhiri, kanthi kerangka, panggolèkan, téma, glosarium, diagram, matématika, pitulas basa antarmuka, salinan PDF lan EPUB, lan kaca sing tansah nganyaraké awaké dhéwé. Kaca iki nuduhaké kabèh mau, lan digawé nganggo md2html.
brand: md2html
kicker: Kabisan
version: 0.7.7
date: 2026-11-23
footer: Ditulis nganggo Markdown lan dirender déning **md2html 0.7.7** dhéwé, saben gambar layar, diagram lan rumus ana ing jero berkas siji iki.
csp: true
embed: true
translated_from: capabilities.md
source_sha256: 493521b114c647756b2e3294586c8d7c59ff8a3fd7364672e88fb27371cfc342
---

# md2html, kabèh sing bisa dilakoni

md2html ngowahi berkas Markdown dadi siji kaca wèb sing ora butuh apa-apa manèh: tanpa server, tanpa jaringan, tanpa font utawa skrip saka ngendi waé. Kaca kuwi nggawa dhéwé kerangka, panggolèkan, téma padhang lan peteng, ukuran teks, glosarium, cathetan sikil, gambar sing bisa digedhèkaké, kode sing diwarnani, matématika sing disusun rapi lan diagram. Kaca iki ditulis nganggo Markdown lan dirender déning md2html, dadi kabèh sing diandharaké makarya ing kéné nalika panjenengan maos.

## Carané maca kaca iki

- **Bab 1** yaiku ing ngendi md2html mlaku, lan carané masang.
- **Bab 2** yaiku kabèh ing siji tabel.
- **Bab 3 nganti 9** nuduhaké apa sing ditampa pamaos, lan apa sing diketik panyerat supaya olèh iku.
- **Bab 10 nganti 12** yaiku setèlan, baris printah, lan keamanan lan mutu.

---

## 1. Ing ngendi md2html mlaku

| | Ing ngendi |
|:--|:--|
| Piranti md2html | macOS (Apple silicon lan Intel), Linux lan Windows, saka siji program Rust tanpa dependensi kajaba pustaka standar; diuji ing telu-teluné ing saben owah-owahan |
| Kaca sing digawé | Chrome, Edge, Firefox lan Safari vèrsi saiki, ing komputer, tablet lan HP, online utawa offline; pamaca layar lan papan ketik ing kabèh bagéan |
| Dibukak saka | server wèb, folder bebarengan, lampiran email, utawa langsung saka disk: kaca kuwi mung siji berkas |
| Dicithak utawa disimpen dadi PDF | saka peramban apa waé, nganggo tata letak cithakané dhéwé; `--pdf` nggawé PDF-é dhéwé, nganggo Chrome, Chromium utawa Edge sing wis ana ing komputer |
| E-reader | `--epub` nggawé EPUB 3, format standar buku éléktronik |
| Basa kaca | pitulas, kalebu sing saka tengen menyang kiwa (bab 8) |

Pasang nganggo panginstal Rust dhéwé, saka kode sumberé:

```console
$ cargo install --git https://github.com/githubrobbi/md2html md2html --locked
$ md2html notes.md
md2html: notes.md -> notes.html (68504 bytes, 9 glossary terms, 0 of them shared)
```

## 2. Sakilas

| | md2html |
|:--|:--|
| Markdown | kabèh CommonMark 0.31.2 (**640 saka 652** conto ujiné lulus), lan tabel, daftar tugas, coretan, pranala otomatis, cathetan sikil lan alert saka GitHub |
| Asil | **siji berkas HTML**, ora ana sing dijupuk saka njaba, ora ana sing kudu dipasang pamaos |
| Maos | kerangka, panggolèkan, téma padhang lan peteng, ukuran teks, glosarium, cathetan sikil, gambar sing bisa digedhèkaké |
| Kode, matématika, diagram | kode diwarnani ing kira-kira rong puluh basa, matématika minangka MathML, diagram alur lan diagram urutan Mermaid digambar minangka SVG, **kabèh tanpa pustaka skrip** |
| Mbagi | pranala menyang bab, larik tabel, gambar, diagram, blok kode, utawa pethikan sing dipilih |
| Basa | tembung-tembungé kaca dhéwé ing **17 basa**, terjemahan digandhèng nganggo saklar, saka tengen menyang kiwa kanggo Ibrani lan Yiddish |
| Nyimpen | kaca kuwi dhéwé, **PDF** nganggo tetenger, **EPUB** kanggo e-reader |
| Kaca urip | kaca sing dimuat manèh yèn Markdown-é owah lan njaga panggonané pamaos |
| Keamanan | ora tau ana skrip saka teks; ora ana berkas sing diwaca ing njaba folderé dokumen |
| Aksesibilitas | ora ana temuan tumrap WCAG 2.2 AA, ing loro téma lan ing ambané layar HP |
| Kacepetan | dokumen 4.000 larik, kalebu diagram lan matématika, ing **kurang saka saprasepuluh detik** |
| Dependensi | **ora ana**: pustaka standar Rust lan md2html |

## 3. Kaca

Kaca iki ana telung bagéan: bilah ing sadawané pucuk, kerangka ing sisih kiwa, lan teks.

![Siji kaca ing téma padhang: kerangka nandhani bab sing lagi diwaca lan bilah nyebut jenengé.](capabilities/light.jpg){width=100%}

Téma peteng manut sistem, utawa pilihané pamaos, lan pilihan iku dieling-eling:

![Kaca sing padha ing téma peteng, ing bagéan kode lan matématika.](capabilities/dark-code.jpg){width=100%}

Ing HP, kerangka lempit ing mburiné saklar ing bilah, lan ora ana sing gulung mangiwa-manengen:

![Kaca ing ambané layar HP.](capabilities/phone.jpg){width=34%}

Sing bisa ditindakaké pamaos:

| Kanggo | Carané |
|:--|:--|
| Nggolèki samubarang | **Golèk** ing bilah, <kbd>⌘</kbd> <kbd>K</kbd> (<kbd>Ctrl</kbd> <kbd>K</kbd>), utawa <kbd>/</kbd> |
| Ndhelikaké utawa nuduhaké kerangka | saklar ing sisih kiwa bilah, utawa <kbd>⌘</kbd> <kbd>\\</kbd> |
| Ngowahi ukuran teks | **Aa** ing bilah, banjur **−** lan **+**; utawa <kbd>⌘</kbd> <kbd>+</kbd> lan <kbd>⌘</kbd> <kbd>−</kbd>, saka 85 nganti 200 persen |
| Ngganti téma | **Aa**, banjur Sistem, Padhang utawa Peteng |
| Nggedhèkaké gambar | klik gambaré; rong klik manèh nggawa luwih cedhak; <kbd>Esc</kbd> mbalèkaké |
| Maos cathetan sikil utawa tembung sing digaris titik | dèlèhaké penunjuk ing ndhuwuré; klik kanggo mrana, lan **bali** nggawa panjenengan wangsul |
| Mbagi bab, larik tabel, gambar, diagram, blok kode, utawa pethikan | simbol bagi ing sisih tengen irah-irahan, ing pungkasan larik tabel ing sangisoré penunjuk, utawa ing pojok gambar, diagram, utawa blok kode ing sangisoré penunjuk, nyalin pranalané (HP mbukak menu baginé); pilih sawetara tembung, lan pranala persis menyang tembung-tembung kuwi bakal ditawakaké |
| Nyimpen | tombol undhuh ing pungkasané bilah: kaca kuwi dhéwé (bisa diwaca tanpa internet), cithak utawa simpen dadi PDF, utawa PDF lan EPUB yèn wis digawé |

Ukuran teks lan téma ana bebarengan ing mburiné **Aa**; nyimpen kaca duwé tombolé dhéwé:

![Aa: ukuran teks lan téma.](capabilities/panel.jpg){width=60%}

## 4. Nulis

Markdown biasa, lan saben wanguné uga kepénak diwaca minangka teks polos. Sing panjenengan serat ana ing ngarep, ing blok kode; asilé ngetutaké ing sawisé.

### Penegasan lan pranala

```markdown
**Kandel**, *miring*, ***loro-loroné***, ~~dicorèt~~ lan `kode`.
Siji [pranala](https://commonmark.org "CommonMark"), siji sing gundhul,
https://spec.commonmark.org/0.31.2/, lan tandha kaya [V] sing tetep kaya ditulis.
```

**Kandel**, *miring*, ***loro-loroné***, ~~dicorèt~~ lan `kode`. Siji [pranala](https://commonmark.org "CommonMark"), siji sing gundhul, https://spec.commonmark.org/0.31.2/, lan tandha kaya [V] sing tetep kaya ditulis.

### Daftar ing jero daftar

```markdown
1. Kapisan, kanthi daftar ing jeroné:
   - siji butir
   - siji manèh, kanthi daftaré dhéwé:
     1. luwih jero
2. Kapindho.
- [x] tugas sing wis rampung
- [ ] tugas sing durung rampung
```

1. Kapisan, kanthi daftar ing jeroné:
   - siji butir
   - siji manèh, kanthi daftaré dhéwé:
     1. luwih jero
2. Kapindho.

- [x] tugas sing wis rampung
- [ ] tugas sing durung rampung

### Kutipan lan kothak katrangan

```markdown
> Kutipan biasa tetep dadi kutipan.

> [!TIP]
> Alert GitHub bisa mlaku: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Cathetan:** semono uga wangun sing luwih lawas, siji tembung kandel lan titik loro.
```

> Kutipan biasa tetep dadi kutipan.

> [!TIP]
> Alert GitHub bisa mlaku: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Cathetan:** semono uga wangun sing luwih lawas, siji tembung kandel lan titik loro.

### Tabel

```markdown
| Ruangan    | Tingkat | Jembar (kaki persegi) |
|:-----------|:-------:|----------------------:|
| Ruang tamu | ngisor  |                   312 |
| Loteng     | ndhuwur |                   205 |
```

| Ruangan | Tingkat | Jembar (kaki persegi) |
|:-------|:------:|-------------:|
| Ruang tamu | ngisor | 312 |
| Loteng | ndhuwur | 205 |

Tabel sing luwih amba tinimbang kolomé gulung ing jero kolom kuwi, lan kacané ora tau mèlu gulung.

### Cathetan sikil

```markdown
Pratélan sing butuh sumber.[^1]

[^1]: Sumberé. Dèlèhaké penunjuk ing nomeré kanggo maos ing kono.
```

Pratélan sing butuh sumber.[^source]

[^source]: Sumberé. Dèlèhaké penunjuk ing nomeré kanggo maos ing kono; klik nomeré kanggo teka mréné, lan bali kanggo wangsul.

### Gambar lan citra

```markdown
![Katrangan ing ngisor gambar.](capabilities/zoom.jpg){width=60%}

Siji ikon ![tandhané](mark.svg){width=18} ing tengah ukara.
```

![Gambar 60 persen saka ambané kolom. Klik gambaré: gambar ngebaki jendhéla, lan rong klik manèh nggawa luwih cedhak.](capabilities/zoom.jpg){width=60%}

Siji ikon ![tandhané](mark.svg){width=18} ing tengah ukara.

### Prekara-prekara cilik

```markdown
Pencet <kbd>Ctrl</kbd>+<kbd>K</kbd>. Banyu iku H<sub>2</sub>O, kamaré 9 m<sup>2</sup>.
Larik sing dipungkasi garis miring walik\
pedhot ing kono.
```

Pencet <kbd>Ctrl</kbd>+<kbd>K</kbd>. Banyu iku H<sub>2</sub>O, kamaré 9 m<sup>2</sup>. Larik sing dipungkasi garis miring walik\
pedhot ing kono.

<details>
<summary>Bagéan sing kabukak kanthi sepisan klik</summary>

Ditulis minangka `<details>` karo `<summary>`, isiné Markdown biasa: **kandel**, siji daftar,

- siji
- loro

</details>

#### Irah-irahan tingkat papat

Irah-irahan ing ngisor tingkat telu ora mlebu kerangka, kaya irah-irahan iki.

## 5. Kode, matématika lan diagram

Kode diwarnani kanggo Rust, Python, JavaScript lan TypeScript, Go, C lan C++, Java, Kotlin, Swift, C#, shell, SQL, Ruby, Lua, PHP, CSS, HTML, JSON, YAML, TOML, Dockerfile lan diff, lan saben blok duwé tombol salin.

```rust
/// The whole page from one Markdown text.
fn main() {
    let page = md2html::render("# Hello\n\nWorld.", &Settings::default());
    println!("{} bytes, nothing to fetch", page.len()); // one file
}
```

```python
@cache
def area(w: float, h: float) -> float:
    """The area of a room."""
    return w * h  # square feet
```

```diff
- every quotation a note
+ a quotation is a quotation; a bold first word makes it a note
```

Matématika disusun déning peramban dhéwé, minangka MathML, tanpa skrip lan tanpa font sing kudu diundhuh. Ing jero larik, téoréma Pythagoras yaiku $a^2 + b^2 = c^2$, lan idhèntitas Euler yaiku $e^{i\pi} + 1 = 0$; cacahing dhuwit kaya $5 utawa $10 tetep dadi dhuwit. Ditampilaké dhéwé:

$$
\int_0^1 x^2 \, dx = \frac{1}{3} \qquad \sum_{k=1}^{n} k = \frac{n(n+1)}{2} \qquad \sqrt[3]{27} = 3
$$

$$
\begin{pmatrix} a & b \\ c & d \end{pmatrix}
\begin{pmatrix} x \\ y \end{pmatrix}
=
\begin{pmatrix} ax + by \\ cx + dy \end{pmatrix}
\qquad
|x| = \begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases}
$$

### Diagram

Diagram alur utawa diagram urutan Mermaid digambar déning md2html dhéwé, minangka SVG ing jero kaca: tanpa skrip, ora ana sing kudu diundhuh, nganggo werna kaca dhéwé ing loro téma, lan tembung-tembungé tetep tembung sing bisa ditemokaké panggolèkan. Iki Markdown-é:

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[Waca Markdown]
  parse --> render{Diagram?}
  render -->|diagram alur utawa urutan| svg[Gambar dadi SVG]
  render -->|jinis liyané| js[mermaid.js, yèn diwènèhi]
  svg --> page([siji kaca HTML])
  js --> page
```
````

Lan iki dadiné:

```mermaid
flowchart LR
  md[notes.md] --> parse[Waca Markdown]
  parse --> render{Diagram?}
  render -->|diagram alur utawa urutan| svg[Gambar dadi SVG]
  render -->|jinis liyané| js[mermaid.js, yèn diwènèhi]
  svg --> page([siji kaca HTML])
  js --> page
```

Diagram urutan, nganggo nomer, kanthi cathetan:

```mermaid
sequenceDiagram
  autonumber
  actor R as Pamaos
  participant P as Kaca
  participant S as Panggolèkan
  R->>P: Cmd K
  P->>S: bukak panggolèkan
  R->>S: ketik "katrangan"
  S-->>R: pethikan-pethikan, sing paling apik dhisik
  R->>S: Enter
  S-->>P: mlumpat, lan sorot tembung-tembungé
  Note over R,P: tanpa jaringan, tanpa server
```

Jinis liyané (diagram pai, diagram Gantt, diagram kelas) digambar déning mermaid.js yèn panyerat nuduhaké salinané marang md2html (`--mermaid-js FILE`); skrip iku banjur mung mlebu ing kaca kasebut. Tanpa iku, diagram-diagram kuwi tetep dadi kode, lan `--check` ngandhakaké.

## 6. Glosarium, cathetan sikil, panggolèkan lan gambar

Bab sing jenengé Glosarium nyimpen tembung-tembungé kaca minangka tabel. Bab iku ora dicithak ing panggonané: minangka gantiné, sebutan kapisan saben istilah ing saben bab dadi tembung sing digaris titik sing nerangaké awaké dhéwé yèn penunjuk ana ing ndhuwuré, lan yèn diklik mlumpat menyang entriné ing ngisor, sing nggawa panjenengan wangsul menyang larik sing lagi panjenengan waos.

![Nyèlèhaké penunjuk ing tembung sing digaris titik nuduhaké tegesé.](capabilities/glossary.jpg){width=70%}

Cathetan sikil makarya kanthi cara sing padha: nomeré nerangaké awaké dhéwé yèn penunjuk ana ing ndhuwuré.

![Nyèlèhaké penunjuk ing nomer cathetan sikil nuduhaké cathetané.](capabilities/footnote.jpg){width=70%}

> [!NOTE]
> Glosarium bisa dienggo bebarengan déning akèh kaca: siji berkas ing `~/.config/md2html/glossary.md`, sing tembung-tembungé banjur dadi pranala ing saben kaca sing dirender ing komputer kuwi.

Panggolèkan nyakup saben tembung ing kaca, kalebu glosarium. Asilé nuduhaké ing ngendi saben tembung ana; sing dipilih disorot ing panggonan panggolèkan mandheg.

![Nggolèki "callout": bab lan pethikan saka saben asil.](capabilities/search.jpg){width=80%}

![Panggonan panggolèkan mandheg: entriné sumunar lan tembung sing ditemokaké disorot.](capabilities/search-landed.jpg){width=80%}

Gambar digedhèkaké ing ndhuwur kaca tanpa ninggal kaca. Yèn luwih gedhé tinimbang jendhéla, gambar ngetutaké penunjuk, dadi panjenengan cukup nuding apa sing kersa panjenengan pirsani.

## 7. Mbagi lan kaca urip

Saben irah-irahan nggawa simbol bagi ing sisih tengené; saben larik tabel, gambar, diagram lan blok kode nuduhaké simbol kuwi yèn ana ing sangisoré penunjuk. Sepisan klik nyalin pranala persis menyang panggonan kuwi, lan ngandhakaké; ing HP, menu bagi saka sistem sing kabukak. Pilih sawetara tembung, lan kaca nawakaké pranala persis menyang tembung-tembung kuwi, sing mbukak kaca ing pethikan kuwi, kanthi disorot.

![Milih tembung-tembung nawakaké pranala menyang tembung-tembung kuwi.](capabilities/share.jpg){width=80%}

Kaca uga bisa njaga awaké dhéwé tetep anyar. `md2html --serve` nuduhaké kaca-kaca ing komputer iki salawasé lagi ditulis, lan saben kaca sing kabukak dimuat manèh nalika Markdown-é disimpen. Kaca sing diterbitaké nganggo `--live` duwé umur dhéwé: yèn umuré entèk, kaca kuwi njaluk kilobyte kapisan saka awaké dhéwé marang server-é, panggonané sidik driji kaca, lan mung dimuat manèh yèn sidik driji kuwi owah. Apa waé carané, pamaos tetep ana ing panggonané: ing paragraf sing padha ing dhuwur sing padha, utawa, yèn paragraf kuwi wis ora ana, ing bab sadurungé; ngalih menyang basa liya uga njaga panggonan kanthi cara sing padha.

## 8. Basa

Tembung-tembungé kaca dhéwé (panggolèkan, kothak katrangan, glosarium, tanggal, saben tombol) ana ing pitulas basa: Inggris, Jerman, Ibrani, Indonésia, Walanda, Jawa, Yiddish, Jerman Ngisor, Prancis, Italia, Spanyol, Jepang sing ditulis nganggo telung cara (nganggo kanji lan kana, mung nganggo kana, lan nganggo rōmaji), Mandarin nganggo aksara prasaja lan nganggo aksara tradhisional, lan Kanton. Cukup nulis `lang: de` ing front matter. Ibrani lan Yiddish ditata saka tengen menyang kiwa, kerangkané ing sisih tengen.

Terjemahan iku mung berkas Markdown liya ing sandhingé asliné, dijenengi miturut basané: `capabilities.de.md` iku `capabilities.md` ing basa Jerman. md2html nemokaké vèrsi-vèrsi sing ana, nggandhèngaké siji lan sijiné nganggo saklar ing bilah (bola donya), ngabari peramban lan mesin panggolèk nganggo `hreflang`, lan mènèhi saben vèrsi tembung-tembung antarmukané dhéwé. Kaca iki ana ing kabèh pitulas basa kuwi; bola donya ngalih ing antarané, lan kaca kabukak ing paragraf sing lagi panjenengan waos.

Saben terjemahan nyebut asliné lan nyathet sidik driji asliné, dadi kaca bisa ngandhakaké vèrsi endi sing dadi pathokan, lan `--check` ngerti yèn asliné wis owah wiwit terjemahan digawé:

```yaml
translated_from: capabilities.md
source_sha256: 5b0c...e1
```

Kanggo basa sing durung ana sing nerjemahaké kaca menyang basa kuwi, peramban duwèké pamaos dhéwé bisa nerjemahaké, yèn peramban iku duwé kabisan kasebut (penerjemah gawan Chrome): ing piranti iku dhéwé, dadi ora ana apa-apa sing metu saka kono, diwènèhi label minangka terjemahan mesin, kanthi dalan bali menyang asliné.

## 9. Nyimpen kaca

Tombol undhuh ing bilah nawakaké apa sing bisa disimpen. Kaca kuwi dhéwé bisa diwaca tanpa internet yèn gambar-gambaré ana ing jeroné (`embed: true`). Cithakan olèh tata letak dhéwé: tombol-tombol ilang, saben pranala nyithak alamaté, ora ana sing kepotong ing antarané rong kaca, lan saben kaca nggawa irah-irahan lan nomeré.

![Kaca sing padha nalika dicithak.](capabilities/print.jpg){width=55%}

`--pdf` nyithak kaca nganggo Chrome sing wis ana ing komputer: tata letak cithakané dhéwé, irah-irahan bab minangka tetenger PDF, irah-irahan kaca ing pucuk saben kaca lan nomeré ing ngisor. `--epub` nggawé buku kanggo e-reader saka kaca sing padha, kanthi daftar isi, gambar, diagram lan matématikané, lan cathetan sikil sing kabukak ing panggonan sing nyebut; md2html nulis dhéwé. Pamaos nemokaké loro-loroné ing sangisoré tombol undhuh.

## 10. Setèlan

Pucuké berkas Markdown asli kaca iki, sing nganggo basa Inggris, unié:

```yaml
---
title: md2html, everything it does
brand: md2html
kicker: Capabilities
version: 0.7.7
date: 2026-11-22
footer: Written in Markdown and rendered by **md2html 0.7.7** itself ...
csp: true      # only this page's own code may run
embed: true    # the screenshots are inside the page
---
```

| Setèlan | Gunané |
|:--|:--|
| `title`, `description`, `author` | irah-irahan, lan tag meta kanggo pratinjau pranala |
| `lang` | basa: tembung-tembung lan tanggal kaca dhéwé ing salah siji saka pitulas basa (`ja-Hira` iku basa Jepang ing kana, `zh-Hant` basa Tionghoa ing aksara tradhisional, `yue` basa Kanton) |
| `translated_from`, `source_sha256` | kanggo terjemahan: asliné, lan sidik driji asliné nalika diterjemahaké |
| `brand`, `brand_link`, `kicker`, `eyebrow`, `footer` | bilah, larik ing ndhuwur irah-irahan, tulisan cilik ing ngisor |
| `date`, `version` | tanggal lan vèrsi sing katon ing ndhuwur irah-irahan |
| `embed` | gambar ing jero kaca, dadi cukup siji berkas |
| `csp` | siji Content-Security-Policy |
| `pdf`, `epub` | PDF lan EPUB saka kaca kuwi, ing sandhingé |
| `live` | umur kaca (`true` tegesé rong menit, utawa sawetara detik): sawisé kuwi kaca dimuat manèh yèn owah, lan pamaos tetep ana ing panggonané |
| `large`, `toc`, `math`, `index`, `html`, `css`, `head` | aksara luwih gedhé, kerangka, matématika, mesin panggolèk, HTML sing luwih sugih, gaya lan head tambahan |

> [!TIP]
> Nganggo `lang: de` panggolèkan muni "Diese Seite durchsuchen", kothak katrangan muni "Hinweis" lan "Achtung", lan tanggalé kawaca "22. November 2026".

## 11. Baris printah

```console
$ md2html notes.md
md2html: notes.md -> notes.html (68504 bytes, 9 glossary terms, 0 of them shared)
$ md2html docs/ -o site/ --nav
$ md2html notes.md --check
notes.md:12: a link points at #setup, which is nowhere on the page
notes.md:30: `plan.svg` does not exist next to this page
md2html: 2 to look at
$ cat notes.md | md2html - > notes.html
$ md2html notes.md --pdf --epub
$ md2html notes.md --serve
```

`--check` ora nulis apa-apa lan nyathet apa sing perlu digatèkaké panyerat, larik siji mbaka siji, lan metu nganggo kode 1 yèn ana apa-apa: pranala menyang jangkar sing ora ana ing kaca, berkas sing ora ana, cathetan sikil tanpa cathetané, HTML mentah sing dituduhaké minangka teks, terjemahan sing luwih lawas tinimbang asliné. Mulané `--check` cocog dadi langkah pungkasan saka build, utawa saka gawéané agèn. `--watch` ngrender manèh saben berkas disimpen, `--nav` nggandhèngaké kaca-kaca ing siji folder nganggo sadurungé lan sabanjuré, lan pranala menyang berkas Markdown liya tumuju menyang kacané.

## 12. Keamanan, mutu lan aksesibilitas

> [!WARNING]
> Kaca bisa dirender saka Markdown sing ditulis wong liya. md2html ora tau nulis skrip saka teks: HTML mentah dituduhaké kaya sing ditulis kajaba sawetara tag sing ora mbebayani, lan pranala `javascript:` utawa `data:` dibuwang, kepiyé waé carané nulis. Dokumen ora bisa nggawé md2html maca berkas ing njaba folderé, lan ora ana input, sanajan sing paling ala, sing bisa nggawé macet utawa mogok.

> [!CAUTION]
> `--html` nglilani HTML sing luwih sugih liwat (gambar, pranala, video, SVG, tata letak), nanging ora tau apa waé sing dilakokaké: ora ana skrip, ora ana penangan prastawa, ora ana frame, ora ana formulir.

Sing dipriksa ing saben owah-owahan, ing macOS, Linux lan Windows:

- 79 uji sing ngrender kaca wutuh lan maca manèh asilé, kasus-kasus keamanan lan kasus-kasus patologis kalebu ing kono;
- 652 conto saka spésifikasi CommonMark, sing cacahé sing lulus olèh mundhak nanging ora tau mudhun;
- lint clippy sing paling ketat, lan vèrsi Rust paling lawas sing dijanjèkaké piranti iki isih bisa kanggo mbangun;
- audit aksesibilitas nganggo axe-core tumrap WCAG 2.2 AA, ing loro téma lan ing ambané layar HP;
- uji klik ing Chrome sing nganggo saben kontrol kaya carané pamaos, nganggo mouse lan nganggo papan ketik, ing ukuran teks normal lan sing luwih gedhé;
- kaca urip, saka wiwitan nganti pungkasan: dimuat manèh yèn ana owah-owahan, panggonané pamaos dijaga, ngalih basa sing tumiba ing paragraf sing padha.

md2html ora duwé kode saka pihak katelu, lan rantai pasokané dikunci lan dipriksa: toolchain, aksi CI, piranti uji. Saben kaca dipungkasi siji larik sing anteng: digawé nganggo md2html, vèrsiné, lan pranala menyang panggonané md2html.

Dalan sing diliwati saben kaca:

```mermaid
flowchart TD
  md[Markdown] --> blocks[Blok lan inline, carané CommonMark]
  blocks --> settings[Front matter lan flag]
  settings --> page[Kaca: kerangka, panggolèkan, téma, glosarium]
  page --> check{--check}
  check -->|ora ana sing kudu didandani| out([siji berkas HTML])
  check -->|larik sing perlu dideleng| md
```

---

Glosarium ing ngisor iki nerangaké tembung-tembung sing dienggo kaca iki; saben tembung mau dadi tembung sing digaris titik ing ndhuwur.

## Glosarium

| Istilah | Teges | Uga |
|---|---|---|
| Markdown | Teks polos kanthi tandha-tandha entèng kanggo irah-irahan, penegasan, daftar lan pranala, kepénak diwaca kaya apa anané lan bisa diowahi dadi kaca. | |
| CommonMark | Spésifikasi Markdown sing saiki dianut akèh-akèhé piranti, kanthi uji kanggo saben aturan. | |
| kerangka | Daftar bab ing sisih kiwa kaca, digawé saka irah-irahané. | kerangkané |
| kothak katrangan | Katrangan sisihan ing jero kothak: cathetan, pitutur, prekara sing wigati, pènget utawa ati-ati. | alert |
| gambar | Citra ing lariké dhéwé, kanthi katrangan ing ngisoré, digedhèkaké yèn diklik. | gambaré |
| front matter | Blok setèlan ing antarané rong larik `---` ing pucuk banget berkas Markdown. | |
| MathML | Basané wèb dhéwé kanggo matématika, digambar déning peramban tanpa skrip. | |
| Content-Security-Policy | Aturan ing jero kaca sing ngandhani peramban kode endi sing olèh mlaku; ing kéné, mung kodené kaca dhéwé. | |
| Mermaid | Basa teks kanggo diagram, ditulis ing blok kode; md2html nggambar diagram alur lan diagram urutané. | |
| WCAG | Web Content Accessibility Guidelines, pedoman aksesibilitas isi wèb; tingkat AA iku target sing lumrah. | |
| terjemahan | Vèrsi kaca ing basa liya, awujud berkas Markdown ing sandhingé asliné. | |
| EPUB | Format standar kanggo buku éléktronik, kaca wèb sing dibungkus lan ditata déning e-reader. | |
