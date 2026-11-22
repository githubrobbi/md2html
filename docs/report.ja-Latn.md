---
title: md2html no dekiru koto subete
description: md2html 0.4 ni tsuite no hōkoku desu. Nio-ke no pēji de tsukau, Markdown o HTML ni henkan suru tsūru de, kono hōkoku mo Markdown de kakare, md2html jishin de henkan sarete imasu. Sono tame, koko de setsumei suru kinō wa subete, kono pēji no ue de mo tsukawarete imasu.
brand: md2html
kicker: Hōkoku
version: 0.7.6
date: 2026-11-22
footer: Markdown de kaki, **md2html 0.7.6** jishin de henkan shimashita. Gazō mo fukumete, subete ga kono hitotsu no pēji no naka ni arimasu. Kazoku no pēji de, ippan ni wa kōkai shite imasen. Go-shitsumon wa Robert made.
csp: true
embed: true
translated_from: report.md
source_sha256: b0b95fa4c9cbe903d80276342c8237cc1fa25befa2dc2a2dead3d6e26b10c131
---

# md2html no dekiru koto subete

md2html wa, Markdown fairu o, hoka ni nani mo iranai hitotsu no HTML pēji ni shimasu. Sābā mo, nettowāku mo, doko ka kara yomikomu fonto ya sukuriputo mo irimasen. Kono hōkoku mo, sō iu pēji desu. Markdown de kakare, md2html de henkan sarete iru no de, koko de setsumei suru kinō wa subete, kono pēji no ue de mo hataraite imasu: hidari no mokuji, kensaku, futatsu no tēma, moji no ōkisa, shita no kādo, yōgoshū no tensen no tsuita go, kyakuchū, kakudai dekiru zuhan, iro no tsuita kōdo, soshite kumihan sareta sūshiki desu.

## Kono pēji no yomikata

- **Dai 1 shō** wa, hanashi no zentai o hitotsu no hyō ni matomete imasu. Ni-fun de yomemasu.
- **Dai 2 shō kara dai 6 shō** de wa, yomu hito ga nani o eru ka, soshite kaku hito ga sono tame ni nani o uchikomu ka o shimeshimasu.
- **Dai 7 shō to dai 8 shō** wa, settei to komando rain desu.
- **Dai 9 shō to dai 10 shō** wa, anzensei to hinshitsu, soshite bājon 0.4 ga dekita keii desu.

---

## 1. Hitome de

| | Mae (0.3) | Ima (0.4) |
|:--|:--:|:--:|
| Yomeru Markdown | handobukku de tsukau bubun dake | CommonMark no subete to, GitHub no tsuika bubun |
| Tsūka suru CommonMark no rei | hakatte inai | **652 chū 640** |
| 4,265 gyō no docenta handobukku | ni-fun ijō | **0.07 byō** |
| `[V]` no shirushi darake no pēji | 38 GB made fukureagatta | sū miri-byō |
| Akusesibiriti no shiteki (axe-core, WCAG 2.2 AA) | sūjū ken | 12 pēji, raito, dāku, sumaho de **zero** |
| Tesuto | 10 | 53, sore ni shiyō no rei 652 to, Chrome de no kurikku tesuto |
| Mermaid no zu | kōdo no mama hyōji | sukuriputo nashi de **byōga** |
| Gengo | Eigo | **17**, hon'yaku no aida no kirikae tsuki, migi kara hidari no gengo mo fukumu |
| Nokoshite oku katachi | pēji | pēji, shiori tsuki no **PDF**, denshi shoseki rīdā-yō no **EPUB** |
| Jikkōji no izon kankei | nashi | nashi |

0.4 de kuwawatta mono o, tasuku risuto ni suru to:

