use super::{WidgetState, WidgetView};

pub trait InheritedWidget: WidgetView + WidgetState {}
