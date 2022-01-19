use crate::canvas::Canvas;

pub trait Renderer<Picture> {
    fn draw(&self, canvas: &Canvas) -> Picture;
    
    fn render(&self, picture: &Picture);
}