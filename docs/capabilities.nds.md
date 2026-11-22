---
title: md2html, allens, wat dat kann
description: md2html maakt ut Markdown ene kumplette Websiet, de allens binnen in sik hett, mit Översicht, Söök, Utsehns, Woortlist, Diagramms, Formels, söventein Spraken för't Bedeenen, Kopien as PDF un EPUB, un Sieden, de sik sülvst op'n neesten Stand hoolt. Disse Siet wiest dat allens, un se is dor sülvst mit maakt.
brand: md2html
kicker: Wat dat kann
version: 0.7.6
date: 2026-11-22
footer: In Markdown schreven un vun **md2html 0.7.6** sülvst rendert, elk Billschirmfoto, elk Diagramm un elk Formel binnen in disse ene Datei.
csp: true
embed: true
translated_from: capabilities.md
source_sha256: 8de84b081e289782fca730fe8ce9485182d1af3ec36101269a0b689740acb851
---

# md2html, allens, wat dat kann

md2html maakt ut en Markdown-Datei ene Websiet, de nix anners bruukt: keen Server, keen Nett, keen Schriften oder Skripten vun jichtenswo. De Siet bringt ehr egen Översicht mit, de Söök, en hell un en düster Utsehn, de Schriftgrött, de Woortlist, Anmarken, Biller, de sik grötter maken laat, bunten Code, sett Formels un Diagramms. Disse Siet is in Markdown schreven un vun md2html rendert, un so is allens, wat se beschrifft, an ehr sülvst an't Wark, wieldess een ehr leest.

## So leest een dat

- **Kapitel 1** seggt, wo md2html loppt, un wo een dat installeert.
- **Kapitel 2** is allens in een Tabell.
- **Kapitel 3 bet 9** wiest, wat en Leser kriggt, un wat en Schriever dorför tippen mutt.
- **Kapitel 10 bet 12** sünd de Instellens, de Befehlsreeg, un Sekerheit un Qualität.

---

## 1. Wo dat loppt

| | Wo |
|:--|:--|
| Dat Warktüüch md2html | macOS (Apple Silicon un Intel), Linux un Windows, ut een Rust-Programm, dat nix anners bruukt as de Standardbibliothek; bi elk Ännern op all dree pröövt |
| De Sieden, de dat maakt | aktuelle Chrome, Edge, Firefox un Safari, op Reekners, Tablets un Telefonen, online oder ahn Internet; Billschirmleser un Tastatur överall |
| Opmaakt vun | en Webserver, en deelten Ornder, en Anhang an en E-Mail, oder liek vun de Fastplaat: de Siet is ene Datei |
| Druckt oder as PDF spiekert | ut elk Browser, mit en egen Layout för't Drucken; `--pdf` maakt dat PDF sülvst, mit dat Chrome, Chromium oder Edge, wat al op'n Reekner is |
| E-Reader | `--epub` maakt en EPUB 3, dat gewöhnliche Format för E-Books |
| Spraken vun de Siet | söventein, ok vun rechts na links (Kapitel 8) |

Installeert warrt mit den egen Installer vun Rust, ut den Bornkode:

```console
$ cargo install --git https://github.com/githubrobbi/md2html --locked
$ md2html notes.md
md2html: notes.md -> notes.html (68504 bytes, 9 glossary terms, 0 of them shared)
```

## 2. Op een Blick

