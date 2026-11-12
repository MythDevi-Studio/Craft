//! File explorer sidebar widget.
//!
//! Shows a collapsible directory tree rooted at the workspace folder.
//! Clicking a file opens it; clicking a directory expands/collapses it.
//! Supports animated expand/collapse and hover highlights.

use std::path::{Path, PathBuf};

use crate::math::{Color, Rect, Vec2};
use crate::ui::animation::Anim;
use crate::ui::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::ui::{DrawCtx, Fonts, Theme};

/// A single node in the file tree.
#[derive(Clone, Debug)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Vec<FileNode>,
    pub depth: usize,
    pub dirty: bool, // has unsaved changes (shown as dot)
}

impl FileNode {
    fn icon(&self) -> &'static str {
        if self.is_dir {
            if self.expanded { "▾" } else { "▸" }
        } else {
            match self.path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "rs" => "⚙",
                "js" | "ts" | "jsx" | "tsx" => "⟨⟩",
                "json" => "{ }",
                "md" => "✎",
                "py" => "🐍",
                "css" => "🎨",
                "html" | "htm" => "⌨",
                _ => "·",
            }
        }
    }
}

/// An event emitted by the file explorer.
#[derive(Debug, Clone)]
pub enum ExplorerEvent {
    FileSelected(PathBuf),
    DirectoryToggled(PathBuf),
}

/// The file explorer sidebar.
pub struct FileExplorer {
    pub root: Option<FileNode>,
    pub width: f32,
    pub visible: bool,
    scroll_y: f32,
    hovered_path: Option<PathBuf>,
    pub events: Vec<ExplorerEvent>,
    width_anim: Anim<f32>,
    scroll_anim: Anim<f32>,
    open_files: Vec<PathBuf>,
}

impl FileExplorer {
    pub fn new() -> Self {
        Self {
            root: None,
            width: 220.0,
            visible: true,
            scroll_y: 0.0,
            hovered_path: None,
            events: Vec::new(),
            width_anim: Anim::new(220.0),
            scroll_anim: Anim::new(0.0),
            open_files: Vec::new(),
        }
    }

    pub fn open_path(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.root = Some(Self::scan_dir(&path, 0, 2));
        }
    }

    pub fn set_open_files(&mut self, files: Vec<PathBuf>) {
        self.open_files = files;
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.width_anim.animate_to(220.0, 0.15);
        } else {
            self.width_anim.animate_to(0.0, 0.15);
        }
    }

    pub fn current_width(&self) -> f32 {
        self.width_anim.value()
    }

    pub fn take_events(&mut self) -> Vec<ExplorerEvent> {
        std::mem::take(&mut self.events)
    }

    fn scan_dir(path: &Path, depth: usize, max_depth: usize) -> FileNode {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        let mut node = FileNode {
            path: path.to_path_buf(),
            name,
            is_dir: true,
            expanded: depth == 0,
            children: Vec::new(),
            depth,
            dirty: false,
        };

        if depth < max_depth {
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut entries: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .collect();
                // Directories first, then files, both alphabetical
                entries.sort_by_key(|e| {
                    let is_file = !e.path().is_dir();
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    (is_file as u8, name)
                });

                for entry in entries {
                    let p = entry.path();
                    let fname = entry.file_name().to_string_lossy().to_string();
                    // Skip hidden files and common noise directories
                    if fname.starts_with('.') || fname == "target" || fname == "node_modules" {
                        continue;
                    }
                    if p.is_dir() {
                        node.children.push(Self::scan_dir(&p, depth + 1, max_depth));
                    } else {
                        node.children.push(FileNode {
                            path: p.clone(),
                            name: fname,
                            is_dir: false,
                            expanded: false,
                            children: Vec::new(),
                            depth: depth + 1,
                            dirty: false,
                        });
                    }
                }
            }
        }
        node
    }

    fn flat_visible<'a>(node: &'a FileNode, out: &mut Vec<&'a FileNode>) {
        out.push(node);
        if node.is_dir && node.expanded {
            for child in &node.children {
                Self::flat_visible(child, out);
            }
        }
    }

    fn toggle_node(node: &mut FileNode, target: &Path) -> bool {
        if node.path == target {
            if node.is_dir {
                node.expanded = !node.expanded;
                // Lazy-expand: scan one more level
                if node.expanded && node.children.is_empty() {
                    if let Some(child_node) = Some(Self::scan_dir(&node.path, node.depth, node.depth + 2)) {
                        node.children = child_node.children;
                    }
                }
            }
            return true;
        }
        for child in &mut node.children {
            if Self::toggle_node(child, target) {
                return true;
            }
        }
        false
    }
}

