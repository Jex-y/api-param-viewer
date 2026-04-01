# ui/

TUI layout and widget composition using ratatui.

## Modules

- **mod.rs** — Top-level `draw()`: splits the frame into breadcrumb bar, main area (tree + detail), and status/search bar.
- **tree_view** — Left pane. Renders the flattened tree rows with expand/collapse arrows, type-colored previews, selection highlighting, and search match highlighting. Manages scroll offset.
- **detail_view** — Right pane. Dispatches to the appropriate renderer: `render_string_content` for `Value::String` (markdown/HTML/tables), `pretty_print_value` for everything else. Manages detail scroll offset.

## Layout

Left pane is 25% width, right pane is 75%. The status bar shows keybinding hints or the search prompt.
