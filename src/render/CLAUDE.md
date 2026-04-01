# render/

Pure content transformation: input data → `Vec<Line<'static>>`. No app state, no frame access.

## Modules

- **html** — Strip HTML tags and decode entities. `inline` mode removes tags silently (for table cells); block mode converts `<br/>`, `</p>` etc. to newlines.
- **markdown** — Single-line markdown rendering: headers, bold, italic, inline code, bullets, horizontal rules.
- **table** — Detect and render markdown tables with box-drawing characters. Handles column width distribution and word wrapping within cells.
- **json** — Recursive JSON pretty-printer. Short strings render inline; long/multiline strings get a bordered text block with HTML stripped.

## Adding new renderers

Each module should expose pure functions. The detail view in `ui/` decides which renderer to call based on value type.