impl Widget for FileExplorer {
    fn paint(&mut self, ctx: &DrawCtx<'_>, rect: Rect, theme: &Theme, fonts: &Fonts) {
        let w = self.width_anim.value();
        if w < 4.0 { return; }

        let clip_rect = Rect::new(rect.x, rect.y, w.min(rect.w), rect.h);
        ctx.clip(clip_rect, |ctx| {
            ctx.fill_rect(clip_rect, theme.bg_surface);
            ctx.draw_vline(clip_rect.right() - 1.0, clip_rect.y, clip_rect.h, theme.border, 1.0);

            let root = match &self.root {
                Some(r) => r,
                None => {
                    let msg = "No folder open";
                    let mw = fonts.measure_ui(msg);
                    ctx.draw_text(msg, Vec2::new(clip_rect.x + (w - mw) / 2.0, clip_rect.y + 20.0), &fonts.ui, theme.fg_muted);
                    return;
                }
            };

            // Header
            ctx.fill_rect(Rect::new(clip_rect.x, clip_rect.y, w, 32.0), theme.bg_elevated);
            let root_name = root.name.to_uppercase();
            ctx.draw_text(&root_name, Vec2::new(clip_rect.x + 12.0, clip_rect.y + 8.0), &fonts.ui, theme.fg_muted);

            let scroll = self.scroll_anim.value();
            let item_h = 24.0;

            let visible: Vec<&FileNode> = {
                let mut v = Vec::new();
                Self::flat_visible(root, &mut v);
                v
            };

            ctx.clip(Rect::new(clip_rect.x, clip_rect.y + 32.0, w, clip_rect.h - 32.0), |ctx| {
                for (i, node) in visible.iter().enumerate() {
                    let iy = clip_rect.y + 32.0 + i as f32 * item_h - scroll;
                    if iy + item_h < clip_rect.y + 32.0 || iy > clip_rect.bottom() { continue; }

                    let row_rect = Rect::new(clip_rect.x, iy, w, item_h);
                    let hovered = self.hovered_path.as_ref() == Some(&node.path);
                    let is_open = self.open_files.contains(&node.path);

                    if hovered || is_open {
                        let bg = if is_open { theme.bg_active } else { theme.bg_hover };
                        ctx.fill_rect(row_rect, bg);
                    }

                    let indent_x = clip_rect.x + 8.0 + node.depth as f32 * 14.0;

                    // Icon
                    let icon = node.icon();
                    let icon_color = if node.is_dir {
                        theme.accent
                    } else {
                        theme.fg_muted
                    };
                    ctx.draw_text(icon, Vec2::new(indent_x, iy + (item_h - fonts.ui.size()) / 2.0), &fonts.ui, icon_color);

                    // Name
                    let name_color = if is_open {
                        theme.fg_primary
                    } else if hovered {
                        theme.fg_primary
                    } else {
                        theme.fg_secondary
                    };
                    let name_x = indent_x + 18.0;
                    ctx.draw_text_clipped(
                        &node.name,
                        Vec2::new(name_x, iy + (item_h - fonts.ui.size()) / 2.0),
                        &fonts.ui,
                        name_color,
                        Rect::new(name_x, iy, w - name_x + clip_rect.x - 4.0, item_h),
                    );

                    // Dirty dot
                    if node.dirty {
                        ctx.fill_circle(Vec2::new(clip_rect.x + w - 10.0, iy + item_h / 2.0), 3.0, theme.accent);
                    }
                }
            });
        });
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _theme: &Theme) -> EventResult {
        let w = self.width_anim.value();
        if w < 4.0 { return EventResult::Ignored; }
        let explorer_rect = Rect::new(rect.x, rect.y, w, rect.h);

        match event {
            UiEvent::MouseMove { pos } => {
                if explorer_rect.contains(*pos) {
                    // Find which node is hovered
                    let scroll = self.scroll_anim.value();
                    let item_h = 24.0;
                    let relative_y = pos.y - rect.y - 32.0 + scroll;
                    let idx = (relative_y / item_h).max(0.0) as usize;

                    if let Some(root) = &self.root {
                        let mut visible = Vec::new();
                        Self::flat_visible(root, &mut visible);
                        self.hovered_path = visible.get(idx).map(|n| n.path.clone());
                    }
                    return EventResult::Consumed;
                }
                self.hovered_path = None;
                EventResult::Ignored
            }
            UiEvent::MouseDown { pos, button } if *button == MouseButton::Left => {
                if explorer_rect.contains(*pos) {
                    let scroll = self.scroll_anim.value();
                    let item_h = 24.0;
                    let relative_y = pos.y - rect.y - 32.0 + scroll;
                    let idx = (relative_y / item_h).max(0.0) as usize;

                    if let Some(root) = &self.root {
                        let mut visible = Vec::new();
                        Self::flat_visible(root, &mut visible);
                        if let Some(node) = visible.get(idx) {
                            if node.is_dir {
                                let p = node.path.clone();
                                if let Some(root) = &mut self.root {
                                    Self::toggle_node(root, &p);
                                }
                                self.events.push(ExplorerEvent::DirectoryToggled(p));
                            } else {
                                self.events.push(ExplorerEvent::FileSelected(node.path.clone()));
                            }
                            return EventResult::Consumed;
                        }
                    }
                }
                EventResult::Ignored
            }
            UiEvent::Scroll { delta, pos } if explorer_rect.contains(*pos) => {
                let new_scroll = (self.scroll_anim.value() - delta.y * 24.0 * 3.0).max(0.0);
                self.scroll_anim.animate_to(new_scroll, 0.08);
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    fn update(&mut self, dt: f32) -> bool {
        let a = self.width_anim.update(dt);
        let b = self.scroll_anim.update(dt);
        a || b
    }
}

impl Default for FileExplorer {
    fn default() -> Self {
        Self::new()
    }
}
