# api-param-viewer

A terminal UI for viewing and exploring LLM API parameter files. Useful for debugging complex API calls with deeply nested messages, tool definitions, and system prompts.

![Rust](https://img.shields.io/badge/rust-stable-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Tree navigation** - Expand/collapse JSON structure with vim-style keybindings
- **Detail pane** - View the full content of any selected node
- **Markdown rendering** - System prompts and message content render with styled headers, bold, italic, code blocks, and bullet lists
- **Table rendering** - Markdown tables display with box-drawing borders and word-wrapped cells
- **HTML stripping** - HTML tags in content are converted to readable text
- **Search** - Find keys or values across the entire structure
- **Syntax highlighting** - JSON values are color-coded by type (strings, numbers, booleans, nulls)

## Install

```sh
cargo install --git https://github.com/Jex-y/api-param-viewer
```

## Usage

```sh
api-param-viewer path/to/api_params.json
```

The JSON file should be a serialised LLM API request body, e.g.:

```json
{
  "model": "claude-opus-4-5-20251101",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "max_tokens": 1024,
  "temperature": 0.7,
  "tools": [...],
  "system": "You are a helpful assistant."
}
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate up/down |
| `l` / `Enter` / `→` | Expand node |
| `h` / `←` | Collapse node / jump to parent |
| `Space` | Toggle expand/collapse |
| `Page Up` / `Page Down` | Jump 20 rows |
| `/` | Search keys and values |
| `n` / `N` | Next / previous search match |
| `d` / `u` | Scroll detail pane down/up |
| `q` / `Esc` | Quit |

## License

MIT
