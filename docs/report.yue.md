---
title: md2html 識做嘅所有嘢
description: 一份關於 md2html 0.4 嘅報告。md2html 係 Nio 家族網頁用嚟將 Markdown 轉做 HTML 嘅工具；呢份報告用 Markdown 寫，再由 md2html 自己轉出嚟，所以入面講到嘅每樣功能，都喺呢一頁上面用緊。
brand: md2html
kicker: 報告
version: 0.7.4
date: 2026-11-22
footer: 用 Markdown 寫，由 **md2html 0.7.4** 自己轉出嚟，圖片同所有嘢都喺呢一頁入面。家族網頁，冇公開列出。有問題請問 Robert。
csp: true
embed: true
translated_from: report.md
source_sha256: 75cc16e86c12a30cad6f921612f5dccec439ad24a508119d923850d7f7b15c6c
---

# md2html 識做嘅所有嘢

md2html 會將一個 Markdown 檔案變成一頁乜都唔使再靠嘅 HTML：唔使伺服器，唔使上網，亦唔使由任何地方攞字型或者腳本。呢份報告就係噉樣嘅一頁。佢用 Markdown 寫，由 md2html 轉出嚟，所以佢講嘅每樣功能，都喺佢自己身上運作緊：左邊嘅目錄、搜尋、兩款主題、字體大小、下面嘅卡、詞彙表入面有虛線嘅詞、註腳、可以放大嘅插圖、有顏色嘅程式碼，同排好版嘅數學式。

## 點樣睇呢一頁

- **第 1 章** 用一個表講晒成個故事。兩分鐘就睇完。
- **第 2 至 6 章** 講讀者會見到啲乜，同埋作者要打啲乜先會有。
- **第 7 同 8 章** 係設定同命令列。
- **第 9 同 10 章** 講安全同品質，同埋 0.4 版係點樣嚟。

---

## 1. 一眼睇晒

| | 以前 (0.3) | 而家 (0.4) |
|:--|:--:|:--:|
| 讀得明嘅 Markdown | 手冊用到嘅一部分 | 全部 CommonMark，加埋 GitHub 嘅擴充 |
| 通過嘅 CommonMark 例子 | 冇量度過 | **652 個入面 640 個** |
| 4,265 行嘅 docenta 手冊 | 超過兩分鐘 | **0.07 秒** |
| 一頁滿晒 `[V]` 記號 | 脹到 38 GB | 幾毫秒 |
| 無障礙問題（axe-core，WCAG 2.2 AA） | 幾十個 | 12 頁，淺色、深色同手機都係 **零** |
| 測試 | 10 | 53 個，加埋規格嘅 652 個例子，同埋喺 Chrome 逐個掣撳一次嘅測試 |
| Mermaid 圖表 | 顯示成程式碼 | 唔使腳本就 **畫出嚟** |
| 語言 | 英文 | **17 種**，可以喺譯本之間切換，包括由右至左嘅語言 |
| 留低嘅方式 | 頁面 | 頁面、有書籤嘅 **PDF**、電子書閱讀器用嘅 **EPUB** |
| 執行時嘅依賴 | 冇 | 冇 |

0.4 帶嚟嘅嘢，寫成任務清單：

- [x] 一個完整嘅 Markdown 讀取器，照 CommonMark 規格描述嘅方法整
- [x] 註腳、任務清單、刪除線、自動連結、可以對齊嘅表格，同 GitHub 嘅五種提示框
- [x] 大約二十種語言嘅程式碼上色，數學式用 MathML 排版
- [x] 喺前置資料入面寫設定，同埋德文版嘅頁面字眼
- [x] `--check`、`--watch`、資料夾、stdin，同可以自己揀開唔開嘅 Content-Security-Policy
- [x] 無障礙審核，同埋喺 CI 入面逐個掣撳一次嘅測試
- [x] Mermaid 流程圖同序列圖，唔使腳本就畫成 SVG（第 4 章）
- [x] 呢份報告有十七種語言，工具列有切換掣（第 7 章）
- [x] 頁面可以留低、列印，或者做成 PDF 或 EPUB 帶走（第 2 章同第 8 章）

