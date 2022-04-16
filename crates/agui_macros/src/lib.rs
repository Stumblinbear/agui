extern crate proc_macro;

use core::panic;

use functional_widget::parse_functional_widget;
use proc_macro::TokenStream;

mod builder;
mod functional_widget;

#[proc_macro_attribute]
pub fn functional_widget(args: TokenStream, input: TokenStream) -> TokenStream {
    parse_functional_widget(args.into(), input.into()).into()
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    builder::build_impl(input.into()).into()
}