- [x] CommonMark no shiyō ga setsumei suru tōri ni tsukurareta, kanzen na Markdown no yomitori
- [x] kyakuchū, tasuku risuto, torikeshisen, jidō rinku, soroe o shitei dekiru hyō, soshite GitHub no itsutsu no arāto
- [x] yaku nijū no gengo de iro o tsuketa kōdo, MathML de kumihan sareta sūshiki
- [x] furonto matā de no settei, soshite pēji jitai no kotoba no Doitsugo-ban
- [x] `--check`, `--watch`, foruda, stdin, nozomeba tsukaeru Content-Security-Policy
- [x] akusesibiriti kansa to, CI de no kurikku tesuto
- [x] Mermaid no furōchāto to shīkensu-zu o, sukuriputo nashi de SVG to shite byōga (dai 4 shō)
- [x] jūnana no gengo de no kono hōkoku to, bā no kirikae (dai 7 shō)
- [x] pēji o nokoshite oku koto, insatsu suru koto, PDF ya EPUB to shite mochiaruku koto (dai 2 shō to dai 8 shō)

> [!IMPORTANT]
> 4sons.com de md2html o tsukatte tsukutta pēji (keikaku, chōsa, kazoku no pēji) wa subete 0.4 de tsukurinaoshi, kōkai-chū datta han to kurabemashita. Kōzō wa onaji ka, yori yoku natte imashita.

## 2. Pēji

Pēji ni wa mittsu no bubun ga arimasu: ue no bā, hidari no mokuji, soshite honbun desu.

![Raito tēma no shōkēsu pēji. Mokuji wa ima yonde iru shō ni shirushi o tsuke, bā ni sono shō no namae ga demasu.](report/light.jpg){width=100%}

Dāku tēma wa shisutemu no settei ka, yomu hito no sentaku ni shitagai, sono sentaku wa oboete okaremasu:

![Onaji pēji no dāku tēma. Kōdo to sūshiki no tokoro desu.](report/dark-code.jpg){width=100%}

Sumaho de wa, mokuji wa bā no suitchi no ura ni tatamare, yoko ni sukurōru suru mono wa arimasen:

![Sumaho no haba de no pēji.](report/phone.jpg){width=34%}

Yomu hito ga dekiru koto:

| Shitai koto | Yarikata |
|:--|:--|
| Nanika o sagasu | bā no **Kensaku**, <kbd>⌘</kbd> <kbd>K</kbd> (<kbd>Ctrl</kbd> <kbd>K</kbd>), mata wa <kbd>/</kbd> |
| Mokuji o kakusu, mata wa hyōji suru | bā no hidari no suitchi, mata wa <kbd>⌘</kbd> <kbd>\\</kbd> |
| Moji no ōkisa o kaeru | bā no **Aa**, sore kara **−** to **+**. Mata wa <kbd>⌘</kbd> <kbd>+</kbd> to <kbd>⌘</kbd> <kbd>−</kbd>. 85 kara 200 pāsento made |
| Tēma o kaeru | **Aa**, sore kara Shisutemu, Raito, mata wa Dāku |
| Zuhan o kakudai suru | kurikku shimasu. Sara ni ni-kai kurikku suru to, motto chikazukimasu. <kbd>Esc</kbd> de moto ni modorimasu |
| Kyakuchū ya tensen no tsuita go o yomu | pointā o nosemasu. Kurikku suru to soko e idō shi, **modoru** de moto no basho ni modorimasu |
| Shō, hyō no gyō, zuhan, daiaguramu, kōdo burokku, bunshō no ichibu o kyōyū suru | midashi no migi, pointā no shita ni aru hyō no gyō no owari, mata wa pointā no shita ni aru zuhan, daiaguramu, kōdo burokku no sumi no kyōyū kigō de sono rinku ga kopī saremasu (sumaho de wa kyōyū shīto ga hirakimasu). Kotoba o sentaku suru to, chōdo sono bubun e no rinku ga teian saremasu |
| Nokoshite oku | bā no saigo ni aru daunrōdo botan: pēji sono mono (ofurain de mo yomemasu), insatsu mata wa PDF to shite hozon, aruiwa, tsukurarete ireba PDF to EPUB |

## 3. Kaku

Futsū no Markdown desu. Dono kakikata mo, purēn tekisuto no mama de mo yoku yomemasu. Kaku mono o saki ni kōdo burokku de shimeshi, dekiagari o sono ato ni shimeshimasu.

### Kyōchō to rinku