> [!IMPORTANT]
> 4sons.com 上面所有用 md2html 整嘅網頁（計劃、研究同家族網頁）都用 0.4 重新整過，再同當時上線嘅版本比較：結構一樣，甚至更好。

## 2. 頁面

頁面有三部分：頂部嘅工具列、左邊嘅目錄，同埋正文。

![淺色主題嘅示範頁：目錄標示住正在睇嘅章節，工具列會顯示個章節名。](report/light.jpg){width=100%}

深色主題會跟系統，或者跟讀者自己揀，而且會記住：

![同一頁嘅深色主題，去到程式碼同數學式嗰度。](report/dark-code.jpg){width=100%}

喺手機上面，目錄會收埋喺工具列個掣後面，冇嘢要左右碌：

![手機闊度嘅頁面。](report/phone.jpg){width=34%}

讀者可以做嘅嘢：

| 想 | 咁做 |
|:--|:--|
| 搵嘢 | 工具列嘅 **搵**、<kbd>⌘</kbd> <kbd>K</kbd>（<kbd>Ctrl</kbd> <kbd>K</kbd>），或者 <kbd>/</kbd> |
| 收埋或者顯示目錄 | 工具列左邊嘅掣，或者 <kbd>⌘</kbd> <kbd>\\</kbd> |
| 改字體大小 | 工具列嘅 **Aa**，再撳 **−** 同 **+**；或者 <kbd>⌘</kbd> <kbd>+</kbd> 同 <kbd>⌘</kbd> <kbd>−</kbd>，由 85% 到 200% |
| 轉主題 | **Aa**，再揀跟系統、淺色或者深色 |
| 放大插圖 | 撳一下；再撳兩下會再近啲；<kbd>Esc</kbd> 就還原 |
| 睇註腳或者有虛線嘅詞 | 將滑鼠停喺上面；撳一下就跳過去，撳 **返回** 就返嚟 |
| 分享一章或者一段 | 標題隔籬嘅 **#** 會複製佢嘅連結（手機就會打開分享選單）；揀幾個字，就會俾你一條啱啱好連去嗰幾個字嘅連結 |
| 留低 | 工具列尾嘅下載掣：呢一頁本身（離線都睇得）、列印或者存做 PDF，或者有整到嘅話就係 PDF 同 EPUB |

## 3. 寫法

普通嘅 Markdown，而每種寫法當純文字睇都一樣好睇。先喺程式碼區塊顯示你寫嘅嘢，跟住就係你得到嘅結果。

### 強調同連結

```markdown
**粗體**、*斜體*、***兩樣都有***、~~刪除~~ 同 `程式碼`。
一個[連結](https://commonmark.org "CommonMark")，一個直接寫出嚟嘅網址
https://spec.commonmark.org/0.31.2/ ，仲有好似 [V] 噉嘅記號會照原樣保留。
```