| | md2html |
|:--|:--|
| Markdown | heel CommonMark 0.31.2 (**640 vun sien 652** Testbispelen gaht dör), un dorto de Tabellen, Opgavenlisten, dat Dörstreken, de Autolinks, Anmarken un Henwieskastens vun GitHub |
| Wat rutkummt | **ene HTML-Datei**, nix warrt nahaalt, nix mutt en Leser installeren |
| Lesen | Översicht, Söök, hell un düster Utsehn, Schriftgrött, Woortlist, Anmarken, Biller, de sik grötter maken laat |
| Code, Formels, Diagramms | bunten Code in so üm un bi twintig Spraken, Formels as MathML, Mermaid-Flussdiagramms un -Sequenzdiagramms as SVG teekent, **allens ahn en Skript-Bibliothek** |
| Delen | en Link to elk Kapitel, elk Tabellenreeg, elk Bild, elk Diagramm, elk Codeblock oder en markeerte Steed |
| Spraken | de Siet ehr egen Wöör in **17 Spraken**, Översettens mit en Schalter verbunnen, vun rechts na links för Hebrääsch un Jiddisch |
| Behollen | de Siet sülvst, en **PDF** mit Leestekens, en **EPUB** för E-Reader |
| Live | Sieden, de nee laadt, wenn ehr Markdown sik ännert, un de Steed behoolt, wo de Leser is |
| Sekerheit | nie en Skript ut den Text; keen Datei buten den Ornder vun dat Dokument warrt leest |
| Barrierefreeheit | nix funnen gegen WCAG 2.2 AA, in beide Utsehns un in Telefonbreed |
| Gauheit | en Dokument mit 4.000 Regen, Diagramms un Formels mit dorbi, in **minner as en Teihntelsekunn** |
| Afhangigkeiten | **keen**: de Standardbibliothek vun Rust un md2html |

## 3. De Siet

De Siet hett dree Delen: en Leist baven, de Översicht links un den Text.

![En Siet in dat helle Utsehn: De Översicht markeert dat Kapitel, dat een jüst leest, un de Leist nöömt dat.](capabilities/light.jpg){width=100%}

Dat düstere Utsehn richt sik na dat System oder na dat, wat de Leser utwählt, un dat warrt sik markt:

![Desülvige Siet in dat düstere Utsehn, bi Code un Formels.](capabilities/dark-code.jpg){width=100%}

Op'n Telefon klappt de Översicht achter den Schalter in de Leist weg, un nix rullt na de Siet:

![De Siet in Telefonbreed.](capabilities/phone.jpg){width=34%}

Wat en Leser doon kann:

| Wat | So geiht dat |
|:--|:--|
| Wat finnen | **Söken** in de Leist, <kbd>⌘</kbd> <kbd>K</kbd> (<kbd>Strg</kbd> <kbd>K</kbd>) oder <kbd>/</kbd> |
| De Översicht wegdoon oder wiesen | de Schalter links in de Leist, oder <kbd>⌘</kbd> <kbd>\\</kbd> |
| De Schriftgrött ännern | **Aa** in de Leist, denn **−** un **+**; oder <kbd>⌘</kbd> <kbd>+</kbd> un <kbd>⌘</kbd> <kbd>−</kbd>, vun 85 bet 200 Perzent |
| Dat Utsehn wesseln | **Aa**, denn System, Hell oder Düster |
| En Bild grötter maken | dor op klicken; noch twee Klicks haalt dat neger ran; <kbd>Esc</kbd> sett dat trüch |
| En Anmarken oder en Woort mit Pünkten lesen | mit den Wieser dor op blieven; en Klick springt dorhen, un **trüch** bringt een trüch |
| En Kapitel, en Tabellenreeg, en Bild, en Diagramm, en Codeblock oder en Steed delen | dat Deel-Teken rechts blangen en Överschrift, an't Enn vun de Tabellenreeg ünner den Wieser, oder in de Eck vun dat Bild, dat Diagramm oder den Codeblock, wo de Wieser op blifft, kopeert ehr Link (en Telefon maakt sien Deel-Menü op); Wöör markeren, un en Link jüst to de Wöör warrt anbaden |
| Behollen | de Knoop för't Dalladen an't Enn vun de Leist: de Siet sülvst (se lett sik ahn Internet lesen), drucken oder as PDF spiekern, oder dat PDF un dat EPUB, wo se maakt worrn sünd |

Schriftgrött un Utsehn liggt tohoop achter **Aa**; för't Behollen vun de Siet gifft dat en egen Knoop:

![Aa: de Schriftgrött un dat Utsehn.](capabilities/panel.jpg){width=60%}

## 4. Schrieven

