extern crate proc_macro;

// use functional_widget::parse_functional_widget;
use proc_macro::TokenStream;
use widget_derive::parse_widget_derive;

mod builder;
// mod functional_widget;
mod widget_derive;

// #[proc_macro_attribute]
// pub fn functional_widget(args: TokenStream, input: TokenStream) -> TokenStream {
//     parse_functional_widget(args, input)
// }

#[proc_macro_derive(Widget, attributes(widget))]
pub fn widget_derive(input: TokenStream) -> TokenStream {
    parse_widget_derive(input)
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    builder::build_impl(input.into()).into()
}
