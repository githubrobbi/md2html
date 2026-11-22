---
title: md2html 的全部功能
description: 一份关于 md2html 0.4 的报告。md2html 是 Nio 家庭页面所用的 Markdown 转 HTML 渲染器；本报告用 Markdown 写成，并由 md2html 自己渲染，因此它描述的每项功能也都应用在本页上。
brand: md2html
kicker: 报告
version: 0.7.2
date: 2026-11-22
footer: 用 Markdown 写成，由 **md2html 0.7.2** 自己渲染，图片等一切都在这一个页面里。家庭页面，未公开列出。如有问题，请找 Robert。
csp: true
embed: true
translated_from: report.md
source_sha256: 18c6eb0683573fb2441f67d53c0f697161a03934fbea849ecc1e20af791cc107
---

# md2html 的全部功能

md2html 把一个 Markdown 文件变成一个不依赖任何其他东西的 HTML 页面：不需要服务器，不需要网络，也不从任何地方加载字体或脚本。本报告就是这样一个页面。它用 Markdown 写成，由 md2html 渲染，所以它描述的每项功能也都在它身上起作用：左边的目录、搜索、两种主题、字号、下面的卡片、术语表里带虚线的词、脚注、可以放大的插图、着色的代码和排版好的数学公式。

## 如何阅读本页

- **第 1 章**用一张表讲完全部内容。两分钟。
- **第 2 至 6 章**展示读者能得到什么，以及作者要输入什么才能得到它。
- **第 7 和 8 章**是设置和命令行。
- **第 9 和 10 章**讲安全与质量，以及 0.4 版是怎么来的。

---

## 1. 概览

| | 之前（0.3） | 现在（0.4） |
|:--|:--:|:--:|
| 能读取的 Markdown | 手册所需的一个子集 | 完整的 CommonMark，以及 GitHub 的扩展 |
| 通过的 CommonMark 示例 | 未测量 | **652 个中的 640 个** |
| 4,265 行的 docenta 手册 | 两分多钟 | **0.07 秒** |
| 满是 `[V]` 标记的页面 | 膨胀到 38 GB | 几毫秒 |
| 无障碍检查发现的问题（axe-core，WCAG 2.2 AA） | 几十个 | **没有**，涵盖 12 个页面，浅色、深色和手机 |
| 测试 | 10 个 | 53 个，外加 652 个规范示例和一次在 Chrome 中的逐项点击测试 |
| Mermaid 图表 | 显示为代码 | **绘制出来**，无需脚本 |
| 语言 | 英语 | **17 种**，可在译文之间切换，包括从右到左书写的语言 |
| 保存形式 | 页面 | 页面、带书签的 **PDF**、适用于电子阅读器的 **EPUB** |
| 运行时依赖 | 无 | 无 |

0.4 带来的内容，以任务列表的形式列出：

- [x] 完整的 Markdown 解析器，按照 CommonMark 规范描述的方式构建
- [x] 脚注、任务列表、删除线、自动链接、对齐的表格，以及 GitHub 的五种警报
- [x] 约二十种语言的代码着色，数学公式排版为 MathML
- [x] 前置元数据中的设置，以及德语版的页面自身文字
- [x] `--check`、`--watch`、文件夹、标准输入、可选启用的 Content-Security-Policy
- [x] 无障碍审核，以及 CI 中的逐项点击测试
- [x] Mermaid 流程图和时序图，绘制为 SVG，无需脚本（第 4 章）
- [x] 本报告的十七种语言版本，顶栏中有切换开关（第 7 章）
- [x] 页面可以保存、打印，或以 PDF 或 EPUB 的形式带走（第 2 章和第 8 章）

> [!IMPORTANT]
> 4sons.com 上所有用 md2html 构建的页面（计划、研究和家庭页面）都用 0.4 重新构建，并与当时线上的版本做了比较：结构相同，或者更好。

## 2. 页面

页面分为三部分：顶部的一条栏、左边的目录，以及正文。

![浅色主题下的展示页面：目录标出正在阅读的章节，顶栏显示它的名称。](report/light.jpg){width=100%}

深色主题跟随系统，或者跟随读者的选择，并且会被记住：

![同一页面的深色主题，停在代码和数学公式处。](report/dark-code.jpg){width=100%}

在手机上，目录收进顶栏的开关后面，页面也不会左右滚动：

