//! View components for the Casette user interface.

pub mod header_view;
pub mod input_view;
pub mod sidebar_view;
pub mod tape_view;

pub use header_view::render_header;
pub use input_view::render_input_pane;
pub use sidebar_view::{render_sidebar, SidebarTab};
pub use tape_view::render_tape;
