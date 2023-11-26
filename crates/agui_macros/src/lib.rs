extern crate proc_macro;

use proc_macro::TokenStream;

mod builder;
mod inherited_widget;
mod props;
mod render_object_widget;
mod stateful_widget;
mod stateless_widget;
mod utils;

use inherited_widget::impl_inherited_widget;
use props::impl_widget_props;
use render_object_widget::impl_render_object_widget;
use stateful_widget::impl_stateful_widget;
use stateless_widget::impl_stateless_widget;

#[proc_macro_derive(WidgetProps, attributes(props, prop))]
pub fn widget_props(input: TokenStream) -> TokenStream {
    impl_widget_props(input.into()).into()
}

#[proc_macro_derive(StatelessWidget, attributes(props, prop))]
pub fn stateless_widget(input: TokenStream) -> TokenStream {
    impl_stateless_widget(input.into()).into()
}

#[proc_macro_derive(StatefulWidget, attributes(props, prop))]
pub fn stateful_widget(input: TokenStream) -> TokenStream {
    impl_stateful_widget(input.into()).into()
}

#[proc_macro_derive(RenderObjectWidget, attributes(props, prop))]
pub fn render_object_widget(input: TokenStream) -> TokenStream {
    impl_render_object_widget(input.into()).into()
}

#[proc_macro_derive(InheritedWidget, attributes(props, prop))]
pub fn inherited_widget(input: TokenStream) -> TokenStream {
    impl_inherited_widget(input.into()).into()
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    builder::build_impl(input.into()).into()
}