![手机宽度下的页面。](report/phone.jpg){width=34%}

读者可以做的事：

| 想要 | 这样做 |
|:--|:--|
| 查找内容 | 顶栏中的**搜索**，<kbd>⌘</kbd> <kbd>K</kbd>（<kbd>Ctrl</kbd> <kbd>K</kbd>），或 <kbd>/</kbd> |
| 隐藏或显示目录 | 顶栏左边的开关，或 <kbd>⌘</kbd> <kbd>\\</kbd> |
| 调整字号 | 顶栏中的 **Aa**，然后 **−** 和 **+**；或 <kbd>⌘</kbd> <kbd>+</kbd> 和 <kbd>⌘</kbd> <kbd>−</kbd>，范围从 85% 到 200% |
| 切换主题 | **Aa**，然后选择跟随系统、浅色或深色 |
| 放大插图 | 点击它；再点两次会拉得更近；<kbd>Esc</kbd> 让它复原 |
| 阅读脚注或带虚线的词 | 把鼠标停在上面；点击则跳过去，**返回**会带你回来 |
| 分享一章或一段文字 | 标题旁的 **#** 会复制它的链接（在手机上会打开分享面板）；选中一些文字，就会提供一个恰好指向它们的链接 |
| 保存 | 顶栏末端的下载按钮：页面本身（可离线阅读）、打印或另存为 PDF，或者已生成的 PDF 和 EPUB |

## 3. 写作

就是普通的 Markdown，而且每种写法作为纯文本读起来也很顺。先在代码块中给出你写的内容，紧随其后的是你得到的结果。

### 强调与链接

```markdown
**粗体**、*斜体*、***两者兼有***、~~删除线~~和`代码`。
一个[链接](https://commonmark.org "CommonMark")，裸链接
https://spec.commonmark.org/0.31.2/ 也可以，还有像 [V] 这样保持原样的标记。
```

