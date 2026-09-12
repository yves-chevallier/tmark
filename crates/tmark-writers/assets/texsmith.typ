// texsmith.typ — the Typst side of the fragment contracts
// (specs/migration/fragment-contracts.md §1, §3 rule 7): one hyphenated
// function per contract macro the tmark Typst writer emits, named
// arguments mirroring the LaTeX keys, content last. TeXSmith copies this
// file next to the `.typ` it builds; a template overrides a function after
// importing it. Every definition here is a plain default a template is
// expected to restyle.

// ---------------------------------------------------------- ts-typesetting

// `\tslead{…}`: a run-in lead.
#let ts-lead(body) = strong(body)

// `\tsdivider`: the `---` divider at the top level of the document; paged
// media break the page.
#let ts-divider() = pagebreak(weak: true)

// `\tsrule`: the same `---` inside a container (quote, callout, figure,
// div, list item, cell, aside, note). It separates without breaking: Typst
// refuses a page break inside a container outright.
#let ts-rule() = block(width: 100%, above: 0.8em, below: 0.8em, line(length: 100%, stroke: 0.4pt + luma(60%)))

// `\tsepigraph[source={…}]{…}`.
#let ts-epigraph(body, source: none) = align(right)[
  #block(width: 60%)[
    #emph(body)
    #if source != none [ #linebreak() --- #source ]
  ]
]

// `\tsaside[side=left]{…}`: a margin note.
#let ts-aside(body, side: "outer") = {
  let dx = if side == "left" { -3.2cm } else { 3.2cm }
  let alignment = if side == "left" { left } else { right }
  place(top + alignment, dx: dx, block(width: 2.6cm, text(size: 0.75em, body)))
}

// `\begin{tsdiv}{name}[keys]…`: a generic container, transparent by
// default; a template dispatches on `name`.
#let ts-div(name, body, ..args) = body

// `\tsmark{…}` is Typst's own `#highlight`; `\tsprogress` and `\tsicon`
// have no writer emission yet.

// -------------------------------------------------------------- ts-callouts

#let ts-callout-colors = (
  note: rgb("#448aff"),
  info: rgb("#00b8d4"),
  tip: rgb("#00bfa5"),
  hint: rgb("#00bfa5"),
  important: rgb("#7c4dff"),
  warning: rgb("#ff9100"),
  caution: rgb("#ff9100"),
  danger: rgb("#ff1744"),
  question: rgb("#64dd17"),
  seealso: rgb("#00b8d4"),
  abstract: rgb("#00b0ff"),
)

// `\begin{tscallout}[kind=note, title={…}, id=x, class={…}, collapsed]…`.
#let ts-callout(body, kind: "note", title: none, id: none, class: (), collapsed: false, ..args) = {
  let color = ts-callout-colors.at(kind, default: luma(40%))
  let heading = if title == none { upper(kind.first()) + kind.slice(1) } else { title }
  block(
    width: 100%,
    radius: 2pt,
    stroke: (left: 1.5pt + color, rest: 0.4pt + color),
    [
      #block(width: 100%, fill: color.lighten(90%), inset: (x: 8pt, y: 4pt))[
        #text(weight: "bold", fill: color)[#heading]
      ]
      #block(width: 100%, inset: (x: 8pt, y: 6pt))[#body]
    ],
  )
}

// ------------------------------------------------------------------ ts-code

// `\begin{tscode}[lang=py, title={…}, linenums, hl_lines={…}, caption={…}]`:
// the body is a fenced raw block; a title becomes a header line, a caption
// wraps the listing in a figure (the writer attaches the `<label>` after
// the call).
#let ts-code(body, title: none, linenums: none, hl-lines: (), caption: none, class: (), ..args) = {
  let listing = block(width: 100%, stroke: 0.4pt + luma(60%), radius: 2pt, inset: 6pt)[
    #if title != none [
      #block(width: 100%, inset: (bottom: 4pt), text(size: 0.85em, weight: "bold", title))
    ]
    #set raw(block: true)
    #body
  ]
  if caption != none { figure(listing, caption: caption, kind: "listing", supplement: [Listing]) } else { listing }
}

// -------------------------------------------------------------- ts-todolist

// `\item[\tsdone]`, `\item[\tstodo]`, `\item[\tspartial]`: a task item.
#let ts-task(state, body) = {
  let mark = if state == "done" { sym.checkmark } else if state == "partial" { sym.circle.filled } else { sym.space }
  box(width: 1.1em, height: 1.1em, stroke: 0.5pt, radius: 1pt, inset: (x: 2pt), align(center + horizon, text(size: 0.8em, mark)))
  h(0.4em)
  body
}

// ------------------------------------------------------------ ts-keystrokes

// `\tskeys{Ctrl,Alt,Del}`.
#let ts-keys(..keys) = keys.pos().map(k => box(stroke: 0.5pt, radius: 2pt, inset: (x: 3pt), outset: (y: 2pt), text(size: 0.85em, k))).join([+])

// -------------------------------------------------------------- ts-glossary

// `\tsgls{term}` and `\tsacr{key}`: the term or key as written; a template
// with a glossary shows the short form and links the entry.
#let ts-gls(term) = term
#let ts-acr(key) = key

// ----------------------------------------------------------------- ts-index

// `\tsindex[registry=r, main]{a!b}`: zero width (fragment-contracts.md
// open question 6).
#let ts-index(..args) = none

// ----------------------------------------------------------------- ts-fonts

// `\tsscript{slug}{…}`, `\tsemoji{…}`: the text itself; a template maps
// a script slug to a font.
#let ts-script(slug, body) = body
#let ts-emoji(body) = body

// ------------------------------------------------------------- subfigures

// The images of a `::: figure` container are sub-figures (spec §Image,
// Figure): the container keeps one figure number and each image takes it
// with a letter, `2a`, `2b`. A sub-figure is therefore a figure of its own
// `kind`, which leaves the image counter alone; the writer resets the
// sub-figure counter before every container, so the letters restart.
#let ts-subfigure(body, caption: none) = figure(
  body,
  caption: caption,
  kind: "ts-subfigure",
  supplement: none,
  numbering: "(a)",
)

// `#ts-subnumber(<fig:left>)`: what a reference to a sub-figure shows —
// the enclosing figure's number and the sub-figure's letter, read at the
// sub-figure's own location (the two counters are separate, so `#ref`
// alone would show the letter only).
#let ts-subnumber(target) = context {
  let matches = query(target)
  if matches.len() > 0 {
    let loc = matches.first().location()
    numbering("1", ..counter(figure.where(kind: image)).at(loc))
    numbering("a", ..counter(figure.where(kind: "ts-subfigure")).at(loc))
  }
}

// --------------------------------------------------------------- references

// `{page}` of a textual reference template: the page of a label.
#let ts-page(target) = context {
  let matches = query(target)
  if matches.len() > 0 { counter(page).at(matches.first().location()).first() }
}