**粗體**、*斜體*、***兩樣都有***、~~刪除~~ 同 `程式碼`。
一個[連結](https://commonmark.org "CommonMark")，一個直接寫出嚟嘅網址
https://spec.commonmark.org/0.31.2/ ，仲有好似 [V] 噉嘅記號會照原樣保留。

### 清單入面嘅清單

```markdown
1. 第一項，入面有個清單：
   - 一點
   - 另一點，有自己嘅清單：
     1. 再深一層
2. 第二項。
```

1. 第一項，入面有個清單：
   - 一點
   - 另一點，有自己嘅清單：
     1. 再深一層
2. 第二項。

### 引文同提示框

```markdown
> 普通嘅引文仍然係引文。

> [!TIP]
> GitHub 嘅提示框用得：NOTE、TIP、IMPORTANT、WARNING、CAUTION。

> **說明:** 舊式寫法都得，即係一個粗體字加一個冒號。
```

> 普通嘅引文仍然係引文。

> [!TIP]
> GitHub 嘅提示框用得：NOTE、TIP、IMPORTANT、WARNING、CAUTION。

> **說明:** 舊式寫法都得，即係一個粗體字加一個冒號。

### 表格

```markdown
| 房間   | 樓層   | 面積（平方呎） |
|:-------|:------:|--------------:|
| 客廳   | 地下   |           312 |
| 閣樓   | 樓上   |           205 |
```

| 房間 | 樓層 | 面積（平方呎） |
|:-------|:------:|--------------:|
| 客廳 | 地下 | 312 |
| 閣樓 | 樓上 | 205 |

### 註腳

```markdown
一個需要出處嘅說法。[^1]

[^1]: 出處。滑鼠停喺個數字上面，就可以喺嗰度睇到。
```

一個需要出處嘅說法。[^source]

[^source]: 出處。滑鼠停喺個數字上面，就可以喺嗰度睇到；撳佢就會嚟到呢度，撳返回就返去。

### 插圖同圖片

```markdown
![圖片下面嘅說明文字。](report/zoom.jpg){width=60%}

一個圖示 ![標誌](mark.svg){width=18} 喺句子中間。
```

![一張佔欄闊 60% 嘅插圖。撳一下會填滿成個視窗，再撳兩下會再近啲。](report/zoom.jpg){width=60%}

一個圖示 ![標誌](mark.svg){width=18} 喺句子中間。

### 細節

```markdown
撳 <kbd>Ctrl</kbd>+<kbd>K</kbd>。水係 H<sub>2</sub>O，間房 9 m<sup>2</sup>。
一行用反斜線結尾\
就會喺嗰度斷行。
```

撳 <kbd>Ctrl</kbd>+<kbd>K</kbd>。水係 H<sub>2</sub>O，間房 9 m<sup>2</sup>。
一行用反斜線結尾\
就會喺嗰度斷行。

<details>
<summary>撳一下先會打開嘅一節</summary>

用 `<details>` 加 `<summary>` 寫，入面係普通 Markdown：**粗體**、一個清單，

- 一
- 二

</details>

#### 第四級標題

第三級以下嘅標題唔會出現喺目錄，好似呢個噉。

## 4. 程式碼同數學式

程式碼會為 Rust、Python、JavaScript 同 TypeScript、Go、C 同 C++、Java、Kotlin、Swift、C#、shell、SQL、Ruby、Lua、PHP、CSS、HTML、JSON、YAML、TOML、Dockerfile 同 diff 上色，每個區塊都有複製掣。

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

數學式由瀏覽器自己用 MathML 排版，唔使腳本，亦唔使下載字型。喺句子入面，畢氏定理係 $a^2 + b^2 = c^2$ ，歐拉恆等式係 $e^{i\pi} + 1 = 0$ ；好似 $5 或者 $10 噉嘅金額仍然係錢。獨立一行顯示：

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

### 圖表

Mermaid 流程圖或者序列圖由 md2html 自己畫，變成頁面入面嘅 SVG：唔使腳本，唔使下載任何嘢，兩款主題都用頁面自己嘅顏色，而圖入面嘅字仍然係字，搜尋搵得到。Markdown 係噉寫：

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[讀 Markdown]
  parse --> render{係唔係圖表？}
  render -->|流程圖或者序列圖| svg[畫成 SVG]
  render -->|其他種類| js[mermaid.js，如果有提供]
  svg --> page([一頁 HTML])
  js --> page
```
````

跟住就會變成噉：

```mermaid
flowchart LR
  md[notes.md] --> parse[讀 Markdown]
  parse --> render{係唔係圖表？}
  render -->|流程圖或者序列圖| svg[畫成 SVG]
  render -->|其他種類| js[mermaid.js，如果有提供]
  svg --> page([一頁 HTML])
  js --> page
```

一個有編號、有備註嘅序列圖：

```mermaid
sequenceDiagram
  autonumber
  actor R as 讀者
  participant P as 頁面
  participant S as 搜尋
  R->>P: Cmd K
  P->>S: 打開搜尋
  R->>S: 輸入 "callout"
  S-->>R: 相關段落，最啱嘅排先
  R->>S: Enter
  S-->>P: 跳過去，再將啲字標亮
  Note over R,P: 唔使網絡，唔使伺服器
```

其他種類（圓餅圖、甘特圖、類別圖）如果作者將 md2html 指向一份 mermaid.js（`--mermaid-js FILE`），就會由 mermaid.js 畫；個腳本只會放入嗰一頁。冇嘅話，佢哋會保持程式碼，`--check` 亦會講出嚟。

## 5. 詞彙表同註腳

一個叫「詞彙表」嘅章節，用表格列出呢頁嘅詞。佢唔會喺原本嘅位置顯示出嚟：反而每個詞喺每一章第一次出現嘅地方，都會變成有虛線嘅詞，滑鼠停喺上面就會解釋自己，撳一下就跳去頁底嘅條目，條目再帶你返去你啱啱睇緊嗰行。

![滑鼠停喺有虛線嘅詞上面，就會顯示佢嘅意思。](report/glossary.jpg){width=70%}

註腳都係一樣：滑鼠停喺個數字上面，就會顯示解釋。

![滑鼠停喺註腳嘅數字上面，就會顯示個註。](report/footnote.jpg){width=70%}

> [!NOTE]
> 詞彙表可以喺唔同頁面之間共用：喺 `~/.config/md2html/glossary.md` 放一個檔案，入面嘅詞就會喺嗰部機轉出嚟嘅每一頁都變成連結。

## 6. 搜尋同插圖

搜尋包括頁面上每一個字，連詞彙表都計埋。結果會顯示每個字喺邊度；揀咗嗰個，會喺搜尋跳到嘅位置標亮。

![搜尋「callout」：每個結果嘅章節同段落。](report/search.jpg){width=80%}

![搜尋跳到嘅位置：條目會發光，搵到嘅字會標亮。](report/search-landed.jpg){width=80%}

插圖會喺頁面上面放大，唔使離開頁面。如果比視窗仲大，佢會跟住滑鼠指標郁，你指住想睇嘅地方就得。

## 7. 設定

呢份報告英文原文嘅 Markdown 檔案，開頭係噉寫：

```yaml
---
title: md2html, everything it does
brand: md2html
kicker: Report
version: 0.7.4
date: 2026-11-22
footer: Written in Markdown and rendered by **md2html 0.7.4** itself ...
csp: true      # only this page's own code may run
embed: true    # the screenshots are inside the page
---
```

| 設定 | 作用 |
|:--|:--|
| `title`、`description`、`author` | 標題，同埋連結預覽用嘅 meta 標籤 |
| `lang` | 語言：頁面自己嘅字眼同日期，可以係上面十七種語言入面任何一種（`ja-Hira` 係淨係用假名寫嘅日文，`zh-Hant` 係繁體字中文，`yue` 係粵語） |
| `translated_from`、`source_sha256` | 用喺譯本：佢嘅原文，同埋翻譯嗰陣原文嘅指紋 |
| `brand`、`brand_link`、`kicker`、`eyebrow`、`footer` | 工具列、標題上面嗰行、頁底嘅細字 |
| `date`、`version` | 標題上面顯示嘅日期同版本 |
| `embed` | 將圖片放入頁面，等成頁只係一個檔案 |
| `csp` | 一個 Content-Security-Policy |
| `pdf`、`epub` | 喺頁面隔籬整埋佢嘅 PDF 同 EPUB |
| `live` | 頁面嘅有效期（`true` 係兩分鐘，或者一個秒數）：到期之後如果頁面改咗就重新載入，讀者睇到邊度都留喺度 |
| `large`、`toc`、`math`、`index`、`html`、`css`、`head` | 大啲嘅字、目錄、數學式、搜尋引擎、原始 HTML、額外樣式同 head |

> [!TIP]
> 用 `lang: de` 嘅話，搜尋會寫「Diese Seite durchsuchen」，提示框會寫「Hinweis」同「Achtung」，日期會係「22. November 2026」。呢個網站上面 Thomas 嘅網頁就係噉變成德文。

### 語言

呢份報告有十七種語言：英文、德文、希伯來文、印尼文、荷蘭文、爪哇文、意第緒文、低地德文、法文、意大利文、西班牙文、三種寫法嘅日文（漢字加假名、淨係假名、羅馬字）、簡體字同繁體字嘅普通話中文，同埋粵語。工具列入面嘅地球儀，可以喺佢哋之間切換，而且頁面會喺你睇緊嗰章打開。

譯本其實只係原文旁邊另一個 Markdown 檔案，用佢嘅語言命名：`report.de.md` 就係德文嘅 `report.md`。md2html 會搵出嗰度有嘅版本，將佢哋互相連結（亦會用 `hreflang` 話俾瀏覽器同搜尋引擎知），再俾每個版本用返自己語言嘅介面字眼：搜尋、提示框、詞彙表、日期。希伯來文同意第緒文會由右至左排版，目錄喺右邊。

每份譯本都會寫明佢嘅原文，並記錄原文嘅指紋，所以頁面可以講明邊個先係正式版本；而原文喺翻譯之後有改動嘅話，`--check` 會察覺到：

```yaml
translated_from: report.md
source_sha256: 5b0c...e1
```

如果一種語言未有人翻譯過，讀者自己嘅瀏覽器有呢個功能嘅話（Chrome 內置嘅翻譯功能），就可以幫手翻譯：喺部裝置本身做，所以家族網頁嘅內容一啲都唔會離開部裝置，而且會標明係機器翻譯，亦可以返去原文。

## 8. 命令列

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

`--check` 唔會寫出任何檔案，只會逐行列出作者要睇嘅地方，有嘢嘅話就以 1 結束：一個連去頁面上冇嘅錨點嘅連結、一個唔存在嘅檔案、一個冇註文嘅註腳、顯示成文字嘅原始 HTML。所以佢啱做 build 嘅最後一步，或者代理程式工作嘅最後一步。`--watch` 每次儲存都會重新轉換，`--nav` 用上一頁同下一頁將一個資料夾入面嘅頁面連埋一齊，而連去另一個 Markdown 檔案嘅連結會去到佢嘅頁面。

`--pdf` 用部電腦本身已經有嘅 Chrome 列印頁面：用頁面自己嘅列印版面，標題變成 PDF 嘅書籤，每頁頂有標題，底有頁數。`--epub` 用同一頁整一本電子書閱讀器用嘅書，有目錄、圖片、圖表同數學式，註腳喺提到佢嘅地方打開；EPUB 係 md2html 自己寫出嚟嘅。讀者喺工具列嘅下載掣入面兩樣都搵到。

`--serve` 喺寫緊嘅時候就喺呢部電腦度顯示啲頁面：Markdown 一儲存，每個打開咗嘅頁面即刻重新載入。用 `--live` 發佈嘅頁面有自己嘅有效期；到期嗰陣，佢會問自己個伺服器攞自己開頭嗰 1 KB，淨係嗰部分改咗先重新載入。無論邊種，讀者都留喺原本嘅位置：同一段喺同一個高度；如果嗰段冇咗，就去返佢前面嗰章。

## 9. 安全、品質同無障礙

> [!WARNING]
> 頁面可能係由其他人寫嘅 Markdown 轉出嚟。md2html 永遠唔會由內文寫出腳本：除咗幾個無害嘅標籤，原始 HTML 會照寫法顯示，而 `javascript:` 或者 `data:` 連結無論點拼都會被刪走。份文件冇辦法令 md2html 讀佢個資料夾以外嘅檔案；無論幾惡意嘅輸入，都冇辦法令佢卡死或者當機。

> [!CAUTION]
> `--html` 會俾更豐富嘅 HTML 通過（圖片、連結、影片、SVG、版面），但係任何會執行嘅嘢都唔會俾過：冇腳本、冇事件處理器、冇框架、冇表單。

每次有改動，都會喺 macOS 同 Ubuntu 上面檢查：

- 53 個測試，會轉出成頁再讀返，包括安全方面嘅情況；
- CommonMark 規格嘅 652 個例子，通過嘅數目可以升，但永遠唔可以跌；
- clippy 最嚴格嘅 lint，同埋呢個工具承諾可以用嚟 build 嘅最舊 Rust 版本；
- 用 axe-core 按 WCAG 2.2 AA 做嘅無障礙審核，兩款主題同手機闊度都做；
- 喺 Chrome 逐個掣撳嘅測試，用晒每個控制項：用鍵盤同滑鼠搜尋、主題、字體大小、目錄、插圖、註腳、詞彙表、複製、列印。

列印有自己嘅版面：控制掣會消失，每條連結都會印埋佢嘅網址，冇嘢會喺跨頁嘅地方被切開，而且每頁都有標題同頁數。

每一頁最尾都有一行唔起眼嘅字：用 md2html 整、佢嘅版本，同埋一條連去 md2html 所在地方嘅連結。

![同一頁嘅列印版。](report/print.jpg){width=55%}

## 10. 0.4 係點樣嚟

一開始係做研究：最好嘅 Markdown 轉換器做咗啲乜，網頁標準對一頁有乜要求，同埋喺呢部機上面呢個工具係點用；最後呢樣原來最有用。4sons.com 上面有四個網頁 build，喺轉換之後會改寫 md2html 嘅輸出，用嚟改品牌、語言、頁尾等等。每一種改寫都變成咗一個設定，而嗰啲 build 亦用新版本逐頁重新整過。

啲改動，寫成轉換器遵守嘅規則嘅 diff：

```diff
- a handbook subset, anything else shown as written
+ all of CommonMark, and anything else shown as written
- every quotation a note
+ a quotation is a quotation; a bold first word makes it a note
- the page's words in English
+ the page's words in English or German
```

之後 0.5 再加咗：圖表。呢度每一頁行嘅路：

```mermaid
flowchart TD
  md[Markdown] --> blocks[區塊同行內元素，照 CommonMark 嘅做法]
  blocks --> settings[前置資料同參數]
  settings --> page[頁面：目錄、搜尋、主題、詞彙表]
  page --> check{--check}
  check -->|冇嘢要改| out([一個 HTML 檔案])
  check -->|要睇嘅行| md
```

---

下面嘅詞彙表解釋呢份報告用到嘅詞；每個詞喺上面都係有虛線嘅詞。

## 詞彙表

| 詞彙 | 意思 | 又稱 |
|---|---|---|
| Markdown | 用簡單記號標示標題、強調、清單同連結嘅純文字，照原樣都睇得明，亦可以轉成網頁。 | |
| CommonMark | 而家大部分工具都跟從嘅 Markdown 規格，每條規則都有測試。 | |
| 目錄 | 頁面左邊嘅章節清單，由頁面嘅標題整出嚟。 | |
| 提示框 | 一個加咗框嘅旁注：說明、貼士、重要事項、警告或者注意。 | |
| 插圖 | 自己佔一行嘅圖片，下面有說明文字，撳一下就會放大。 | |
| 前置資料 | Markdown 檔案最頂、兩行 `---` 之間嘅設定區塊。 | |
| MathML | 網頁本身用嚟寫數學嘅語言，由瀏覽器直接畫，唔使腳本。 | |
| Content-Security-Policy | 頁面入面嘅一條規則，話俾瀏覽器知邊啲程式碼可以執行；喺呢度，只有頁面自己嘅。 | |
| Mermaid | 一種用文字畫圖表嘅語言，寫喺程式碼區塊入面；md2html 會畫佢嘅流程圖同序列圖。 | |
| WCAG | Web Content Accessibility Guidelines（網頁內容無障礙指引）；AA 級係通常嘅目標。 | |
| 譯本 | 一頁喺另一種語言嘅版本，係原文旁邊嘅一個 Markdown 檔案。 | |