**粗体**、*斜体*、***两者兼有***、~~删除线~~和`代码`。一个[链接](https://commonmark.org "CommonMark")，裸链接 https://spec.commonmark.org/0.31.2/ 也可以，还有像 [V] 这样保持原样的标记。

### 列表中的列表

```markdown
1. 第一项，带一个嵌套列表：
   - 一个项目
   - 另一个，带有它自己的列表：
     1. 更深一层
2. 第二项。
```

1. 第一项，带一个嵌套列表：
   - 一个项目
   - 另一个，带有它自己的列表：
     1. 更深一层
2. 第二项。

### 引用与标注

```markdown
> 普通的引用仍然是引用。

> [!TIP]
> GitHub 的警报都能用：NOTE、TIP、IMPORTANT、WARNING、CAUTION。

> **说明:** 旧的写法也一样可以：一个粗体词加一个冒号。
```

> 普通的引用仍然是引用。

> [!TIP]
> GitHub 的警报都能用：NOTE、TIP、IMPORTANT、WARNING、CAUTION。

> **说明:** 旧的写法也一样可以：一个粗体词加一个冒号。

### 表格

```markdown
| 房间 | 楼层 | 面积（平方英尺） |
|:-----|:----:|-----------------:|
| 客厅 | 一楼 |              312 |
| 阁楼 | 楼上 |              205 |
```

| 房间 | 楼层 | 面积（平方英尺） |
|:-------|:------:|-------------:|
| 客厅 | 一楼 | 312 |
| 阁楼 | 楼上 | 205 |

### 脚注

```markdown
一个需要出处的说法。[^1]

[^1]: 出处。把鼠标停在编号上，就能在原处读到它。
```

一个需要出处的说法。[^source]

[^source]: 出处。把鼠标停在编号上，就能在原处读到它；点击编号会来到这里，点击**返回**则回到原处。

### 插图与图片

```markdown
![图片下方的说明文字。](report/zoom.jpg){width=60%}

一个图标 ![标志](mark.svg){width=18} 嵌在句子当中。
```

![一张占栏宽 60% 的插图。点击它：它会充满窗口，再点两次会拉得更近。](report/zoom.jpg){width=60%}

一个图标 ![标志](mark.svg){width=18} 嵌在句子当中。

### 小细节

```markdown
按 <kbd>Ctrl</kbd>+<kbd>K</kbd>。水是 H<sub>2</sub>O，房间 9 m<sup>2</sup>。
以反斜杠结尾的一行\
会在那里换行。
```

按 <kbd>Ctrl</kbd>+<kbd>K</kbd>。水是 H<sub>2</sub>O，房间 9 m<sup>2</sup>。以反斜杠结尾的一行\
会在那里换行。

<details>
<summary>点击即可展开的一节</summary>

用 `<details>` 加一个 `<summary>` 写成，里面是普通的 Markdown：**粗体**、一个列表，

- 一
- 二

</details>

#### 第四级标题

第三级以下的标题不会进入目录，这个标题就是如此。

## 4. 代码与数学公式

代码会按 Rust、Python、JavaScript 和 TypeScript、Go、C 和 C++、Java、Kotlin、Swift、C#、shell、SQL、Ruby、Lua、PHP、CSS、HTML、JSON、YAML、TOML、Dockerfile 和 diff 着色，每个代码块都有一个复制按钮。

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

数学公式由浏览器自己排版，使用 MathML，不需要脚本，也不需要下载字体。行内公式：勾股定理是 $a^2 + b^2 = c^2$，欧拉恒等式是 $e^{i\pi} + 1 = 0$；而像 $5 或 $10 这样的金额仍然是钱。独立显示的公式：

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

### 图表

Mermaid 流程图或时序图由 md2html 自己绘制，以 SVG 的形式放在页面里：没有脚本，无需下载任何东西，两种主题下都用页面自己的颜色，图中的文字仍然是文字，搜索可以找到。下面是 Markdown 写法：

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[读取 Markdown]
  parse --> render{是图表吗？}
  render -->|流程图或时序图| svg[绘制为 SVG]
  render -->|其他类型| js[mermaid.js（如已提供）]
  svg --> page([一个 HTML 页面])
  js --> page
```
````

它会变成这样：

```mermaid
flowchart LR
  md[notes.md] --> parse[读取 Markdown]
  parse --> render{是图表吗？}
  render -->|流程图或时序图| svg[绘制为 SVG]
  render -->|其他类型| js[mermaid.js（如已提供）]
  svg --> page([一个 HTML 页面])
  js --> page
```

一张带编号和注释的时序图：

```mermaid
sequenceDiagram
  autonumber
  actor R as 读者
  participant P as 页面
  participant S as 搜索
  R->>P: Cmd K
  P->>S: 打开搜索
  R->>S: 输入 "callout"
  S-->>R: 匹配的段落，最相关的在前
  R->>S: Enter
  S-->>P: 跳转过去，并点亮这些词
  Note over R,P: 无需网络，无需服务器
```

其他类型（饼图、甘特图、类图）由 mermaid.js 绘制，前提是作者让 md2html 指向它的一份副本（`--mermaid-js FILE`）；这个脚本只会进入那一个页面。没有它，这些图仍以代码显示，`--check` 也会指出这一点。

## 5. 术语表与脚注

名为“术语表”的一章把页面用到的词列成一张表。它不会在原处显示；取而代之的是，每章中每个术语第一次出现时都会变成带虚线的词，鼠标停在上面就会显示解释，点击它则跳到页面底部的条目，那里又能带你回到正在读的那一行。

![鼠标停在带虚线的词上，会显示它的含义。](report/glossary.jpg){width=70%}

脚注也是这样：鼠标停在编号上，就会显示注释内容。

![鼠标停在脚注编号上，会显示该注释。](report/footnote.jpg){width=70%}

> [!NOTE]
> 术语表可以在多个页面之间共享：放在 `~/.config/md2html/glossary.md` 的一个文件，其中的词会在这台机器上渲染的每个页面中被链接。

## 6. 搜索与插图

搜索覆盖页面上的每一个词，包括术语表。结果会显示每个词所在的位置；选中的那一条会在搜索跳转到的地方被点亮。

![搜索“callout”：每条结果所在的章节和段落。](report/search.jpg){width=80%}

![搜索跳转到的位置：条目发出微光，找到的词被点亮。](report/search-landed.jpg){width=80%}

插图在页面上方放大，不会离开页面。如果它比窗口还大，就会跟随指针移动，所以你指向哪里，就能看到哪里。

## 7. 设置

本报告英文原文的 Markdown 文件开头是这样的：

```yaml
---
title: md2html, everything it does
brand: md2html
kicker: Report
version: 0.7.2
date: 2026-11-22
footer: Written in Markdown and rendered by **md2html 0.7.2** itself ...
csp: true      # only this page's own code may run
embed: true    # the screenshots are inside the page
---
```

| 设置 | 作用 |
|:--|:--|
| `title`、`description`、`author` | 标题，以及用于链接预览的 meta 标签 |
| `lang` | 语言：页面自身的文字和日期，可为上面十七种语言中的任何一种（`ja-Hira` 是只用假名书写的日语，`zh-Hant` 是繁体字中文，`yue` 是粤语） |
| `translated_from`、`source_sha256` | 用于译文：它的原文，以及翻译时原文的指纹 |
| `brand`、`brand_link`、`kicker`、`eyebrow`、`footer` | 顶栏、标题上方的那一行、页脚的小字 |
| `date`、`version` | 标题上方显示的日期和版本 |
| `embed` | 把图片放进页面里，使它成为一个文件 |
| `csp` | 一条 Content-Security-Policy |
| `pdf`、`epub` | 在页面旁边生成它的 PDF 和 EPUB |
| `live` | 页面的有效期（`true` 为两分钟，也可以是秒数）：到期后若页面已改变就重新加载，并保留读者的位置 |
| `large`、`toc`、`math`、`index`、`html`、`css`、`head` | 更大的字号、目录、数学公式、搜索引擎、原始 HTML、额外的样式和 head |

> [!TIP]
> 设置 `lang: de` 后，搜索框显示“Diese Seite durchsuchen”，标注显示“Hinweis”和“Achtung”，日期显示为“22. November 2026”。本站的 Thomas 页面就是这样以德语呈现的。

### 语言

本报告有十七种语言版本：英语、德语、希伯来语、印尼语、荷兰语、爪哇语、意第绪语、低地德语、法语、意大利语、西班牙语、三种写法的日语（汉字加假名、只用假名、罗马字）、简体字和繁体字的普通话中文，以及粤语。顶栏中的地球图标可以在它们之间切换，页面会打开到你正在读的那一章。

译文就是原文旁边的另一个 Markdown 文件，以其语言命名：`report.de.md` 就是德语版的 `report.md`。md2html 会找出现有的各个版本，把它们相互链接起来（并通过 `hreflang` 告诉浏览器和搜索引擎），还为每个版本配上它自己界面的文字：搜索、标注、术语表、日期。希伯来语和意第绪语从右到左排版，目录在右边。

每个译文都注明它的原文，并记录原文的指纹，因此页面可以说明哪个版本具有权威性；如果原文在翻译之后有改动，`--check` 也会发现：

```yaml
translated_from: report.md
source_sha256: 5b0c...e1
```

对于还没有人把页面翻译成的语言，读者自己的浏览器可以代为翻译，前提是它具备这项能力（例如 Chrome 的内置翻译器）：翻译在设备本身上完成，家庭页面的任何内容都不会离开这台设备，并且会标明这是机器翻译，同时提供返回原文的方式。

## 8. 命令行

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

`--check` 不写入任何东西，而是逐行列出作者应该查看的地方，只要有任何问题就以 1 退出：指向页面上不存在的锚点的链接、不存在的文件、缺少注释内容的脚注、被当作文本显示的原始 HTML。因此它适合作为构建的最后一步，或者代理程序工作的最后一步。`--watch` 在每次保存时重新渲染，`--nav` 用“上一页”和“下一页”把一个文件夹中的页面链接起来，而指向另一个 Markdown 文件的链接会跳到它的页面。

`--pdf` 用电脑上已有的 Chrome 打印页面：使用页面自己的打印版式，标题成为 PDF 的书签，每页顶部是标题，底部是页码。`--epub` 用同一个页面生成一本适用于电子阅读器的书，包含目录、图片、图表和数学公式，脚注会在提到它们的地方打开；EPUB 由 md2html 自己写出。读者在顶栏的下载按钮下面可以找到这两者。

`--serve` 在页面编写期间就在这台电脑上显示它们：Markdown 一保存，每个打开的页面都会立即重新加载。用 `--live` 发布的页面有自己的有效期；到期时，它向自己的服务器请求自身开头的 1 KB，只有这部分改变了才重新加载。无论哪种方式，读者都停留在原来的位置：同一段落位于同样的高度；如果该段落已经不在了，则回到它之前的那一章。

## 9. 安全、质量与无障碍

> [!WARNING]
> 页面可能是从别人写的 Markdown 渲染出来的。md2html 绝不会从文本中生成脚本：原始 HTML 会按原样显示（少数无害的标签除外），而 `javascript:` 或 `data:` 链接无论怎样拼写都会被丢弃。文档无法让 md2html 读取它所在文件夹之外的文件；任何输入，无论多么恶意，都无法让它卡住或崩溃。

> [!CAUTION]
> `--html` 会让更丰富的 HTML 通过（图片、链接、视频、SVG、布局），但绝不会放过任何能运行的东西：没有脚本、没有事件处理程序、没有框架、没有表单。

每次改动时，都会在 macOS 和 Ubuntu 上检查以下各项：

- 53 个测试，渲染完整的页面并读回检查，其中包括安全方面的用例；
- CommonMark 规范的 652 个示例，通过的数量可以上升，但绝不能下降；
- clippy 最严格的 lint，以及本工具承诺支持构建的最旧 Rust 版本；
- 用 axe-core 按 WCAG 2.2 AA 进行的无障碍审核，涵盖两种主题和手机宽度；
- 在 Chrome 中的一次逐项点击测试，用到每一个控件：用键盘和鼠标搜索、主题、字号、目录、插图、脚注、术语表、复制、打印。

打印有自己的版式：控件隐去，每个链接都打印出地址，没有任何内容被分页截断，而且每页都带有标题和页码。

每个页面都以一行不起眼的文字结尾：由 md2html 制作、它的版本号，以及一个指向 md2html 所在之处的链接。

![同一页面的打印效果。](report/print.jpg){width=55%}

## 10. 0.4 是怎么来的

它始于调研：最好的 Markdown 渲染器做了什么，网页标准对页面有什么要求，以及这台机器上是如何使用这个工具的，而最后这一项被证明是最有用的。4sons.com 上有四个页面构建会在渲染之后改写 md2html 的输出，以更改品牌、语言、页脚等内容。每一处这样的改写都变成了一项设置，这些构建也针对新版本逐页重新构建。

这些变化，以渲染器所遵循规则的 diff 来表示：

```diff
- a handbook subset, anything else shown as written
+ all of CommonMark, and anything else shown as written
- every quotation a note
+ a quotation is a quotation; a bold first word makes it a note
- the page's words in English
+ the page's words in English or German
```

此后 0.5 又新增了：图表。这里每个页面所走的路径：

```mermaid
flowchart TD
  md[Markdown] --> blocks[块与行内元素，按 CommonMark 的方式]
  blocks --> settings[前置元数据与命令行参数]
  settings --> page[页面：目录、搜索、主题、术语表]
  page --> check{--check}
  check -->|无需修改| out([一个 HTML 文件])
  check -->|有待查看的行| md
```

---

下面的术语表解释本报告用到的词；每个词在上文中都是带虚线的词。

## 术语表

| 术语 | 释义 | 又称 |
|---|---|---|
| Markdown | 带有少量标记的纯文本，用来表示标题、强调、列表和链接，原样即可阅读，也可以转换成页面。 | |
| CommonMark | 如今大多数工具都遵循的 Markdown 规范，每条规则都有对应的测试。 | |
| 目录 | 页面左边的章节列表，由页面的标题生成。 | |
| 标注 | 一个带框的旁注：说明、提示、重要事项、警告或注意事项。 | 警报 |
| 插图 | 单独占一行的图片，下方附有说明文字，点击即可放大。 | |
| 前置元数据 | Markdown 文件最顶部两条 `---` 线之间的设置块。 | |
| MathML | 网页自己的数学语言，由浏览器绘制，无需脚本。 | |
| Content-Security-Policy | 页面中的一条规则，告诉浏览器哪些代码可以运行；在这里，只有页面自己的代码可以运行。 | |
| Mermaid | 一种用于绘制图表的文本语言，写在代码块中；md2html 会绘制它的流程图和时序图。 | |
| WCAG | 《Web 内容无障碍指南》（Web Content Accessibility Guidelines）；AA 级是通常的目标。 | |
| 译文 | 页面在另一种语言中的版本，即原文旁边的一个 Markdown 文件。 | 译本 |
