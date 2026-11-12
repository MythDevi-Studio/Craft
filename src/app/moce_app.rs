//! The main application struct and winit ApplicationHandler implementation.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::editor::{Document, Language};
use crate::math::{Rect, Size, Vec2};
use crate::ui::context::UiContext;
use crate::ui::painter::{DrawCtx, Fonts, Painter};
use crate::ui::theme::Theme;
use crate::ui::widget::{Key, Modifiers, UiEvent, Widget};
use crate::ui::widgets::{
    CommandPalette, EditorView, ExplorerEvent, FileExplorer, FindReplaceBar, FindReplaceEvent,
    NotificationKind, NotificationManager, PaletteItem, PaletteKind, Tab, TabBar, TitleBar,
    TitleBarEvent,
};
use crate::ui::widgets::status_bar::StatusBar;
use crate::ui::widgets::minimap::{self, Minimap};
use crate::ui::widgets::lsp_popup::{CompletionEvent, CompletionPopup, HoverPopup};

/// Actions returned from `dispatch_event` that require window-level responses.
/// These MUST be acted on inside the same OS event callback where they were generated.
#[derive(Default)]
struct DispatchResult {
    /// Call `window.drag_window()` immediately.
    start_drag: bool,
    /// Exit the application.
    exit: bool,
}

static NEXT_TAB_ID: AtomicUsize = AtomicUsize::new(1);
fn next_tab_id() -> usize {
    NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed)
}

struct WindowState {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,
    painter: Painter,
}

pub struct MoceApp {
    window_state: Option<WindowState>,
    ctx: Option<UiContext>,

    // Chrome widgets
    title_bar: TitleBar,
    tab_bar: TabBar,
    status_bar: StatusBar,

    // Editor content
    editors: Vec<EditorView>,

    // Overlay / sidebar widgets
    file_explorer: FileExplorer,
    command_palette: CommandPalette,
    find_replace: FindReplaceBar,
    minimap: Minimap,
    hover_popup: HoverPopup,
    completion_popup: CompletionPopup,
    notifications: NotificationManager,

    // Clipboard
    clipboard: Option<arboard::Clipboard>,

    // Frame timing
    last_frame: Instant,
    needs_redraw: bool,

    // Double-click tracking for title bar maximize
    last_click_time: Instant,
    last_click_pos: Vec2,
}

impl MoceApp {
    pub fn new() -> Self {
        let title_bar = TitleBar::new("Moce");
        let mut tab_bar = TabBar::new();

        let doc = Document::from_str(WELCOME_TEXT, Language::PlainText);
        let tab_id = next_tab_id();
        tab_bar.add_tab(Tab::new(tab_id, "Welcome"));
        tab_bar.active_id = Some(tab_id);
        let editor = EditorView::new(doc);

        let mut status = StatusBar::new();
        status.set_cursor(0, 0);
        status.set_language("Plain Text");

        let clipboard = arboard::Clipboard::new().ok();

        Self {
            window_state: None,
            ctx: None,
            title_bar,
            tab_bar,
            status_bar: status,
            editors: vec![editor],
            file_explorer: FileExplorer::new(),
            command_palette: CommandPalette::new(),
            find_replace: FindReplaceBar::new(),
            minimap: Minimap::new(),
            hover_popup: HoverPopup::new(),
            completion_popup: CompletionPopup::new(),
            notifications: NotificationManager::new(),
            clipboard,
            last_frame: Instant::now(),
            needs_redraw: true,
            last_click_time: Instant::now(),
            last_click_pos: Vec2::ZERO,
        }
    }

