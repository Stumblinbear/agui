use agpu::RenderPass;
use agui::{widget::WidgetID, WidgetManager};

pub trait RenderWidget {
    fn draw(&self, manager: &WidgetManager, widget_id: WidgetID);

    fn render(&self, pass: &mut RenderPass);
}

#[derive(Default)]
pub struct RenderQuad {
    
}

impl RenderWidget for RenderQuad {
    fn draw(&self, manager: &WidgetManager, widget_id: WidgetID) {
        let rect = manager.get_rect(&widget_id).expect("quad has no rect");

        
    }

    fn render(&self, pass: &mut RenderPass) {
        
    }
}