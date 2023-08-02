extern crate proc_macro;

mod builder;
mod inherited_widget;
mod layout_widget;
mod paint_widget;
mod stateful_widget;
mod stateless_widget;
mod utils;

use inherited_widget::impl_inherited_widget;
use layout_widget::impl_layout_widget;
use paint_widget::impl_paint_widget;
use proc_macro::TokenStream;
use stateful_widget::impl_stateful_widget;
use stateless_widget::impl_stateless_widget;

#[proc_macro_derive(StatelessWidget)]
pub fn stateless_widget(input: TokenStream) -> TokenStream {
    impl_stateless_widget(input.into()).into()
}

#[proc_macro_derive(StatefulWidget)]
pub fn stateful_widget(input: TokenStream) -> TokenStream {
    impl_stateful_widget(input.into()).into()
}

#[proc_macro_derive(LayoutWidget)]
pub fn layout_widget(input: TokenStream) -> TokenStream {
    impl_layout_widget(input.into()).into()
}

#[proc_macro_derive(PaintWidget, attributes(child))]
pub fn paint_widget(input: TokenStream) -> TokenStream {
    impl_paint_widget(input.into()).into()
}

#[proc_macro_derive(InheritedWidget, attributes(child))]
pub fn inherited_widget(input: TokenStream) -> TokenStream {
    impl_inherited_widget(input.into()).into()
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    builder::build_impl(input.into()).into()
}
