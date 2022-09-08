use std::{
    fs::File,
    io::{self, BufReader, Read},
};

use glyph_brush_layout::ab_glyph::{FontArc, InvalidFont};
use slotmap::new_key_type;

use crate::{
    unit::Font,
    util::tree::Tree,
    widget::{BoxedWidget, WidgetId},
};

use super::event::WidgetEvent;

new_key_type! {
    pub struct LayerId;
}

pub struct Layer {}

pub enum RenderEvent {}

#[derive(Default)]
pub struct RenderManager {
    fonts: Vec<FontArc>,

    layer_tree: Tree<LayerId, Layer>,
}

impl RenderManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_fonts(&self) -> &[FontArc] {
        &self.fonts
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<Font> {
        let f = File::open(filename)?;

        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(self.load_font(font))
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> Result<Font, InvalidFont> {
        let font = FontArc::try_from_slice(bytes)?;

        Ok(self.load_font(font))
    }

    pub fn load_font(&mut self, font: FontArc) -> Font {
        let font_id = self.fonts.len();

        self.fonts.push(font.clone());

        Font(font_id, Some(font))
    }

    /// Get the widget build context.
    pub fn get_tree(&self) -> &Tree<LayerId, Layer> {
        &self.layer_tree
    }

    /// Get the widget build context.
    pub fn get_root(&self) -> Option<LayerId> {
        self.layer_tree.get_root()
    }

    pub fn update(
        &mut self,
        widget_tree: &Tree<WidgetId, BoxedWidget>,
        widget_events: &[WidgetEvent],
    ) -> Vec<RenderEvent> {
        let mut render_events = Vec::new();

        for event in widget_events {
            match event {
                WidgetEvent::Spawned {
                    parent_id,
                    widget_id,
                } => {
                    println!("spawned: {:?} {:?}", parent_id, widget_id);
                }

                WidgetEvent::Rebuilt { widget_id } => {
                    println!("rebuilt: {:?}", widget_id);
                }

                WidgetEvent::Reparent {
                    parent_id,
                    widget_id,
                } => todo!(),

                WidgetEvent::Layout { widget_id } => todo!(),
                WidgetEvent::Destroyed { widget_id } => todo!(),
            }
        }

        render_events
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        manager::{RenderManager, WidgetManager},
        widget::{BuildContext, BuildResult, Widget, WidgetBuilder},
    };

    #[derive(Debug, Default)]
    struct TestWidget {
        pub children: Vec<Widget>,
    }

    impl WidgetBuilder for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            (&self.children).into()
        }
    }

    #[test]
    pub fn adding_a_root_widget() {
        let mut widget_manager = WidgetManager::new();
        let mut render_manager = RenderManager::new();

        widget_manager.set_root(TestWidget::default());

        let widget_events = widget_manager.update();

        assert_ne!(
            widget_manager.get_root(),
            None,
            "root widget should have been added"
        );

        if let Some(widget_events) = widget_events {
            render_manager.update(widget_manager.get_tree(), &widget_events);
        }

        assert_ne!(
            render_manager.get_root(),
            None,
            "root layer should have been added"
        );
    }
}
