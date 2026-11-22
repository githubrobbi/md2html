---
title: md2html, todo lo que hace
description: Un informe sobre md2html 0.4, el conversor de Markdown a HTML de las páginas de la familia Nio, escrito en Markdown y generado por el propio md2html, de modo que cada capacidad que describe se aplica también en la página.
brand: md2html
kicker: Informe
version: 0.7.2
date: 2026-11-22
footer: Escrito en Markdown y generado por el propio **md2html 0.7.2**, con las imágenes y todo dentro de esta única página. Página familiar, no listada públicamente. Las preguntas, a Robert.
csp: true
embed: true
translated_from: report.md
source_sha256: 18c6eb0683573fb2441f67d53c0f697161a03934fbea849ecc1e20af791cc107
---

# md2html, todo lo que hace

md2html convierte un archivo Markdown en una sola página HTML que no necesita nada más: ni servidor, ni red, ni fuentes ni scripts de ninguna parte. Este informe es una de esas páginas. Se escribió en Markdown y lo generó md2html, así que cada capacidad que describe también actúa sobre él: el índice a la izquierda, la búsqueda, los dos temas, el tamaño del texto, las tarjetas de abajo, las palabras punteadas del glosario, las notas al pie, las figuras que se amplían, el código coloreado y las fórmulas matemáticas bien compuestas.

## Cómo leer esto

- **Capítulo 1** es toda la historia en una tabla. Dos minutos.
- **Capítulos 2 a 6** muestran lo que obtiene quien lee y lo que escribe quien redacta para obtenerlo.
- **Capítulos 7 y 8** son los ajustes y la línea de comandos.
- **Capítulos 9 y 10** son la seguridad y la calidad, y cómo surgió la versión 0.4.

---

## 1. De un vistazo

| | Antes (0.3) | Ahora (0.4) |
|:--|:--:|:--:|
| Markdown que lee | un subconjunto para manuales | todo CommonMark, y las extensiones de GitHub |
| Ejemplos de CommonMark que pasan | sin medir | **640 de 652** |
| El manual de docenta, de 4265 líneas | más de dos minutos | **0,07 segundos** |
| Una página llena de marcas `[V]` | crecía hasta 38 GB | milisegundos |
| Hallazgos de accesibilidad (axe-core, WCAG 2.2 AA) | docenas | **ninguno**, en 12 páginas, en claro, en oscuro y en teléfono |
| Pruebas | 10 | 53, más los 652 ejemplos de la especificación y un recorrido de clics en Chrome |
| Diagramas Mermaid | mostrados como código | **dibujados**, sin script |
| Idiomas | inglés | **17**, con un selector entre traducciones, de derecha a izquierda incluido |
| Para conservar | la página | la página, un **PDF** con marcadores, un **EPUB** para lectores electrónicos |
| Dependencias en tiempo de ejecución | ninguna | ninguna |

Lo que llegó con la 0.4, como lista de tareas:

- [x] un lector de Markdown completo, construido tal como lo describe la especificación de CommonMark
- [x] notas al pie, listas de tareas, tachado, enlaces automáticos, tablas alineadas y las cinco alertas de GitHub
- [x] código coloreado en una veintena de lenguajes, matemáticas compuestas como MathML
- [x] ajustes en el front matter, y las palabras propias de la página en alemán
- [x] `--check`, `--watch`, carpetas, stdin, una Content-Security-Policy opcional
- [x] una auditoría de accesibilidad y un recorrido de clics en la CI
- [x] diagramas de flujo y de secuencia de Mermaid, dibujados como SVG sin script (capítulo 4)
- [x] este informe en diecisiete idiomas, con un selector en la barra (capítulo 7)
- [x] la página guardada, impresa o llevada consigo como PDF o EPUB (capítulos 2 y 8)

> [!IMPORTANT]
> Todas las páginas de 4sons.com hechas con md2html (los planes, la investigación y las páginas familiares) se regeneraron con la 0.4 y se compararon con la versión publicada: la misma estructura, o mejor.

## 2. La página

La página tiene tres partes: una barra en la parte superior, el índice a la izquierda y el texto.

![La página de muestra en el tema claro: el índice marca el capítulo que se está leyendo y la barra lo nombra.](report/light.jpg){width=100%}

El tema oscuro sigue al sistema, o a la elección del lector, y se recuerda:

![La misma página en el tema oscuro, a la altura del código y las matemáticas.](report/dark-code.jpg){width=100%}

En un teléfono, el índice se pliega detrás del interruptor de la barra, y nada se desplaza hacia los lados:

