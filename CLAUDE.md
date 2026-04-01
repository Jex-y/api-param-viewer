# api-param-viewer

TUI for exploring LLM API parameter JSON files (Anthropic, OpenAI, etc). Built for debugging — the goal is to make large, deeply nested API payloads easy to navigate and read.

## Build & run

```sh
cargo build --release
cargo install --path .
api-param-viewer path/to/params.json
```

## Architecture

Hexagonal-ish layered structure. Dependencies flow inward — outer layers know about inner layers, never the reverse.

```
main.rs          # I/O shell: CLI, terminal, event loop
├── app.rs       # Domain: state, tree building, navigation, search
├── tree.rs      # Domain: TreeRow/PathSeg types, preview heuristics
├── render/      # Pure transforms: content → styled lines
│   ├── html     # HTML stripping & entity decoding
│   ├── markdown # Markdown → styled spans
│   ├── table    # Markdown tables → box-drawn tables
│   └── json     # JSON values → styled lines (with inline text blocks)
└── ui/          # TUI composition: styled lines → ratatui widgets
    ├── tree_view
    └── detail_view
```

**Domain** (`app`, `tree`) has no rendering or TUI dependencies — only `serde_json` and `ratatui::style::Color` for type coloring.

**Render** modules are pure functions: `&str` or `&Value` in, `Vec<Line>` out. No app state, no frame access.

**UI** composes render output into widgets and draws them. Owns layout decisions and scrolling viewport logic.

## Key principles

- **Read-only viewer** — never modifies the input file
- **Render content, not escapes** — strings with `\n`, HTML, and markdown should display as readable text, not raw JSON escapes
- **Semantic previews** — tree labels should surface useful info (message roles, tool names, content types) so users can navigate without expanding everything
- **Fit the terminal** — tables and text must word-wrap to the available width, never overflow
