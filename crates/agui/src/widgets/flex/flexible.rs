use bon::Builder;

use super::FlexFit;

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Flexible<Child> {
    #[builder(finish_fn)]
    pub child: Child,

    #[builder(default)]
    pub flex: f32,

    #[builder(default)]
    pub fit: FlexFit,
}

impl<Child> From<Child> for Flexible<Child> {
    fn from(child: Child) -> Self {
        Self::builder().child(child)
    }
}