![La página con el ancho de un teléfono.](report/phone.jpg){width=34%}

Lo que puede hacer quien lee:

| Para | Haga esto |
|:--|:--|
| Encontrar algo | **Buscar** en la barra, <kbd>⌘</kbd> <kbd>K</kbd> (<kbd>Ctrl</kbd> <kbd>K</kbd>), o <kbd>/</kbd> |
| Ocultar o mostrar el índice | el interruptor a la izquierda de la barra, o <kbd>⌘</kbd> <kbd>\\</kbd> |
| Cambiar el tamaño del texto | **Aa** en la barra, luego **−** y **+**; o <kbd>⌘</kbd> <kbd>+</kbd> y <kbd>⌘</kbd> <kbd>−</kbd>, del 85 al 200 % |
| Cambiar el tema | **Aa**, luego Sistema, Claro u Oscuro |
| Ampliar una figura | haga clic en ella; dos clics más la acercan; <kbd>Esc</kbd> la devuelve a su sitio |
| Leer una nota al pie o una palabra punteada | pose el puntero sobre ella; haga clic para ir hasta ella, y **volver** lo trae de regreso |
| Compartir un capítulo o un pasaje | el **#** junto a un título copia su enlace (un teléfono abre su menú de compartir); seleccione unas palabras, y se le ofrece un enlace exactamente a ellas |
| Guardarla | el botón de descarga al final de la barra: la propia página (se lee sin conexión), imprimir o guardar como PDF, o el PDF y el EPUB donde se hayan generado |

## 3. Escribir

Markdown corriente, y cada construcción se lee bien también como texto plano. Lo que usted escribe va primero, en un bloque de código; lo que obtiene viene a continuación.

### Énfasis y enlaces

```markdown
**Negrita**, *cursiva*, ***ambas***, ~~tachado~~ y `código`.
Un [enlace](https://commonmark.org "CommonMark"), uno suelto,
https://spec.commonmark.org/0.31.2/, y marcas como [V] que se quedan tal como se escribieron.
```