```markdown
**Futoji**, *shatai*, ***ryōhō***, ~~torikeshi~~, `kōdo`.
[Rinku](https://commonmark.org "CommonMark"), sono mama kaita rinku
https://spec.commonmark.org/0.31.2/, soshite [V] no yō na shirushi wa kaita tōri ni nokorimasu.
```

**Futoji**, *shatai*, ***ryōhō***, ~~torikeshi~~, `kōdo`. [Rinku](https://commonmark.org "CommonMark"), sono mama kaita rinku https://spec.commonmark.org/0.31.2/, soshite [V] no yō na shirushi wa kaita tōri ni nokorimasu.

### Risuto no naka no risuto

```markdown
1. Hitotsume. Naka ni risuto ga arimasu:
   - kajōgaki
   - mō hitotsu. Kore ni mo naka ni:
     1. sara ni fukaku
2. Futatsume.
```

1. Hitotsume. Naka ni risuto ga arimasu:
   - kajōgaki
   - mō hitotsu. Kore ni mo naka ni:
     1. sara ni fukaku
2. Futatsume.

### In'yō to kōruauto

```markdown
> Futsū no in'yō wa, in'yō no mama desu.

> [!TIP]
> GitHub no arāto ga tsukaemasu: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Memo:** furui kakikata, tsumari futoji no go to koron mo tsukaemasu.
```

> Futsū no in'yō wa, in'yō no mama desu.

> [!TIP]
> GitHub no arāto ga tsukaemasu: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Memo:** furui kakikata, tsumari futoji no go to koron mo tsukaemasu.

### Hyō

```markdown
| Heya    | Kai   | Menseki (sq ft) |
|:--------|:-----:|----------------:|
| Ribingu | ikkai |             312 |
| Rofuto  | nikai |             205 |
```

| Heya | Kai | Menseki (sq ft) |
|:--------|:-----:|----------------:|
| Ribingu | ikkai | 312 |
| Rofuto | nikai | 205 |

### Kyakuchū

```markdown
Shutten ga hitsuyō na shuchō.[^1]

[^1]: Shutten. Bangō ni pointā o noseru to, sono ba de yomemasu.
```

Shutten ga hitsuyō na shuchō.[^source]

[^source]: Shutten. Bangō ni pointā o noseru to, sono ba de yomemasu. Kurikku suru to koko ni ki, modoru de moto no basho ni modorimasu.

### Zuhan to gazō

```markdown
![Gazō no shita no kyapushon.](report/zoom.jpg){width=60%}

Bun no tochū ni aikon ![māku](mark.svg){width=18} ga arimasu.
```

![Honbun no haba no 60 pāsento no zuhan. Kurikku suru to windō ippai ni hirogari, sara ni ni-kai kurikku suru to motto chikazukimasu.](report/zoom.jpg){width=60%}

Bun no tochū ni aikon ![māku](mark.svg){width=18} ga arimasu.

### Komakai koto

```markdown
<kbd>Ctrl</kbd>+<kbd>K</kbd> o oshimasu. Mizu wa H<sub>2</sub>O, heya wa 9 m<sup>2</sup>.
Bakkusurasshu de owaru gyō wa\
soko de kaigyō shimasu.
```

<kbd>Ctrl</kbd>+<kbd>K</kbd> o oshimasu. Mizu wa H<sub>2</sub>O, heya wa 9 m<sup>2</sup>. Bakkusurasshu de owaru gyō wa\
soko de kaigyō shimasu.

<details>
<summary>Kurikku suru to hiraku sekushon</summary>

`<details>` to `<summary>` de kakare, naka wa futsū no Markdown desu: **futoji**, risuto,

- hitotsu
- futatsu

</details>

#### Dai yon reberu no midashi

Dai san reberu yori shita no midashi wa, kono midashi no yō ni, mokuji ni wa demasen.

## 4. Kōdo to sūshiki

Kōdo wa Rust, Python, JavaScript to TypeScript, Go, C to C++, Java, Kotlin, Swift, C#, sheru, SQL, Ruby, Lua, PHP, CSS, HTML, JSON, YAML, TOML, Dockerfile, soshite diff ni iro ga tsuki, dono burokku ni mo kopī botan ga arimasu.

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

```yaml
title: The plans   # front matter is YAML
lang: de
embed: true
```

Sūshiki wa burauza jishin ga MathML to shite kumihan shimasu. Sukuriputo mo, daunrōdo suru fonto mo irimasen. Bun no naka de wa, Pitagorasu no teiri wa $a^2 + b^2 = c^2$, Ōirā no tōshiki wa $e^{i\pi} + 1 = 0$ desu. $5 ya $10 no yō na kingaku wa okane no mama desu. Betsu no gyō ni hyōji suru to:

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

### Daiaguramu

Mermaid no furōchāto ya shīkensu-zu wa, md2html jishin ga pēji no naka no SVG to shite egakimasu. Sukuriputo mo daunrōdo mo irazu, dochira no tēma de mo pēji jishin no iro o tsukai, zu no naka no kotoba wa kensaku de mitsukaru kotoba no mama desu. Markdown wa kō kakimasu:

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[Markdown o yomu]
  parse --> render{Daiaguramu?}
  render -->|furōchāto ka shīkensu| svg[SVG to shite egaku]
  render -->|sore igai no shurui| js[mermaid.js, ataerareta baai]
  svg --> page([hitotsu no HTML pēji])
  js --> page
```
````

Soshite, kō narimasu:

```mermaid
flowchart LR
  md[notes.md] --> parse[Markdown o yomu]
  parse --> render{Daiaguramu?}
  render -->|furōchāto ka shīkensu| svg[SVG to shite egaku]
  render -->|sore igai no shurui| js[mermaid.js, ataerareta baai]
  svg --> page([hitotsu no HTML pēji])
  js --> page
```

Bangō to memo no tsuita shīkensu-zu:

```mermaid
sequenceDiagram
  autonumber
  actor R as Dokusha
  participant P as Pēji
  participant S as Kensaku
  R->>P: Cmd K
  P->>S: kensaku o hiraku
  R->>S: "callout" to nyūryoku
  S-->>R: kasho, yoi mono kara jun ni
  R->>S: Enter
  S-->>P: tobu, soshite kotoba o hikaraseru
  Note over R,P: nettowāku nashi, sābā nashi
```

Sono hoka no shurui (en gurafu, ganto chāto, kurasu-zu) wa, chosha ga md2html ni mermaid.js no kopī no basho o oshieta baai (`--mermaid-js FILE`) ni, mermaid.js ga egakimasu. Sono toki, sukuriputo wa sono pēji ni dake hairimasu. Oshienakereba kōdo no mama de, `--check` ga sō tsutaemasu.

## 5. Yōgoshū to kyakuchū

Yōgoshū to iu namae no shō ni wa, pēji no kotoba ga hyō to shite naraberaremasu. Kono shō wa, kaita basho ni wa sono mama hyōji saremasen. Kawari ni, kaku shō de kaku yōgo ga saisho ni deta tokoro ga tensen no tsuita go ni nari, pointā o noseru to imi o setsumei shimasu. Kurikku suru to pēji no saigo no kōmoku e tobi, soko kara yonde ita gyō e modoremasu.

![Tensen no tsuita go ni pointā o noseru to, imi ga hyōji saremasu.](report/glossary.jpg){width=70%}

Kyakuchū mo onaji shikumi desu. Bangō ni pointā o noseru to, chū no naiyō ga hyōji saremasu.

![Kyakuchū no bangō ni pointā o noseru to, chū ga hyōji saremasu.](report/footnote.jpg){width=70%}

> [!NOTE]
> Yōgoshū wa fukusū no pēji de kyōtsū ni dekimasu. `~/.config/md2html/glossary.md` ni fairu o hitotsu oku to, sono kotoba wa, sono konpyūta de henkan suru subete no pēji de rinku ni narimasu.

## 6. Kensaku to zuhan

Kensaku wa pēji no subete no kotoba o taishō ni shi, yōgoshū mo fukumimasu. Kekka ni wa kaku go ga doko ni aru ka ga hyōji sare, eranda kekka wa, kensaku de tobu saki de hikarimasu.

!["callout" o kensaku shita tokoro: kaku kekka no shō to kasho.](report/search.jpg){width=80%}

![Kensaku de tonda saki: kōmoku ga hikari, mitsukatta go ga hikatte imasu.](report/search-landed.jpg){width=80%}

Zuhan wa pēji kara hanarezu ni, pēji no ue de kakudai saremasu. Windō yori ōkii toki wa pointā ni tsuite ugoku no de, mitai tokoro o sasu dake desu.

## 7. Settei

Kono hōkoku no Eigo no genpon no Markdown fairu wa, sentō ga kō natte imasu:

```yaml
---
title: md2html, everything it does
brand: md2html
kicker: Report
version: 0.7.6
date: 2026-11-22
footer: Written in Markdown and rendered by **md2html 0.7.6** itself ...
csp: true      # only this page's own code may run
embed: true    # the screenshots are inside the page
---
```

| Settei | Kinō |
|:--|:--|
| `title`, `description`, `author` | taitoru to, rinku no purebyū-yō no meta tagu |
| `lang` | gengo: pēji jitai no kotoba to hizuke o, ue no jūnana no gengo no dore de mo hyōji (`ja-Hira` wa kana dake de kaita Nihongo, `zh-Hant` wa hantaiji no Chūgokugo, `yue` wa Kantongo) |
| `translated_from`, `source_sha256` | hon'yaku no baai: sono genpon to, hon'yaku shita jiten de no genpon no shimon |
| `brand`, `brand_link`, `kicker`, `eyebrow`, `footer` | bā, taitoru no ue no gyō, pēji no ichiban shita no komakai moji |
| `date`, `version` | taitoru no ue ni hyōji suru hizuke to bājon |
| `embed` | gazō o pēji no naka ni ire, hitotsu no fairu ni suru |
| `csp` | Content-Security-Policy |
| `pdf`, `epub` | pēji no PDF to EPUB o, sono tonari ni tsukuru |
| `live` | pēji no yūkō kikan (`true` wa ni-fun, mata wa byōsū): sono ato, kawatte ireba yomikominaoshi, dokusha no ichi wa sono mama |
| `large`, `toc`, `math`, `index`, `html`, `css`, `head` | ōkii moji, mokuji, sūshiki, kensaku enjin, nama no HTML, tsuika no sutairu to head |

> [!TIP]
> `lang: de` ni suru to, kensaku wa "Diese Seite durchsuchen" to hyōji shi, kōruauto wa "Hinweis" ya "Achtung" to nari, hizuke wa "22. November 2026" to narimasu. Kono saito no Thomas no pēji wa, kono yō ni shite Doitsugo ni natte imasu.

### Gengo

Kono hōkoku wa jūnana no gengo de yomemasu: Eigo, Doitsugo, Heburaigo, Indoneshiago, Orandago, Jawago, Idisshugo, Teichi Doitsugo, Furansugo, Itariago, Supeingo, mittsu no kakikata no Nihongo (kanji to kana, kana dake, soshite rōmaji), kantaiji to hantaiji no Chūgokugo (futsūwa), soshite Kantongo desu. Bā no naka no chikyūgi de kirikaerare, pēji wa yonde ita shō de hirakimasu.

Hon'yaku wa, genpon no tonari ni oita betsu no Markdown fairu ni suginaku, fairu-mei ni sono gengo ga tsuite imasu: `report.de.md` wa `report.md` no Doitsugo-ban desu. md2html wa soko ni aru han o mitsuke, tagai ni rinku shi (burauza to kensaku enjin ni wa `hreflang` de tsutae), sorezore ni sono gengo no intāfēsu no kotoba o ataemasu: kensaku, kōruauto, yōgoshū, hizuke desu. Heburaigo to Idisshugo wa migi kara hidari ni kumareru no de, mokuji wa migi ni narimasu.

Kaku hon'yaku wa jibun no genpon o shimeshi, genpon no shimon o kiroku shimasu. Sono tame, pēji wa dore ga seishiki na han ka o shimesu koto ga deki, hon'yaku shita ato ni genpon ga kawatta baai wa `--check` ga ki ga tsukimasu:

```yaml
translated_from: report.md
source_sha256: 5b0c...e1
```

Dare mo hon'yaku shite inai gengo no baai wa, yomu hito no burauza ni sono kinō ga areba (Chrome no hyōjun no hon'yaku kinō), burauza ga hon'yaku dekimasu. Tanmatsu no naka de okonawareru no de, kazoku no pēji no naiyō wa nani mo soto ni demasen. Kikai hon'yaku de aru koto ga hyōji sare, genpon ni modoru hōhō mo arimasu.

## 8. Komando rain

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

`--check` wa nani mo kakidasazu, chosha ga mite oku beki tokoro o gyō goto ni rekkyo shi, nanika areba 1 de shūryō shimasu: pēji ni nai ankā e no rinku, sonzai shinai fairu, chū no nai kyakuchū, tekisuto to shite hyōji sarete shimau nama no HTML nado desu. Sono tame, birudo no saigo no suteppu ya, ējento no sagyō no saigo no suteppu ni muite imasu. `--watch` wa hozon suru tabi ni henkan shinaoshi, `--nav` wa foruda no pēji o "mae" to "tsugi" de tsunagi, betsu no Markdown fairu e no rinku wa sono pēji e tsunagarimasu.

`--pdf` wa, konpyūta ni sude ni aru Chrome de pēji o insatsu shimasu: pēji jishin no insatsu-yō reiauto de, midashi wa PDF no shiori ni nari, kaku pēji no ue ni wa taitoru, shita ni wa pēji bangō ga tsukimasu. `--epub` wa onaji pēji kara denshi shoseki rīdā-yō no hon o tsukurimasu. Mokuji, gazō, zu, sūshiki ga hairi, kyakuchū wa sore ga dete kuru basho de hirakimasu. EPUB wa md2html ga jibun de kakidashimasu. Dokusha wa dochira mo bā no daunrōdo botan no naka ni mitsukeraremasu.

`--serve` wa, kaite iru aida, kono konpyūta de pēji o hyōji shimasu: Markdown o hozon shita shunkan ni, hiraite iru pēji ga subete yomikominaosaremasu. `--live` o tsukete kōkai shita pēji ni wa, sore jishin no yūkō kikan ga arimasu. Kikan ga kireru to, pēji wa jibun no sābā ni jibun jishin no saisho no ichi kirobaito o toiawase, sore ga kawatte ita toki dake yomikominaoshimasu. Dochira no baai mo, dokusha wa ita basho ni todomarimasu: onaji danraku ga onaji takasa ni kuru ka, sono danraku ga nakunatte ireba, sono mae no shō ni modorimasu.

## 9. Anzensei, hinshitsu, akusesibiriti

> [!WARNING]
> Pēji wa, tanin ga kaita Markdown kara henkan sareru koto mo arimasu. md2html wa honbun kara sukuriputo o kesshite tsukurimasen. Nama no HTML wa, ikutsu ka no mugai na tagu o nozoite kaita tōri ni hyōji sare, `javascript:` ya `data:` no rinku wa, donna tsuzuri de kakarete ite mo torinozokaremasu. Bunsho ga md2html ni, sono foruda no soto no fairu o yomaseru koto wa dekimasen. Mata, donna ni akui no aru nyūryoku de mo, md2html o tomeru koto mo, kurasshu saseru koto mo dekimasen.

> [!CAUTION]
> `--html` wa, yori yutaka na HTML (gazō, rinku, bideo, SVG, reiauto) o tōshimasu ga, jikkō sareru mono wa kesshite tōshimasen: sukuriputo mo, ibento handora mo, furēmu mo, fōmu mo tōrimasen.

macOS to Ubuntu de, henkō no tabi ni kensa sareru koto:

- pēji zentai o henkan shite yomimodosu 53 no tesuto. Sekyuriti no kēsu mo fukumimasu;
- CommonMark no shiyō no 652 no rei. Tsūka suru kazu wa fueru koto wa atte mo, kesshite herimasen;
- clippy no mottomo kibishii rinto to, tsūru ga birudo dekiru to yakusoku shite iru mottomo furui Rust;
- WCAG 2.2 AA o kijun ni axe-core de okonau akusesibiriti kansa. Ryōhō no tēma to sumaho no haba de okonaimasu;
- Chrome de subete no sōsa o tsukau kurikku tesuto: kībōdo to mausu de no kensaku, tēma, moji no ōkisa, mokuji, zuhan, kyakuchū, yōgoshū, kopī, insatsu.

Insatsu ni wa sennyō no reiauto ga arimasu: sōsa botan wa kie, subete no rinku wa adoresu mo insatsu sare, pēji no sakaime de kirete shimau mono wa naku, kaku pēji ni wa taitoru to pēji bangō ga tsukimasu.

Dono pēji mo, hikaeme na ichigyō de owarimasu: md2html de tsukutta koto, sono bājon, soshite md2html no ibasho e no rinku desu.

![Onaji pēji no insatsu.](report/print.jpg){width=55%}

## 10. 0.4 ga dekiru made

Hajimari wa chōsa deshita: sugureta Markdown rendarā wa nani o shite iru ka, web no hyōjun wa pēji ni nani o motomete iru ka, soshite kono konpyūta de tsūru ga dō tsukawarete iru ka. Kono saigo no ten ga, mottomo yaku ni tachimashita. 4sons.com no yottsu no pēji no birudo wa, henkan no ato de md2html no shutsuryoku o kakikae, burando, gengo, futtā nado o kaete imashita. Sono kakikae wa sorezore settei ni nari, birudo wa atarashii bājon de pēji goto ni tsukurinaosaremashita.

Henkō o, rendarā ga shitagau rūru no diff to shite shimesu to:

```diff
- a handbook subset, anything else shown as written
+ all of CommonMark, and anything else shown as written
- every quotation a note
+ a quotation is a quotation; a bold first word makes it a note
- the page's words in English
+ the page's words in English or German
```

Sono ato, 0.5 de kuwawatta no wa daiaguramu desu. Koko no dono pēji mo tadoru michi:

```mermaid
flowchart TD
  md[Markdown] --> blocks[Burokku to inrain, CommonMark no yarikata de]
  blocks --> settings[Furonto matā to furaggu]
  settings --> page[Pēji: mokuji, kensaku, tēma, yōgoshū]
  page --> check{--check}
  check -->|naosu tokoro nashi| out([hitotsu no HTML fairu])
  check -->|miru beki gyō| md
```

---

Shita no yōgoshū wa, kono hōkoku de tsukau kotoba o setsumei shimasu. Dore mo, ue de wa tensen no tsuita go deshita.

## Yōgoshū

| Yōgo | Imi | Betsumei |
|---|---|---|
| Markdown | Midashi, kyōchō, risuto, rinku o karui kigō de arawasu purēn tekisuto. Sono mama de yomeru shi, pēji ni henkan mo dekimasu. | |
| CommonMark | Ima no ōku no tsūru ga shitagau Markdown no shiyō de, subete no rūru ni tesuto ga arimasu. | |
| mokuji | Pēji no hidari ni aru shō no ichiran de, midashi kara tsukuraremasu. | |
| kōruauto | Waku de kakonda sokuchū: memo, hinto, jūyō na koto, keikoku, chūi no ikutsu ka. | arāto |
| zuhan | Sore dake de hitotsu no gyō ni oita gazō de, shita ni kyapushon ga tsuki, kurikku de kakudai saremasu. | |
| furonto matā | Markdown fairu no ichiban ue no, futatsu no `---` no gyō no aida ni aru settei no burokku. | |
| MathML | Web jishin no sūshiki no gengo de, burauza ga sukuriputo nashi de egakimasu. | |
| Content-Security-Policy | Dono kōdo o jikkō shite yoi ka o burauza ni tsutaeru, pēji no naka no rūru. Koko de wa, pēji jishin no kōdo dake desu. | |
| Mermaid | Kōdo burokku ni kaku, daiaguramu no tame no tekisuto gengo. md2html wa sono furōchāto to shīkensu-zu o egakimasu. | |
| WCAG | Web Content Accessibility Guidelines (web kontentsu no akusesibiriti no gaidorain). Reberu AA ga futsū no mokuhyō desu. | |
| hon'yaku | Betsu no gengo de kakareta pēji no han de, genpon no tonari ni aru Markdown fairu desu. | |
