//! Concrete UI widgets for the Moce editor.

pub mod button;
pub mod command_palette;
pub mod editor_view;
pub mod file_explorer;
pub mod find_replace;
pub mod lsp_popup;
pub mod minimap;
pub mod notification;
pub mod scrollbar;
pub mod status_bar;
pub mod tab_bar;
pub mod title_bar;

pub use button::Button;
pub use command_palette::{CommandPalette, PaletteItem, PaletteKind};
pub use editor_view::EditorView;
pub use file_explorer::{ExplorerEvent, FileExplorer};
pub use find_replace::{FindReplaceBar, FindReplaceEvent};
pub use lsp_popup::{CompletionPopup, HoverPopup};
pub use minimap::Minimap;
pub use notification::{NotificationKind, NotificationManager};
pub use scrollbar::Scrollbar;
pub use status_bar::StatusBar;
pub use tab_bar::{Tab, TabBar, TabBarEvent};
pub use title_bar::{TitleBar, TitleBarEvent};