**Negrita**, *cursiva*, ***ambas***, ~~tachado~~ y `código`. Un [enlace](https://commonmark.org "CommonMark"), uno suelto, https://spec.commonmark.org/0.31.2/, y marcas como [V] que se quedan tal como se escribieron.

### Listas dentro de listas

```markdown
1. Primero, con una lista anidada:
   - una viñeta
   - otra, con la suya propia:
     1. más adentro
2. Segundo.
```

1. Primero, con una lista anidada:
   - una viñeta
   - otra, con la suya propia:
     1. más adentro
2. Segundo.

### Citas y recuadros

```markdown
> Una cita corriente sigue siendo una cita.

> [!TIP]
> Las alertas de GitHub funcionan: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Nota:** también la forma antigua de recuadro, una palabra en negrita y dos puntos.
```

> Una cita corriente sigue siendo una cita.

> [!TIP]
> Las alertas de GitHub funcionan: NOTE, TIP, IMPORTANT, WARNING, CAUTION.

> **Nota:** también la forma antigua de recuadro, una palabra en negrita y dos puntos.

### Tablas

```markdown
| Habitación | Planta | Área (pies²) |
|:-----------|:------:|-------------:|
| Salón      | baja   |          312 |
| Altillo    | alta   |          205 |
```

| Habitación | Planta | Área (pies²) |
|:-----------|:------:|-------------:|
| Salón | baja | 312 |
| Altillo | alta | 205 |

### Notas al pie

```markdown
Una afirmación que necesita una fuente.[^1]

[^1]: La fuente. Pose el puntero sobre el número para leerla ahí.
```

Una afirmación que necesita una fuente.[^source]

[^source]: La fuente. Pose el puntero sobre el número para leerla ahí; haga clic en él para venir aquí, y en volver para regresar.

### Figuras e imágenes

```markdown
![Un pie de foto bajo la imagen.](report/zoom.jpg){width=60%}

Un icono ![la marca](mark.svg){width=18} dentro de una frase.
```

![Una figura al 60 % de la columna. Haga clic en ella: llena la ventana, y dos clics más la acercan.](report/zoom.jpg){width=60%}

Un icono ![la marca](mark.svg){width=18} dentro de una frase.

### Cosas pequeñas

```markdown
Pulse <kbd>Ctrl</kbd>+<kbd>K</kbd>. El agua es H<sub>2</sub>O, la habitación mide 9 m<sup>2</sup>.
Una línea que termina en barra invertida\
se corta ahí.
```

Pulse <kbd>Ctrl</kbd>+<kbd>K</kbd>. El agua es H<sub>2</sub>O, la habitación mide 9 m<sup>2</sup>. Una línea que termina en barra invertida\
se corta ahí.

<details>
<summary>Una sección que se abre con un clic</summary>

Escrita como `<details>` con un `<summary>`, con Markdown corriente dentro: **negrita**, una lista,

- uno
- dos

</details>

#### Un encabezado de cuarto nivel

Los encabezados por debajo del tercer nivel quedan fuera del índice, como este.

## 4. Código y matemáticas

El código se colorea para Rust, Python, JavaScript y TypeScript, Go, C y C++, Java, Kotlin, Swift, C#, el shell, SQL, Ruby, Lua, PHP, CSS, HTML, JSON, YAML, TOML, Dockerfiles y diffs, y cada bloque tiene un botón para copiar.

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

Las matemáticas las compone el propio navegador, como MathML, sin script y sin ninguna fuente que descargar. En línea, el teorema de Pitágoras es $a^2 + b^2 = c^2$, y la identidad de Euler es $e^{i\pi} + 1 = 0$; una cantidad en dólares como $5 o $10 sigue siendo dinero. En bloque:

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

### Diagramas

Un diagrama de flujo o de secuencia de Mermaid lo dibuja el propio md2html, como SVG dentro de la página: sin script, sin nada que descargar, con los colores de la página en ambos temas, y sus palabras siguen siendo palabras que la búsqueda encuentra. Este es el Markdown:

````markdown
```mermaid
flowchart LR
  md[notes.md] --> parse[Leer el Markdown]
  parse --> render{¿Diagrama?}
  render -->|de flujo o de secuencia| svg[Dibujarlo como SVG]
  render -->|de otro tipo| js[mermaid.js, si se indica]
  svg --> page([una página HTML])
  js --> page
```
````

Y esto es en lo que se convierte:

```mermaid
flowchart LR
  md[notes.md] --> parse[Leer el Markdown]
  parse --> render{¿Diagrama?}
  render -->|de flujo o de secuencia| svg[Dibujarlo como SVG]
  render -->|de otro tipo| js[mermaid.js, si se indica]
  svg --> page([una página HTML])
  js --> page
```

Un diagrama de secuencia, numerado, con una nota:

```mermaid
sequenceDiagram
  autonumber
  actor R as Lector
  participant P as La página
  participant S as La búsqueda
  R->>P: Cmd K
  P->>S: abrir la búsqueda
  R->>S: escribir "recuadro"
  S-->>R: los pasajes, los mejores primero
  R->>S: Intro
  S-->>P: saltar, e iluminar las palabras
  Note over R,P: sin red, sin servidor
```

Otros tipos (gráficos circulares, diagramas de Gantt, diagramas de clases) los dibuja mermaid.js cuando el autor le indica a md2html dónde hay una copia (`--mermaid-js FILE`); el script entra entonces solo en esa página. Sin ella, se quedan como código, y `--check` lo advierte.

## 5. El glosario y las notas al pie

Un capítulo llamado Glosario reúne las palabras de la página en una tabla. No se muestra donde está: en su lugar, la primera mención de cada término en cada capítulo se convierte en una palabra punteada que se explica sola al posar el puntero sobre ella, y un clic en ella salta a la entrada al pie, que lleva de vuelta a la línea que usted estaba leyendo.

![Al posar el puntero sobre una palabra punteada se muestra su significado.](report/glossary.jpg){width=70%}

Las notas al pie funcionan igual: el número se explica solo al posar el puntero sobre él.

![Al posar el puntero sobre el número de una nota al pie se muestra la nota.](report/footnote.jpg){width=70%}

> [!NOTE]
> Un glosario puede compartirse entre páginas: un solo archivo en `~/.config/md2html/glossary.md`, cuyas palabras se enlazan entonces en cada página generada en esa máquina.

## 6. Búsqueda y figuras

La búsqueda abarca todas las palabras de la página, el glosario incluido. Los resultados muestran dónde está cada palabra; la elegida se ilumina allí donde aterriza la búsqueda.

![Buscando "callout": el capítulo y el pasaje de cada resultado.](report/search.jpg){width=80%}

![Donde aterrizó la búsqueda: la entrada resplandece y la palabra encontrada se ilumina.](report/search-landed.jpg){width=80%}

Una figura se amplía sobre la página sin salir de ella. Si es más grande que la ventana, sigue al puntero, de modo que usted señala lo que quiere ver.

## 7. Ajustes

La parte superior del archivo Markdown original de este informe dice:

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

| Ajuste | Qué hace |
|:--|:--|
| `title`, `description`, `author` | el título, y las metaetiquetas para las vistas previas de enlaces |
| `lang` | el idioma: las palabras propias de la página y las fechas en cualquiera de los diecisiete de arriba (`ja-Hira` es el japonés en kana, `zh-Hant` el chino en caracteres tradicionales, `yue` el cantonés) |
| `translated_from`, `source_sha256` | para una traducción: su original, y la huella del original en el momento de traducirlo |
| `brand`, `brand_link`, `kicker`, `eyebrow`, `footer` | la barra, la línea sobre el título, la letra pequeña |
| `date`, `version` | la fecha y la versión que se muestran sobre el título |
| `embed` | las imágenes dentro de la página, para que sea un solo archivo |
| `csp` | una Content-Security-Policy |
| `pdf`, `epub` | un PDF y un EPUB de la página, junto a ella |
| `live` | el tiempo de vida de la página (`true` son dos minutos, o un número de segundos): después se recarga si ha cambiado, conservando el lugar del lector |
| `large`, `toc`, `math`, `index`, `html`, `css`, `head` | letra más grande, el índice, las matemáticas, los motores de búsqueda, el HTML sin procesar, estilo y head adicionales |

> [!TIP]
> Con `lang: de` la búsqueda dice "Diese Seite durchsuchen", los recuadros dicen "Hinweis" y "Achtung", y la fecha se lee "22. November 2026". Las páginas de Thomas en este sitio están en alemán de esta manera.

### Idiomas

Este informe existe en diecisiete idiomas: inglés, alemán, hebreo, indonesio, neerlandés, javanés, ídish, bajo alemán, francés, italiano, español, japonés escrito de tres maneras (en kanji y kana, solo en kana y en rōmaji), chino mandarín en caracteres simplificados y en caracteres tradicionales, y cantonés. El globo de la barra cambia de uno a otro, y la página se abre en el capítulo que usted estaba leyendo.

Una traducción es simplemente otro archivo Markdown junto al original, con el nombre de su idioma: `report.de.md` es el `report.md` en alemán. md2html encuentra las versiones que existen, las enlaza entre sí (y se lo indica a los navegadores y a los motores de búsqueda con `hreflang`), y da a cada una las palabras de su propia interfaz: la búsqueda, los recuadros, el glosario, las fechas. El hebreo y el ídish se disponen de derecha a izquierda, con el índice a la derecha.

Cada traducción nombra su original y registra la huella de ese original, de modo que la página puede decir cuál es la versión que da fe, y `--check` advierte cuando el original ha cambiado desde que se hizo la traducción:

```yaml
translated_from: report.md
source_sha256: 5b0c...e1
```

Para un idioma al que nadie ha traducido una página, el propio navegador del lector puede traducirla, si tiene esa capacidad (el traductor integrado de Chrome): en el propio dispositivo, para que nada de una página familiar salga de él, y señalada como traducción automática, con una forma de volver al original.

## 8. La línea de comandos

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

`--check` no escribe nada y enumera, línea por línea, lo que un autor debería revisar, y termina con el código 1 cuando hay algo: un enlace a un ancla que no está en la página, un archivo que no existe, una nota al pie sin su texto, HTML sin procesar mostrado como texto. Eso lo convierte en el último paso de una compilación, o del trabajo de un agente. `--watch` vuelve a generar la página cada vez que se guarda, `--nav` enlaza las páginas de una carpeta con anterior y siguiente, y un enlace a otro archivo Markdown lleva a su página.

`--pdf` imprime la página con el Chrome que ya está instalado en el equipo: su propio diseño de impresión, los encabezados como marcadores del PDF, el título en la cabecera de cada página y su número al pie. `--epub` hace con la misma página un libro para lectores electrónicos, con su índice, sus imágenes, diagramas y matemáticas, y notas al pie que se abren donde se mencionan; md2html lo escribe por sí mismo. El lector encuentra ambos bajo el botón de descarga de la barra.

`--serve` muestra las páginas en este equipo mientras se escriben: cada página abierta se recarga en cuanto se guarda el Markdown. Una página publicada con `--live` tiene su propio tiempo de vida; cuando se agota, pide a su servidor el primer kilobyte de sí misma y solo se recarga si ha cambiado. En ambos casos el lector se queda donde estaba: en el mismo párrafo a la misma altura o, si ya no está, en el capítulo anterior.

## 9. Seguridad, calidad y accesibilidad

> [!WARNING]
> Una página puede generarse a partir de Markdown que escribió otra persona. md2html nunca escribe un script a partir del texto: el HTML sin procesar se muestra tal como se escribió, salvo unas pocas etiquetas inofensivas, y los enlaces `javascript:` o `data:` se descartan, se escriban como se escriban. Un documento no puede hacer que md2html lea un archivo fuera de su carpeta, y ninguna entrada, por hostil que sea, puede bloquearlo ni hacerlo fallar.

> [!CAUTION]
> `--html` deja pasar un HTML más rico (imágenes, enlaces, vídeo, SVG, maquetación), pero nunca nada que se ejecute: ni scripts, ni manejadores de eventos, ni marcos, ni formularios.

Lo que se comprueba, en cada cambio, en macOS y en Ubuntu:

- 53 pruebas que generan páginas completas y las vuelven a leer, entre ellas los casos de seguridad;
- los 652 ejemplos de la especificación de CommonMark, cuyo número de aprobados puede subir pero nunca bajar;
- los lints más estrictos de clippy, y la versión más antigua de Rust con la que la herramienta promete compilar;
- una auditoría de accesibilidad con axe-core según WCAG 2.2 AA, en ambos temas y con el ancho de un teléfono;
- un recorrido de clics en Chrome que usa cada control: la búsqueda con el teclado y con clics, el tema, el tamaño del texto, el índice, una figura, una nota al pie, el glosario, copiar, imprimir.

La impresión tiene su propio diseño: los controles desaparecen, cada enlace imprime su dirección, nada queda cortado entre dos páginas y las páginas llevan el título y su número.

Cada página termina con una línea discreta: hecho con md2html, su versión y un enlace al lugar donde vive md2html.

![La misma página impresa.](report/print.jpg){width=55%}

## 10. Cómo surgió la 0.4

Empezó con una investigación: qué hacen los mejores conversores de Markdown, qué exigen los estándares web a una página y cómo se usa la herramienta en esta máquina, que resultó ser la parte más útil. Cuatro procesos de generación de páginas en 4sons.com reescribían la salida de md2html después de generarla, para cambiar la marca, el idioma, el pie de página y más. Cada una de esas reescrituras se convirtió en un ajuste, y las páginas se volvieron a generar con la nueva versión, una por una.

Los cambios, como un diff de las reglas que sigue el conversor:

```diff
- a handbook subset, anything else shown as written
+ all of CommonMark, and anything else shown as written
- every quotation a note
+ a quotation is a quotation; a bold first word makes it a note
- the page's words in English
+ the page's words in English or German
```

Lo que la 0.5 añadió desde entonces: diagramas. El camino que recorre cada página de este sitio:

```mermaid
flowchart TD
  md[Markdown] --> blocks[Bloques y elementos en línea, a la manera de CommonMark]
  blocks --> settings[Front matter y opciones]
  settings --> page[La página: índice, búsqueda, temas, glosario]
  page --> check{--check}
  check -->|nada que corregir| out([un archivo HTML])
  check -->|líneas que revisar| md
```

---

El glosario de abajo explica las palabras que usa este informe; cada una era una palabra punteada más arriba.

## Glosario

| Término | Significado | También |
|---|---|---|
| Markdown | Texto plano con marcas ligeras para encabezados, énfasis, listas y enlaces, legible tal cual y convertible en una página. | |
| CommonMark | La especificación de Markdown que hoy sigue la mayoría de las herramientas, con una prueba para cada regla. | |
| índice | La lista de capítulos a la izquierda de la página, hecha a partir de sus encabezados. | |
| recuadro | Un aparte enmarcado: una nota, un consejo, algo importante, una advertencia o una precaución. | recuadros, alerta, alertas |
| figura | Una imagen en una línea propia, con su pie debajo, que se amplía con un clic. | figuras |
| front matter | El bloque de ajustes entre dos líneas `---` al principio de un archivo Markdown. | |
| MathML | El lenguaje propio de la web para las matemáticas, que el navegador dibuja sin script. | |
| Content-Security-Policy | Una regla en la página que indica al navegador qué código puede ejecutarse; aquí, solo el de la propia página. | |
| Mermaid | Un lenguaje de texto para diagramas, escrito en un bloque de código; md2html dibuja sus diagramas de flujo y de secuencia. | |
| WCAG | Las pautas de accesibilidad para el contenido web (Web Content Accessibility Guidelines); el nivel AA es el objetivo habitual. | |
| traducción | Una versión de una página en otro idioma, un archivo Markdown junto al original. | traducciones |