    pub fn open_file(&mut self, path: PathBuf) {
        // Check if already open
        if let Some(idx) = self.editors.iter().position(|e| e.doc.path.as_deref() == Some(&path)) {
            let tab_id = self.tab_bar.tabs.get(idx).map(|t| t.id);
            if let Some(id) = tab_id {
                self.tab_bar.active_id = Some(id);
                return;
            }
        }
        match Document::from_file(path.clone()) {
            Ok(doc) => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
                let id = next_tab_id();
                self.tab_bar.add_tab(Tab::new(id, name));
                self.tab_bar.active_id = Some(id);
                self.editors.push(EditorView::new(doc));
                self.notifications.info(format!("Opened {}", path.display()));
            }
            Err(e) => {
                self.notifications.error(format!("Failed to open file: {e}"));
            }
        }
    }

    fn active_editor_idx(&self) -> Option<usize> {
        let active_id = self.tab_bar.active_id?;
        self.tab_bar.tabs.iter().position(|t| t.id == active_id)
    }

    fn default_palette_items() -> Vec<PaletteItem> {
        vec![
            PaletteItem { id: "new_file".into(), label: "New File".into(), description: "Ctrl+N".into(), kind: PaletteKind::Command },
            PaletteItem { id: "open_file".into(), label: "Open File…".into(), description: "Ctrl+O".into(), kind: PaletteKind::Command },
            PaletteItem { id: "save".into(), label: "Save".into(), description: "Ctrl+S".into(), kind: PaletteKind::Command },
            PaletteItem { id: "save_as".into(), label: "Save As…".into(), description: "Ctrl+Shift+S".into(), kind: PaletteKind::Command },
            PaletteItem { id: "find".into(), label: "Find".into(), description: "Ctrl+F".into(), kind: PaletteKind::Command },
            PaletteItem { id: "replace".into(), label: "Find & Replace".into(), description: "Ctrl+H".into(), kind: PaletteKind::Command },
            PaletteItem { id: "toggle_explorer".into(), label: "Toggle File Explorer".into(), description: "Ctrl+B".into(), kind: PaletteKind::Command },
            PaletteItem { id: "toggle_minimap".into(), label: "Toggle Minimap".into(), description: "".into(), kind: PaletteKind::Command },
            PaletteItem { id: "close_tab".into(), label: "Close Tab".into(), description: "Ctrl+W".into(), kind: PaletteKind::Command },
            PaletteItem { id: "toggle_theme".into(), label: "Toggle Light/Dark Theme".into(), description: "".into(), kind: PaletteKind::Command },
            PaletteItem { id: "zoom_in".into(), label: "Increase Font Size".into(), description: "Ctrl++".into(), kind: PaletteKind::Command },
            PaletteItem { id: "zoom_out".into(), label: "Decrease Font Size".into(), description: "Ctrl+-".into(), kind: PaletteKind::Command },
        ]
    }

    fn handle_palette_event(&mut self, id: &str) {
        match id {
            "new_file" => {
                let doc = Document::new();
                let tab_id = next_tab_id();
                self.tab_bar.add_tab(Tab::new(tab_id, "untitled"));
                self.tab_bar.active_id = Some(tab_id);
                self.editors.push(EditorView::new(doc));
            }
            "save" => {
                if let Some(idx) = self.active_editor_idx() {
                    if idx < self.editors.len() {
                        match self.editors[idx].doc.save() {
                            Ok(_) => self.notifications.success("File saved"),
                            Err(e) => self.notifications.error(format!("Save failed: {e}")),
                        }
                    }
                }
            }
            "find" => { self.find_replace.show(false); }
            "replace" => { self.find_replace.show(true); }
            "toggle_explorer" => { self.file_explorer.toggle_visibility(); }
            "toggle_minimap" => {
                self.minimap.visible = !self.minimap.visible;
            }
            "close_tab" => {
                if let Some(id) = self.tab_bar.active_id {
                    let idx = self.tab_bar.tabs.iter().position(|t| t.id == id);
                    self.tab_bar.close_tab(id);
                    if let Some(idx) = idx {
                        if idx < self.editors.len() { self.editors.remove(idx); }
                    }
                }
            }
            "toggle_theme" => {
                // Swap theme dark/light
                if let Some(ctx) = &mut self.ctx {
                    let is_dark = ctx.theme.bg_base.r < 50;
                    ctx.theme = if is_dark { Theme::light() } else { Theme::dark() };
                }
            }
            "zoom_in" => {
                if let Some(ctx) = &mut self.ctx {
                    let new_size = (ctx.theme.font_size + 1.0).min(32.0);
                    ctx.theme.font_size = new_size;
                    ctx.fonts = Fonts::load(new_size, new_size - 1.0);
                }
            }
            "zoom_out" => {
                if let Some(ctx) = &mut self.ctx {
                    let new_size = (ctx.theme.font_size - 1.0).max(8.0);
                    ctx.theme.font_size = new_size;
                    ctx.fonts = Fonts::load(new_size, new_size - 1.0);
                }
            }
            _ => {}
        }
    }

    fn handle_find_events(&mut self) {
        let events: Vec<FindReplaceEvent> = self.find_replace.take_events();
        for ev in events {
            match ev {
                FindReplaceEvent::Close => {}
                FindReplaceEvent::QueryChanged(q) => {
                    if let Some(idx) = self.active_editor_idx() {
                        if idx < self.editors.len() {
                            self.editors[idx].set_search_query(q.clone());
                            let count = self.editors[idx].search_match_count();
                            self.find_replace.match_count = count;
                        }
                    }
                }
                FindReplaceEvent::FindNext => {
                    if let Some(idx) = self.active_editor_idx() {
                        if idx < self.editors.len() {
                            self.editors[idx].find_next();
                        }
                    }
                }
                FindReplaceEvent::FindPrev => {
                    if let Some(idx) = self.active_editor_idx() {
                        if idx < self.editors.len() {
                            self.editors[idx].find_prev();
                        }
                    }
                }
                FindReplaceEvent::ReplaceOne => {
                    if let Some(idx) = self.active_editor_idx() {
                        if idx < self.editors.len() {
                            let repl = self.find_replace.replacement.clone();
                            self.editors[idx].replace_current(&repl);
                        }
                    }
                }
                FindReplaceEvent::ReplaceAll => {
                    if let Some(idx) = self.active_editor_idx() {
                        if idx < self.editors.len() {
                            let repl = self.find_replace.replacement.clone();
                            let count = self.editors[idx].replace_all(&repl);
                            self.notifications.info(format!("Replaced {count} occurrences"));
                        }
                    }
                }
            }
        }
    }

    fn tick(&mut self, dt: f32) -> bool {
        let mut needs = self.needs_redraw;
        needs |= self.title_bar.update(dt);
        needs |= self.tab_bar.update(dt);
        for editor in &mut self.editors {
            needs |= editor.update(dt);
        }
        needs |= self.command_palette.update(dt);
        needs |= self.find_replace.update(dt);
        needs |= self.file_explorer.update(dt);
        needs |= self.hover_popup.update(dt);
        needs |= self.completion_popup.update(dt);
        needs |= self.notifications.update(dt);
        self.needs_redraw = false;
        needs
    }

    fn render(&mut self) {
        let (win_w, win_h) = {
            let ws = match &self.window_state {
                Some(w) => w,
                None => return,
            };
            (ws.painter.size().w, ws.painter.size().h)
        };

        let (title_h, tab_h, status_h, explorer_w, minimap_w, find_h) = {
            let ctx = match &self.ctx {
                Some(c) => c,
                None => return,
            };
            (
                ctx.theme.title_bar_height,
                ctx.theme.tab_height,
                ctx.theme.status_bar_height,
                self.file_explorer.current_width(),
                if self.minimap.visible { 100.0_f32 } else { 0.0 },
                self.find_replace.bar_height(),
            )
        };

        let title_rect = Rect::new(0.0, 0.0, win_w, title_h);
        let tab_rect = Rect::new(0.0, title_h, win_w, tab_h);
        let explorer_rect = Rect::new(0.0, title_h + tab_h, explorer_w, win_h - title_h - tab_h - status_h);
        let content_x = explorer_w;
        let content_w = win_w - explorer_w - minimap_w;
        let find_rect = Rect::new(content_x, title_h + tab_h, content_w, find_h);
        let editor_rect = Rect::new(content_x, title_h + tab_h + find_h, content_w, win_h - title_h - tab_h - status_h - find_h);
        let minimap_rect = Rect::new(win_w - minimap_w, title_h + tab_h, minimap_w, win_h - title_h - tab_h - status_h);
        let status_rect = Rect::new(0.0, win_h - status_h, win_w, status_h);
        let window_rect = Rect::new(0.0, 0.0, win_w, win_h);

        let theme_ptr = &self.ctx.as_ref().unwrap().theme as *const Theme;
        let fonts_ptr = &self.ctx.as_ref().unwrap().fonts as *const Fonts;

        {
            let ws = self.window_state.as_mut().unwrap();
            let canvas = ws.painter.canvas();
            let ctx = DrawCtx { canvas };

            let theme: &Theme = unsafe { &*theme_ptr };
            let fonts: &Fonts = unsafe { &*fonts_ptr };

            ctx.clear(theme.bg_base);

            // ── File explorer ──────────────────────────────────────────
            self.file_explorer.paint(&ctx, Rect::new(0.0, title_h + tab_h, explorer_w.max(1.0), win_h - title_h - tab_h - status_h), theme, fonts);

            // ── Tab bar ────────────────────────────────────────────────
            self.tab_bar.paint(&ctx, tab_rect, theme, fonts);

            // ── Find/replace bar ───────────────────────────────────────
            if find_h > 1.0 {
                self.find_replace.paint(&ctx, find_rect, theme, fonts);
            }

            // ── Active editor ──────────────────────────────────────────
            let active_idx = self.tab_bar.active_id.and_then(|id|
                self.tab_bar.tabs.iter().position(|t| t.id == id)
            );
            if let Some(idx) = active_idx {
                if idx < self.editors.len() {
                    self.editors[idx].paint(&ctx, editor_rect, theme, fonts);

                    // ── Minimap ────────────────────────────────────────
                    if self.minimap.visible {
                        self.minimap.paint(&ctx, minimap_rect, theme, fonts);
                        let scroll_y = self.editors[idx].scroll_y();
                        let viewport_h = editor_rect.h;
                        // Split the borrow: get buffer as a raw pointer to allow
                        // simultaneous mutable access to highlighter
                        let buffer_ptr = &self.editors[idx].doc.buffer as *const crate::editor::Buffer;
                        let buffer = unsafe { &*buffer_ptr };
                        minimap::paint_minimap_content(
                            &ctx,
                            minimap_rect,
                            self.editors[idx].highlighter_mut(),
                            buffer,
                            scroll_y,
                            viewport_h,
                            theme,
                        );
                    }
                }
            } else {
                ctx.fill_rect(editor_rect, theme.bg_base);
                let msg = "Open a file to start editing";
                let w = fonts.measure_ui(msg);
                ctx.draw_text(
                    msg,
                    Vec2::new(editor_rect.x + (editor_rect.w - w) / 2.0, editor_rect.y + editor_rect.h / 2.0),
                    &fonts.ui,
                    theme.fg_muted,
                );
            }

            // ── Title bar ──────────────────────────────────────────────
            self.title_bar.paint(&ctx, title_rect, theme, fonts);

            // ── Status bar ─────────────────────────────────────────────
            self.status_bar.paint(&ctx, status_rect, theme, fonts);

            // ── Overlays (drawn last — always on top) ──────────────────
            self.command_palette.paint(&ctx, window_rect, theme, fonts);
            self.hover_popup.paint(&ctx, window_rect, theme, fonts);
            self.completion_popup.paint(&ctx, window_rect, theme, fonts);
            self.notifications.paint(&ctx, window_rect, theme, fonts);
        }

        // ── Blit to window ─────────────────────────────────────────────
        {
            let ws = self.window_state.as_mut().unwrap();
            let w = ws.painter.size().w as u32;
            let h = ws.painter.size().h as u32;
            let Some(nw) = NonZeroU32::new(w) else { return };
            let Some(nh) = NonZeroU32::new(h) else { return };
            if ws.surface.resize(nw, nh).is_err() { return; }
            let Ok(mut buffer) = ws.surface.buffer_mut() else { return };
            if let Some(pixels) = ws.painter.pixels() {
                let len = (w * h) as usize;
                if pixels.len() == len && buffer.len() == len {
                    buffer.copy_from_slice(&pixels);
                }
            }
            let _ = buffer.present();
        }
    }

    fn dispatch_event(&mut self, event: &UiEvent) -> DispatchResult {
        let mut result = DispatchResult::default();
        let (win_w, win_h, title_h, tab_h, status_h, explorer_w, minimap_w, find_h) = {
            let ctx = match &self.ctx {
                Some(c) => c,
                None => return result,
            };
            let ew = self.file_explorer.current_width();
            let mw = if self.minimap.visible { 100.0 } else { 0.0 };
            let fh = self.find_replace.bar_height();
            (
                ctx.input.window_size.w,
                ctx.input.window_size.h,
                ctx.theme.title_bar_height,
                ctx.theme.tab_height,
                ctx.theme.status_bar_height,
                ew, mw, fh,
            )
        };

        let title_rect = Rect::new(0.0, 0.0, win_w, title_h);
        let tab_rect = Rect::new(0.0, title_h, win_w, tab_h);
        let explorer_rect = Rect::new(0.0, title_h + tab_h, explorer_w, win_h - title_h - tab_h - status_h);
        let content_x = explorer_w;
        let content_w = win_w - explorer_w - minimap_w;
        let find_rect = Rect::new(content_x, title_h + tab_h, content_w, find_h);
        let editor_rect = Rect::new(content_x, title_h + tab_h + find_h, content_w, win_h - title_h - tab_h - status_h - find_h);
        let window_rect = Rect::new(0.0, 0.0, win_w, win_h);

        let theme_ptr = &self.ctx.as_ref().unwrap().theme as *const Theme;

        // Command palette absorbs all input when open
        {
            let theme = unsafe { &*theme_ptr };
            if self.command_palette.update(0.0) || self.command_palette.visible {
                if self.command_palette.on_event(event, window_rect, theme).is_consumed() {
                    let palette_events = self.command_palette.take_events();
                    for pe in palette_events {
                        self.handle_palette_event(&pe.item_id.clone());
                    }
                    self.needs_redraw = true;
                    return result;
                }
            }
        }

        // Completion popup
        {
            let theme = unsafe { &*theme_ptr };
            if self.completion_popup.is_visible() {
                if self.completion_popup.on_event(event, window_rect, theme).is_consumed() {
                    let comp_events = self.completion_popup.take_events();
                    for ce in comp_events {
                        if let CompletionEvent::Accepted(item) = ce {
                            if let Some(idx) = self.active_editor_idx() {
                                if idx < self.editors.len() {
                                    let insert = item.insert_text.unwrap_or(item.label);
                                    self.editors[idx].doc.insert_text(&insert);
                                }
                            }
                        }
                    }
                    self.needs_redraw = true;
                    return result;
                }
            }
        }

        // Find/replace
        {
            let theme = unsafe { &*theme_ptr };
            if self.find_replace.visible {
                self.find_replace.on_event(event, find_rect, theme);
                self.handle_find_events();
            }
        }

        // Title bar
        {
            let theme = unsafe { &*theme_ptr };
            self.title_bar.on_event(event, title_rect, theme);
        }

        // Tab bar
        {
            let theme = unsafe { &*theme_ptr };
            self.tab_bar.on_event(event, tab_rect, theme);
        }

        // File explorer
        {
            let theme = unsafe { &*theme_ptr };
            self.file_explorer.on_event(event, explorer_rect, theme);
        }

        // Active editor
        {
            let theme = unsafe { &*theme_ptr };
            let active_idx = self.tab_bar.active_id.and_then(|id|
                self.tab_bar.tabs.iter().position(|t| t.id == id)
            );
            if let Some(idx) = active_idx {
                if idx < self.editors.len() {
                    self.editors[idx].on_event(event, editor_rect, theme);
                }
            }
        }

        // Process tab events
        let tab_events: Vec<_> = self.tab_bar.take_events();
        for ev in tab_events {
            match ev {
                crate::ui::widgets::TabBarEvent::TabSelected(id) => {
                    self.tab_bar.active_id = Some(id);
                }
                crate::ui::widgets::TabBarEvent::TabClosed(id) => {
                    let idx = self.tab_bar.tabs.iter().position(|t| t.id == id);
                    self.tab_bar.close_tab(id);
                    if let Some(idx) = idx {
                        if idx < self.editors.len() { self.editors.remove(idx); }
                    }
                }
                crate::ui::widgets::TabBarEvent::NewTabRequested => {
                    let doc = Document::new();
                    let id = next_tab_id();
                    self.tab_bar.add_tab(Tab::new(id, "untitled"));
                    self.tab_bar.active_id = Some(id);
                    self.editors.push(EditorView::new(doc));
                }
            }
        }

        // Process title bar events — MUST happen right after on_event so
        // StartDrag fires within the same OS event callback frame.
        let title_events: Vec<TitleBarEvent> = self.title_bar.take_events();
        for ev in title_events {
            match ev {
                TitleBarEvent::Close => {
                    result.exit = true;
                }
                TitleBarEvent::Minimize => {
                    if let Some(ws) = &self.window_state {
                        ws.window.set_minimized(true);
                    }
                }
                TitleBarEvent::Maximize => {
                    if let Some(ws) = &self.window_state {
                        ws.window.set_maximized(!ws.window.is_maximized());
                    }
                }
                TitleBarEvent::StartDrag => {
                    // Signal caller to call drag_window() immediately
                    result.start_drag = true;
                }
            }
        }

        // Process file explorer events
        let explorer_events: Vec<ExplorerEvent> = self.file_explorer.take_events();
        for ev in explorer_events {
            if let ExplorerEvent::FileSelected(path) = ev {
                self.open_file(path);
            }
        }

        // Handle clipboard operations from the editor
        if let Some(idx) = self.active_editor_idx() {
            if idx < self.editors.len() {
                let editor = &mut self.editors[idx];
                if let Some(op) = editor.take_clipboard_op() {
                    match op {
                        ClipboardOp::Copy => {
                            if let Some(text) = editor.doc.selected_text() {
                                if let Some(cb) = &mut self.clipboard {
                                    let _ = cb.set_text(text);
                                }
                            }
                        }
                        ClipboardOp::Cut => {
                            if let Some(text) = editor.doc.selected_text() {
                                if let Some(cb) = &mut self.clipboard {
                                    let _ = cb.set_text(text);
                                }
                                editor.doc.backspace();
                            }
                        }
                        ClipboardOp::Paste => {
                            if let Some(cb) = &mut self.clipboard {
                                if let Ok(text) = cb.get_text() {
                                    editor.doc.insert_text(&text);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update status bar
        if let Some(idx) = self.active_editor_idx() {
            if idx < self.editors.len() {
                let cur = self.editors[idx].doc.cursor.position;
                let lang = self.editors[idx].doc.language.name().to_string();
                let dirty = self.editors[idx].doc.dirty;
                self.status_bar.set_cursor(cur.line, cur.col);
                self.status_bar.set_language(&lang);
                // Update tab dirty indicator
                if let Some(tab) = self.tab_bar.tabs.get_mut(idx) {
                    tab.dirty = dirty;
                }
            }
        }

        self.needs_redraw = true;
        result
    }

    fn handle_global_shortcut(&mut self, key: &Key, modifiers: &Modifiers) {
        if modifiers.ctrl {
            match key {
                Key::Char('p') | Key::Char('P') if modifiers.shift => {
                    // Ctrl+Shift+P: command palette
                    let items = Self::default_palette_items();
                    self.command_palette.set_items(items);
                    self.command_palette.show();
                }
                Key::Char('p') | Key::Char('P') => {
                    // Ctrl+P: file picker (also uses command palette)
                    let mut items = Self::default_palette_items();
                    // Add open files
                    for editor in &self.editors {
                        if let Some(path) = &editor.doc.path {
                            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                            items.insert(0, PaletteItem {
                                id: format!("file:{}", path.display()),
                                label: name.to_string(),
                                description: path.parent().and_then(|p| p.to_str()).unwrap_or("").to_string(),
                                kind: PaletteKind::RecentFile,
                            });
                        }
                    }
                    self.command_palette.set_items(items);
                    self.command_palette.show();
                }
                Key::Char('f') | Key::Char('F') => { self.find_replace.show(false); }
                Key::Char('h') | Key::Char('H') => { self.find_replace.show(true); }
                Key::Char('b') | Key::Char('B') => { self.file_explorer.toggle_visibility(); }
                Key::Char('n') | Key::Char('N') => {
                    let doc = Document::new();
                    let id = next_tab_id();
                    self.tab_bar.add_tab(Tab::new(id, "untitled"));
                    self.tab_bar.active_id = Some(id);
                    self.editors.push(EditorView::new(doc));
                }
                Key::Char('w') | Key::Char('W') => {
                    if let Some(active_id) = self.tab_bar.active_id {
                        let idx = self.tab_bar.tabs.iter().position(|t| t.id == active_id);
                        self.tab_bar.close_tab(active_id);
                        if let Some(idx) = idx {
                            if idx < self.editors.len() { self.editors.remove(idx); }
                        }
                    }
                }
                Key::Char('+') | Key::Char('=') => {
                    if let Some(ctx) = &mut self.ctx {
                        let new_size = (ctx.theme.font_size + 1.0).min(32.0);
                        ctx.theme.font_size = new_size;
                        ctx.fonts = Fonts::load(new_size, new_size - 1.0);
                    }
                }
                Key::Char('-') | Key::Char('_') => {
                    if let Some(ctx) = &mut self.ctx {
                        let new_size = (ctx.theme.font_size - 1.0).max(8.0);
                        ctx.theme.font_size = new_size;
                        ctx.fonts = Fonts::load(new_size, new_size - 1.0);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClipboardOp {
    Copy,
    Cut,
    Paste,
}

impl ApplicationHandler for MoceApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let win_attrs = WindowAttributes::default()
            .with_title("Moce")
            .with_inner_size(PhysicalSize::new(1280, 820))
            .with_min_inner_size(PhysicalSize::new(400, 300))
            .with_decorations(false);

        let window = match event_loop.create_window(win_attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => { log::error!("Failed to create window: {}", e); event_loop.exit(); return; }
        };

        let sb_ctx = softbuffer::Context::new(window.clone()).expect("softbuffer context");
        let surface = softbuffer::Surface::new(&sb_ctx, window.clone()).expect("softbuffer surface");

        let size = window.inner_size();
        let painter = Painter::new(size.width as i32, size.height as i32);
        let win_size = Size::new(size.width as f32, size.height as f32);

        let theme = Theme::dark();
        let fonts = Fonts::load(theme.font_size, theme.font_size - 1.0);
        let ctx = UiContext::new(theme, fonts, win_size);

        self.ctx = Some(ctx);
        self.window_state = Some(WindowState { window: window.clone(), surface, painter });

        // Try to open the current working directory in the explorer
        if let Ok(cwd) = std::env::current_dir() {
            self.file_explorer.open_path(cwd.clone());
            self.title_bar.subtitle = cwd.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
        }

        self.notifications.info("Welcome to Moce! Press Ctrl+Shift+P for commands.");

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => { event_loop.exit(); }
            WindowEvent::Resized(size) => {
                if let Some(ws) = &mut self.window_state {
                    ws.painter.resize(size.width as i32, size.height as i32);
                }
                if let Some(ctx) = &mut self.ctx {
                    ctx.input.resize(Size::new(size.width as f32, size.height as f32));
                }
                self.dispatch_event(&UiEvent::Resize(Size::new(size.width as f32, size.height as f32)));
                self.needs_redraw = true;
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
                self.last_frame = now;
                let needs = self.tick(dt);
                if needs || self.needs_redraw {
                    self.render();
                }
                if needs {
                    if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = Vec2::new(position.x as f32, position.y as f32);
                if let Some(ctx) = &mut self.ctx { ctx.input.mouse_pos = pos; }
                self.dispatch_event(&UiEvent::MouseMove { pos });
                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    winit::event::MouseButton::Left => crate::ui::widget::MouseButton::Left,
                    winit::event::MouseButton::Right => crate::ui::widget::MouseButton::Right,
                    winit::event::MouseButton::Middle => crate::ui::widget::MouseButton::Middle,
                    _ => crate::ui::widget::MouseButton::Other(0),
                };
                let pos = self.ctx.as_ref().map_or(Vec2::ZERO, |c| c.input.mouse_pos);
                if let Some(ctx) = &mut self.ctx {
                    ctx.input.mouse_buttons.left = btn == crate::ui::widget::MouseButton::Left
                        && state == ElementState::Pressed;
                }

                // ── Double-click detection ─────────────────────────────
                // A double-click on the title bar drag area maximizes/restores.
                if btn == crate::ui::widget::MouseButton::Left && state == ElementState::Pressed {
                    let now = Instant::now();
                    let dt = now.duration_since(self.last_click_time).as_millis();
                    let dist = (pos - self.last_click_pos).length();
                    if dt < 400 && dist < 8.0 {
                        // Double click — check if on title bar
                        let title_h = self.ctx.as_ref().map_or(38.0, |c| c.theme.title_bar_height);
                        if pos.y < title_h && pos.x > 80.0 {
                            if let Some(ws) = &self.window_state {
                                ws.window.set_maximized(!ws.window.is_maximized());
                            }
                        }
                    }
                    self.last_click_time = now;
                    self.last_click_pos  = pos;
                }

                let ev = match state {
                    ElementState::Pressed  => UiEvent::MouseDown { pos, button: btn },
                    ElementState::Released => UiEvent::MouseUp   { pos, button: btn },
                };
                let result = self.dispatch_event(&ev);

                if result.exit {
                    event_loop.exit();
                    return;
                }
                if result.start_drag {
                    // drag_window() MUST be called here — same OS callback, button still down.
                    if let Some(ws) = &self.window_state {
                        let _ = ws.window.drag_window();
                    }
                }

                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
                    MouseScrollDelta::PixelDelta(d)   => Vec2::new(d.x as f32 / 20.0, d.y as f32 / 20.0),
                };
                let pos = self.ctx.as_ref().map_or(Vec2::ZERO, |c| c.input.mouse_pos);
                self.dispatch_event(&UiEvent::Scroll { delta: scroll, pos });
                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                let modifiers = self.ctx.as_ref().map(|c| c.input.modifiers).unwrap_or_default();
                let key = winit_key_to_ui(&key_event.logical_key);
                let ui_mods = Modifiers {
                    shift: modifiers.shift,
                    ctrl: modifiers.ctrl,
                    alt: modifiers.alt,
                    super_key: modifiers.super_key,
                };
                if key_event.state == ElementState::Pressed {
                    self.handle_global_shortcut(&key, &ui_mods);
                    let result = self.dispatch_event(&UiEvent::KeyDown { key, modifiers: ui_mods });
                    if result.exit { event_loop.exit(); return; }
                }
                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            WindowEvent::ModifiersChanged(mods) => {
                if let Some(ctx) = &mut self.ctx {
                    ctx.input.modifiers.shift      = mods.state().shift_key();
                    ctx.input.modifiers.ctrl       = mods.state().control_key();
                    ctx.input.modifiers.alt        = mods.state().alt_key();
                    ctx.input.modifiers.super_key  = mods.state().super_key();
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.dispatch_event(&UiEvent::TextInput(text));
                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            WindowEvent::DroppedFile(path) => {
                self.open_file(path);
                if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let any = self.editors.iter_mut().any(|e| e.update(0.0))
            || self.notifications.has_active();
        if any || self.needs_redraw {
            if let Some(ws) = &self.window_state { ws.window.request_redraw(); }
        }
    }
}

fn winit_key_to_ui(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(NamedKey::ArrowLeft) => Key::Left,
        WinitKey::Named(NamedKey::ArrowRight) => Key::Right,
        WinitKey::Named(NamedKey::ArrowUp) => Key::Up,
        WinitKey::Named(NamedKey::ArrowDown) => Key::Down,
        WinitKey::Named(NamedKey::Home) => Key::Home,
        WinitKey::Named(NamedKey::End) => Key::End,
        WinitKey::Named(NamedKey::PageUp) => Key::PageUp,
        WinitKey::Named(NamedKey::PageDown) => Key::PageDown,
        WinitKey::Named(NamedKey::Enter) => Key::Enter,
        WinitKey::Named(NamedKey::Escape) => Key::Escape,
        WinitKey::Named(NamedKey::Backspace) => Key::Backspace,
        WinitKey::Named(NamedKey::Delete) => Key::Delete,
        WinitKey::Named(NamedKey::Tab) => Key::Tab,
        WinitKey::Named(NamedKey::Space) => Key::Char(' '),
        WinitKey::Named(NamedKey::F1) => Key::F(1),
        WinitKey::Named(NamedKey::F2) => Key::F(2),
        WinitKey::Named(NamedKey::F3) => Key::F(3),
        WinitKey::Named(NamedKey::F4) => Key::F(4),
        WinitKey::Named(NamedKey::F5) => Key::F(5),
        WinitKey::Named(NamedKey::F6) => Key::F(6),
        WinitKey::Named(NamedKey::F7) => Key::F(7),
        WinitKey::Named(NamedKey::F8) => Key::F(8),
        WinitKey::Named(NamedKey::F9) => Key::F(9),
        WinitKey::Named(NamedKey::F10) => Key::F(10),
        WinitKey::Named(NamedKey::F11) => Key::F(11),
        WinitKey::Named(NamedKey::F12) => Key::F(12),
        WinitKey::Character(s) => {
            let mut chars = s.chars();
            if let Some(c) = chars.next() {
                if chars.next().is_none() { return Key::Char(c); }
            }
            Key::Unknown
        }
        _ => Key::Unknown,
    }
}

const WELCOME_TEXT: &str = r#"
  ███╗   ███╗ ██████╗  ██████╗███████╗
  ████╗ ████║██╔═══██╗██╔════╝██╔════╝
  ██╔████╔██║██║   ██║██║     █████╗
  ██║╚██╔╝██║██║   ██║██║     ██╔══╝
  ██║ ╚═╝ ██║╚██████╔╝╚██████╗███████╗
  ╚═╝     ╚═╝ ╚═════╝  ╚═════╝╚══════╝

  A modern code editor built with Rust + Skia

  Getting started
  ───────────────
  Ctrl+Shift+P    Command palette
  Ctrl+P          Quick file open
  Ctrl+N          New file
  Ctrl+O / Drop   Open file
  Ctrl+S          Save
  Ctrl+W          Close tab
  Ctrl+B          Toggle explorer
  Ctrl+F          Find
  Ctrl+H          Find & replace
  Ctrl++/-        Zoom in/out
  Ctrl+Z/Y        Undo / redo
  Ctrl+C/V/X      Copy / paste / cut

"#;