Normaal Markdown, un elk Stück lett sik ok as blanke Text good lesen. Wat een schrifft, steiht toeerst, in en Codeblock; wat een dorvun kriggt, kummt dorna.

### Rutheven un Links

```markdown
**Fett**, *schreeg*, ***beids***, ~~dörstreken~~ un `Code`.
En [Link](https://commonmark.org "CommonMark"), en blanken,
https://spec.commonmark.org/0.31.2/, un Tekens as [V], de blievt, as se schreven sünd.
```

**Fett**, *schreeg*, ***beids***, ~~dörstreken~~ un `Code`. En [Link](https://commonmark.org "CommonMark"), en blanken, https://spec.commonmark.org/0.31.2/, un Tekens as [V], de blievt, as se schreven sünd.

### Listen in Listen

```markdown
1. Eerstens, mit en List dorin:
   - en Punkt
   - noch een, mit sien egen:
     1. deper
2. Tweetens.
- [x] en Opgaav, de daan is
- [ ] en Opgaav, de noch open is
```

1. Eerstens, mit en List dorin:
   - en Punkt
   - noch een, mit sien egen:
     1. deper
2. Tweetens.

- [x] en Opgaav, de daan is
- [ ] en Opgaav, de noch open is

### Zitaten un Henwieskastens

```markdown
> En eenfach Zitat blifft en Zitat.

> [!TIP]
> De Henwieskastens vun GitHub gaht: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Henwies:** Ok de öllere Form warrt en Henwieskasten: en fett Woort un en Dubbelpunkt.
```

> En eenfach Zitat blifft en Zitat.

> [!TIP]
> De Henwieskastens vun GitHub gaht: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Henwies:** Ok de öllere Form warrt en Henwieskasten: en fett Woort un en Dubbelpunkt.

### Tabellen

```markdown
| Ruum  | Stockwark | Grött (sq ft) |
|:------|:---------:|--------------:|
| Stuuv | ünnen     |           312 |
| Böön  | baven     |           205 |
```

| Ruum | Stockwark | Grött (sq ft) |
|:-------|:------:|-------------:|
| Stuuv | ünnen | 312 |
| Böön | baven | 205 |

En Tabell, de breder is as de Spalt, rullt binnen in ehr, un de Siet deit dat nie.

### Anmarken

```markdown
En Behaupten, de en Born bruukt.[^1]

[^1]: De Born. Mit den Wieser op de Tall blieven, denn leest een ehr dor.
```

En Behaupten, de en Born bruukt.[^source]

[^source]: De Born. Mit den Wieser op de Tall blieven, denn leest een ehr dor; en Klick dor op bringt een hierher, un trüch geiht dat wedder torüch.

### Biller un Grafiken

```markdown
![En Ünnerschrift ünner dat Bild.](capabilities/zoom.jpg){width=60%}

En Symbol ![dat Teken](mark.svg){width=18} merrn in'n Satz.
```

![En Bild op 60 Perzent vun de Spalt. Klick dor op: Dat füllt dat Finster, un noch twee Klicks haalt dat neger ran.](capabilities/zoom.jpg){width=60%}

En Symbol ![dat Teken](mark.svg){width=18} merrn in'n Satz.

### Lütt Kraam

```markdown
Drück <kbd>Strg</kbd>+<kbd>K</kbd>. Water is H<sub>2</sub>O, de Ruum 9 m<sup>2</sup>.
En Reeg, de mit en Backslash ophöört,\
breekt dor üm.
```

Drück <kbd>Strg</kbd>+<kbd>K</kbd>. Water is H<sub>2</sub>O, de Ruum 9 m<sup>2</sup>. En Reeg, de mit en Backslash ophöört,\
breekt dor üm.

<details>
<summary>En Afsnitt, de bi'n Klick opgeiht</summary>

Schreven as `<details>` mit en `<summary>`, mit normaal Markdown dorin: **fett**, en List,

- een
- twee

</details>

#### En Överschrift op de veerte Ebeen

Överschriften ünner de drüdde Ebeen blievt ut de Översicht rut, so as disse hier.

## 5. Code, Formels un Diagramms

Code warrt bunt maakt för Rust, Python, JavaScript un TypeScript, Go, C un C++, Java, Kotlin, Swift, C#, de Shell, SQL, Ruby, Lua, PHP, CSS, HTML, JSON, YAML, TOML, Dockerfiles un Diffs, un elk Block hett en Knoop to'n Koperen.

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

Formels sett de Browser sülvst, as MathML, ahn Skript un ahn Schrift, de een dalladen mutt. In'n Text is de Satz vun Pythagoras $a^2 + b^2 = c^2$, un Euler sien Identität is $e^{i\pi} + 1 = 0$; en Geldbedrag as $5 oder $10 blifft Geld. As egen Block:

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

### Diagramms

En Mermaid-Flussdiagramm oder -Sequenzdiagramm teekent md2html sülvst, as SVG binnen in de Siet: keen Skript, nix to'n Dalladen, de Siet ehr egen Klören in beide Utsehns, un sien Wöör blievt Wöör, de de Söök finnt. Dit is dat Markdown:

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[Dat Markdown lesen]
  parse --> render{Diagramm?}
  render -->|Fluss- oder Sequenzdiagramm| svg[As SVG teken]
  render -->|en anner Oort| js[mermaid.js, wenn dat dor is]
  svg --> page([ene HTML-Siet])
  js --> page
```
````

Un dit warrt dorut:

```mermaid
flowchart LR
  md[notes.md] --> parse[Dat Markdown lesen]
  parse --> render{Diagramm?}
  render -->|Fluss- oder Sequenzdiagramm| svg[As SVG teken]
  render -->|en anner Oort| js[mermaid.js, wenn dat dor is]
  svg --> page([ene HTML-Siet])
  js --> page
```

En Sequenzdiagramm, nummereert, mit en Notiz:

```mermaid
sequenceDiagram
  autonumber
  actor R as Leser
  participant P as De Siet
  participant S as Söök
  R->>P: Cmd K
  P->>S: de Söök opmaken
  R->>S: "Henwies" tippen
  S-->>R: de Steden, de besten toeerst
  R->>S: Enter
  S-->>P: hinspringen, un de Wöör lüchten laten
  Note over R,P: keen Nett, keen Server
```

Anner Oorten (Tortendiagramms, Gantt-Diagramms, Klassendiagramms) teekent mermaid.js, wenn een md2html op en Kopie dorvun wiest (`--mermaid-js FILE`); dat Skript kummt denn blots in disse ene Siet. Ahn dat blievt se Code, un `--check` seggt dat.

## 6. Woortlist, Anmarken, Söök un Biller

En Kapitel mit den Naam Woortlist höllt de Siet ehr Wöör as Tabell. Dat warrt nich dor druckt, wo dat steiht: Statts dat warrt dat eerste Mal, dat en Woort in en Kapitel vörkummt, to en Woort mit Pünkten, dat sik sülvst verklaart, wenn de Wieser dor op liggt, un en Klick dor op springt na'n Indrag ünnen an de Siet, de een wedder na de Reeg bringt, de een jüst leest hett.

![Liggt de Wieser op en Woort mit Pünkten, wiest dat sien Bedüden.](capabilities/glossary.jpg){width=70%}

Anmarken gaht jüst so: De Tall verklaart sik sülvst, wenn de Wieser dor op liggt.

![Liggt de Wieser op de Tall vun en Anmarken, wiest se den Text dorto.](capabilities/footnote.jpg){width=70%}

> [!NOTE]
> En Woortlist kann för mehr as een Siet gellen: ene Datei ünner `~/.config/md2html/glossary.md`, un ehr Wöör warrt denn op elk Siet verlinkt, de op dissen Reekner rendert warrt.

De Söök geiht över elk Woort op de Siet, de Woortlist mit. De Funnen wiest, wo elk Woort steiht; dat utwählte lücht dor op, wo de Söök ankummt.

![Söken na „callout“: dat Kapitel un de Steed vun elk Fund.](capabilities/search.jpg){width=80%}

![Wo de Söök ankamen is: De Indrag glöht, un dat funnen Woort lücht.](capabilities/search-landed.jpg){width=80%}

En Bild warrt över de Siet grötter, ahn dat een ehr verlaten mutt. Is dat grötter as dat Finster, folgt dat den Wieser, so dat een op dat wiest, wat een sehn will.

## 7. Delen un Live-Sieden

Elk Överschrift hett rechts blangen sik en Deel-Teken; elk Tabellenreeg, elk Bild, elk Diagramm un elk Codeblock wiest een, wenn de Wieser dor op liggt. En Klick kopeert en Link jüst to disse Steed un seggt dat ok; op'n Telefon geiht statts dat dat Deel-Menü vun't System op. Markeer en poor Wöör, un de Siet bütt en Link jüst to ehr an, de de Siet bi disse Steed opmaakt, rutheven.

![Markeerte Wöör beedt en Link to sik an.](capabilities/share.jpg){width=80%}

En Siet kann sik ok sülvst op'n neesten Stand holen. `md2html --serve` wiest de Sieden op dissen Reekner, wieldess se schreven warrt, un elk open Siet laadt nee, sowie ehr Markdown spiekert warrt. En Siet, de rutbröcht is un mit `--live` maakt worrn is, hett ehr egen Levenstiet: is de üm, fraagt de Siet ehrn Server na dat eerste Kilobyte vun sik sülvst, wo ehr Fingerafdruck steiht, un laadt blots nee, wenn sik de ännert hett. So oder so blifft, wer leest, dor, wo he weer: bi densülvigen Afsatz op desülvige Hööch oder, wenn de weg is, bi dat Kapitel dorvör; wesselt een na en anner Spraak, blifft de Steed op desülvige Wies.

## 8. Spraken

De Siet ehr egen Wöör (de Söök, de Henwieskastens, de Woortlist, de Datums, elk Knoop) gifft dat in söventein Spraken: Engelsch, Düütsch, Hebrääsch, Indonesisch, Nedderlannsch, Javaansch, Jiddisch, Plattdüütsch, Franzöösch, Italieensch, Spaansch, Japaansch op dree Oorten schreven (in Kanji un Kana, blots in Kana un in Rōmaji), Mandarin-Chineesch in vereenfachte un in traditschonelle Schriftteken, un Kantoneesch. `lang: de` in'n Vörspann is allens, wat een dorför bruukt. Hebrääsch un Jiddisch warrt vun rechts na links sett, de Översicht rechts.

En Översetten is eenfach noch en Markdown-Datei blangen dat Original, nöömt na ehr Spraak: `capabilities.de.md` is dat düütsche `capabilities.md`. md2html finnt de Versionen, de dor sünd, verbinnt se mit en Schalter in de Leist (den Globus), seggt dat Browsers un Söökmaschinen mit `hreflang` un gifft elk de Wöör för ehr egen Bedeenen. Disse Siet gifft dat in all söventein; de Globus wesselt twischen ehr, un de Siet geiht bi den Afsatz op, den een jüst leest hett.

Elk Översetten nöömt ehr Original un schrifft den Fingerafdruck vun't Original op, so dat de Siet seggen kann, welk Version de gellt, un `--check` marken deit, wenn dat Original sik ännert hett, siet dat översett worrn is:

```yaml
translated_from: capabilities.md
source_sha256: 5b0c...e1
```

För en Spraak, in de noch nüms en Siet översett hett, kann de Browser vun'n Leser ehr översetten, wenn he dat kann (de inboote Översetter vun Chrome): op dat Reedschap sülvst, so dat dor nix rutgeiht, kenntlich maakt as Maschien-Översetten, mit en Weg trüch na dat Original.

## 9. En Siet behollen

De Knoop för't Dalladen in de Leist bütt dat an, wat dor to behollen is. De Siet sülvst lett sik ahn Internet lesen, wenn ehr Biller binnen in ehr sünd (`embed: true`). För't Drucken gifft dat en egen Layout: De Knööp fallt weg, elk Link druckt sien Adress, nix warrt över en Siet weg dörsneden, un de Sieden dreegt den Titel un ehr Nummer.

![Desülvige Siet in'n Druck.](capabilities/print.jpg){width=55%}

`--pdf` druckt de Siet mit dat Chrome, wat al op'n Reekner is: ehr egen Layout för't Drucken, de Överschriften as de Leestekens vun't PDF, de Titel baven op elk Siet un ehr Nummer nerrn. `--epub` maakt ut desülvige Siet en Book för E-Reader, mit Inholtsverteken, Biller, Diagramms un Formels, un mit Anmarkens, de dor opgaht, wo se nöömt warrt; md2html schrifft dat sülvst. Wer leest, finnt beide ünner den Knoop för't Dalladen.

## 10. Instellens

Baven in de Markdown-Datei vun dat engelsche Original vun disse Siet steiht:

```yaml
---
title: md2html, everything it does
brand: md2html
kicker: Capabilities
version: 0.7.6
date: 2026-11-22
footer: Written in Markdown and rendered by **md2html 0.7.6** itself ...
csp: true      # only this page's own code may run
embed: true    # the screenshots are inside the page
---
```

| Instellen | Wat dat deit |
|:--|:--|
| `title`, `description`, `author` | de Titel, un de Meta-Tags för de Vörschau vun Links |
| `lang` | de Spraak: de Siet ehr egen Wöör un de Datums in elk vun de söventein (`ja-Hira` is Japaansch in Kana, `zh-Hant` Chineesch in traditschonelle Schriftteken, `yue` Kantoneesch) |
| `translated_from`, `source_sha256` | för en Översetten: dat Original, un de Fingerafdruck vun't Original to de Tiet, as översett worrn is |
| `brand`, `brand_link`, `kicker`, `eyebrow`, `footer` | de Leist, de Reeg över den Titel, dat Lüttgedruckte |
| `date`, `version` | Datum un Version över den Titel |
| `embed` | Biller binnen in de Siet, so dat se ene Datei is |
| `csp` | en Content-Security-Policy |
| `pdf`, `epub` | en PDF un en EPUB vun de Siet, blangen ehr |
| `live` | de Levenstiet vun de Siet (`true` sünd twee Minuten, anners en Tall vun Sekunnen): denn laadt se nee, wenn se sik ännert hett, un wer leest, blifft an desülvige Steed |
| `large`, `toc`, `math`, `index`, `html`, `css`, `head` | gröttere Schrift, de Översicht, Formels, Söökmaschinen, rieker HTML, extra Stil un Head |

> [!TIP]
> Mit `lang: de` seggt de Söök „Diese Seite durchsuchen“, de Henwieskastens seggt „Hinweis“ un „Achtung“, un dat Datum lett „22. November 2026“.

## 11. De Befehlsreeg

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

`--check` schrifft nix, list Reeg för Reeg op, wat en Schriever sik ankieken schull, un höört mit 1 op, wenn dor wat is: en Link na en Anker, de nich op de Siet is, en Datei, de nich dor is, en Anmarken ahn ehrn Text, HTML, dat as Text wiest warrt, en Översetten, de öller is as ehr Original. Dormit is dat de letzte Schritt vun en Build, oder vun de Arbeit vun en Agent. `--watch` rendert bi elk Spiekern nee, `--nav` verbinnt de Sieden vun en Ornder mit vörige un neegste, un en Link na en anner Markdown-Datei geiht na ehr Siet.

## 12. Sekerheit, Qualität un Barrierefreeheit

> [!WARNING]
> En Siet kann ut Markdown rendert warrn, dat en anner schreven hett. md2html schrifft nie en Skript ut den Text: HTML in'n Text warrt so wiest, as dat schreven is, bet op en poor harmlose Tags, un `javascript:`- oder `data:`-Links warrt wegdaan, egaal wo se schreven sünd. En Dokument kann md2html nich dorto bringen, en Datei buten sien Ornder to lesen, un keen Ingaav, egaal wo bösaardig, kann dat lahmleggen oder afstörten laten.

> [!CAUTION]
> `--html` lett rieker HTML dör (Biller, Links, Video, SVG, Layout), man nie wat, dat loppt: keen Skript, keen Event-Handler, keen Frame, keen Formular.

Wat bi elk Ännern pröövt warrt, op macOS, Linux un Windows:

- 79 Tests, de hele Sieden rendert un wedder leest, de Sekerheitsfäll un de verdreihten Fäll dormang;
- de 652 Bispelen vun de CommonMark-Spezifikatschoon, un de Tall vun de, de dörgaht, dörv stiegen, man nie fallen;
- de strengsten Lints vun clippy, un dat öllste Rust, op dat sik dat Warktüüch boen laten schall;
- en Pröven op Barrierefreeheit mit axe-core gegen WCAG 2.2 AA, in beide Utsehns un in Telefonbreed;
- en Dörklicken in Chrome, dat elk Knoop so bruukt, as en Leser dat dee, mit Muus un mit Tastatur, bi normale un bi gröttere Schriftgrött;
- Live-Sieden, vun Anfang bet Enn: nee laden, wenn sik wat ännert, de Steed vun'n Leser behollen, en Spraakwessel, de bi densülvigen Afsatz ankummt.

md2html hett keen Code vun annere Lüüd, un sien Lieferkeed is fastleggt un pröövt: dat Warktüüch to'n Boen, de CI-Akschonen, de Warktüüch för de Tests. Elk Siet hört mit en stille Reeg op: maakt mit md2html, sien Version, un en Link dorhen, wo md2html tohuus is.

De Weg, den elk Siet nimmt:

```mermaid
flowchart TD
  md[Markdown] --> blocks[Blöck un Inlines, so as CommonMark dat will]
  blocks --> settings[Vörspann un Schalters]
  settings --> page[De Siet: Översicht, Söök, Utsehns, Woortlist]
  page --> check{--check}
  check -->|nix to repareren| out([ene HTML-Datei])
  check -->|Regen, de een ankieken schull| md
```

---

De Woortlist hierünner verklaart de Wöör, de disse Siet bruukt; elk weer baven en Woort mit Pünkten.

## Woortlist

| Woort | Bedüden | Ook |
|---|---|---|
| Markdown | Blanke Text mit lütte Tekens för Överschriften, Rutheven, Listen un Links, de een so lesen kann, as he is, un de sik to en Siet maken lett. | |
| CommonMark | De Spezifikatschoon vun Markdown, de de mehrsten Warktüüch nu folgt, mit en Test för elk Regel. | |
| Översicht | De List vun de Kapitels links op de Siet, maakt ut ehr Överschriften. | |
| Henwieskasten | En Kasten an de Siet: en Henwies, en Tipp, wat Wichtiges, en Wohrschau oder en Vörsicht. | Henwieskastens |
| Bild | En Grafik op en egen Reeg, mit ehr Ünnerschrift dorünner, de bi'n Klick grötter warrt. | Biller |
| Vörspann | De Block mit Instellens twischen twee `---`-Regen ganz baven in en Markdown-Datei. | |
| MathML | De egen Spraak vun't Web för Formels, de de Browser ahn Skript teekent. | |
| Content-Security-Policy | En Regel in de Siet, de den Browser seggt, welk Code lopen dörv; hier blots de vun de Siet sülvst. | |
| Mermaid | En Textspraak för Diagramms, in en Codeblock schreven; md2html teekent ehr Flussdiagramms un Sequenzdiagramms. | |
| WCAG | De Web Content Accessibility Guidelines, de Regeln för Barrierefreeheit in't Web; de Stuuf AA is dat gewöhnliche Teel. | |
| Översetten | En Version vun en Siet in en anner Spraak, en Markdown-Datei blangen dat Original. | Översettens |
| EPUB | Dat gewöhnliche Format för E-Books, en inpackte Websiet, de en E-Reader sülvst sett. | |
