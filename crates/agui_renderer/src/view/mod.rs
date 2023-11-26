mod current_view;
mod element;
mod widget;

pub use current_view::*;
pub use widget::*;

slotmap::new_key_type! {
    pub struct RenderViewId;
}
