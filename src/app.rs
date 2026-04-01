use std::collections::HashSet;

use serde_json::Value;

use crate::tree::{array_child_label, estimate_tokens, path_key, value_preview, PathSeg, TreeRow};

pub struct App {
    root: Value,
    pub rows: Vec<TreeRow>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub detail_scroll: usize,
    expanded_paths: HashSet<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub search_match_idx: Option<usize>,
    pub token_estimate: usize,
    pub context_limit: usize,
    pub model_name: String,
}

fn model_context_limit(model: &str) -> usize {
    let m = model.to_lowercase();
    // Claude models
    if m.contains("opus") && (m.contains("4-6") || m.contains("4.6")) {
        return 1_000_000;
    }
    if m.contains("sonnet") && (m.contains("4-6") || m.contains("4.6")) {
        return 1_000_000;
    }
    if m.contains("opus") || m.contains("sonnet") {
        return 200_000;
    }
    if m.contains("claude-3") || m.contains("haiku") {
        return 200_000;
    }
    if m.contains("claude") {
        return 200_000;
    }
    // OpenAI models
    if m.contains("gpt-4o") || m.contains("gpt-4-turbo") {
        return 128_000;
    }
    if m.contains("gpt-4-32k") {
        return 32_768;
    }
    if m.contains("gpt-4") {
        return 8_192;
    }
    if m.contains("gpt-3.5") {
        return 16_385;
    }
    if m.contains("o1") || m.contains("o3") || m.contains("o4") {
        return 200_000;
    }
    // Google models
    if m.contains("gemini") {
        return 1_000_000;
    }
    // Default fallback
    128_000
}

impl App {
    pub fn new(root: Value) -> Self {
        let token_estimate = serde_json::to_string(&root).unwrap_or_default().len() / 4;
        let model_name = root.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let context_limit = model_context_limit(&model_name);
        let mut app = App {
            root,
            rows: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            detail_scroll: 0,
            expanded_paths: HashSet::new(),
            search_mode: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: None,
            token_estimate,
            context_limit,
            model_name,
        };
        app.expanded_paths.insert(String::new());
        app.rebuild_rows();
        app
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        let root = self.root.clone();
        self.build_rows(&root, 0, &[], "root");
    }

    fn build_rows(&mut self, value: &Value, depth: usize, path: &[PathSeg], key: &str) {
        let pk = path_key(path);
        let is_expanded = self.expanded_paths.contains(&pk);
        let (preview, expandable, type_color) = value_preview(value);

        self.rows.push(TreeRow {
            depth,
            key: key.to_string(),
            preview,
            expandable,
            expanded: is_expanded,
            path: path.to_vec(),
            type_color,
            tokens: estimate_tokens(value),
        });

        if is_expanded {
            match value {
                Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSeg::Index(i));
                        let label = array_child_label(i, item);
                        self.build_rows(item, depth + 1, &child_path, &label);
                    }
                }
                Value::Object(obj) => {
                    for (k, v) in obj.iter() {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSeg::Key(k.clone()));
                        self.build_rows(v, depth + 1, &child_path, k);
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_path(&self, path: &[PathSeg]) -> &Value {
        let mut current = &self.root;
        for seg in path {
            current = match seg {
                PathSeg::Key(k) => &current[k],
                PathSeg::Index(i) => &current[*i],
            };
        }
        current
    }

    pub fn selected_value(&self) -> Option<&Value> {
        self.rows
            .get(self.selected)
            .map(|row| self.resolve_path(&row.path))
    }

    pub fn selected_token_estimate(&self) -> usize {
        self.selected_value()
            .map(|v| serde_json::to_string(v).unwrap_or_default().len() / 4)
            .unwrap_or(0)
    }

    pub fn breadcrumb(&self) -> String {
        if let Some(row) = self.rows.get(self.selected) {
            let parts: Vec<String> = std::iter::once("$".to_string())
                .chain(row.path.iter().map(|s| match s {
                    PathSeg::Key(k) => k.clone(),
                    PathSeg::Index(i) => format!("[{}]", i),
                }))
                .collect();
            parts.join(".")
        } else {
            String::new()
        }
    }

    // Navigation

    pub fn toggle_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            if row.expandable {
                let pk = path_key(&row.path);
                if self.expanded_paths.contains(&pk) {
                    let prefix = if pk.is_empty() {
                        String::new()
                    } else {
                        format!("{}/", pk)
                    };
                    self.expanded_paths
                        .retain(|p| p != &pk && !p.starts_with(&prefix));
                } else {
                    self.expanded_paths.insert(pk);
                }
                self.rebuild_rows();
                if self.selected >= self.rows.len() {
                    self.selected = self.rows.len().saturating_sub(1);
                }
            }
        }
        self.detail_scroll = 0;
    }

    pub fn expand_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            if row.expandable && !row.expanded {
                let pk = path_key(&row.path);
                self.expanded_paths.insert(pk);
                self.rebuild_rows();
            }
        }
    }

    pub fn collapse_selected(&mut self) {
        if let Some(row) = self.rows.get(self.selected) {
            let pk = path_key(&row.path);
            if row.expandable && row.expanded {
                let prefix = if pk.is_empty() {
                    String::new()
                } else {
                    format!("{}/", pk)
                };
                self.expanded_paths
                    .retain(|p| p != &pk && !p.starts_with(&prefix));
                self.rebuild_rows();
            } else if row.depth > 0 {
                let target_depth = row.depth - 1;
                for i in (0..self.selected).rev() {
                    if self.rows[i].depth == target_depth {
                        self.selected = i;
                        break;
                    }
                }
            }
        }
        self.detail_scroll = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
        self.detail_scroll = 0;
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.selected = (self.selected + page_size).min(self.rows.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    // Search

    pub fn update_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_matches.clear();
        self.search_match_idx = None;
        if query.is_empty() {
            return;
        }
        for (i, row) in self.rows.iter().enumerate() {
            if row.key.to_lowercase().contains(&query)
                || row.preview.to_lowercase().contains(&query)
            {
                self.search_matches.push(i);
            }
        }
        if !self.search_matches.is_empty() {
            self.search_match_idx = Some(0);
            self.selected = self.search_matches[0];
            self.detail_scroll = 0;
        }
    }

    pub fn next_search_match(&mut self) {
        if let Some(idx) = self.search_match_idx {
            let next = (idx + 1) % self.search_matches.len();
            self.search_match_idx = Some(next);
            self.selected = self.search_matches[next];
            self.detail_scroll = 0;
        }
    }

    pub fn prev_search_match(&mut self) {
        if let Some(idx) = self.search_match_idx {
            let prev = if idx == 0 {
                self.search_matches.len() - 1
            } else {
                idx - 1
            };
            self.search_match_idx = Some(prev);
            self.selected = self.search_matches[prev];
            self.detail_scroll = 0;
        }
    }
}
