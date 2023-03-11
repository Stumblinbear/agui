extern crate proc_macro;

mod builder;
mod functional_widget;
mod inherited_widget;
mod stateful_widget;
mod stateless_widget;
mod utils;

use functional_widget::parse_functional_widget;
use inherited_widget::impl_inherited_widget;
use proc_macro::TokenStream;
use stateful_widget::impl_stateful_widget;
use stateless_widget::impl_stateless_widget;

#[proc_macro_attribute]
pub fn functional_widget(args: TokenStream, input: TokenStream) -> TokenStream {
    parse_functional_widget(args.into(), input.into()).into()
}

#[proc_macro_derive(StatelessWidget)]
pub fn stateless_widget(input: TokenStream) -> TokenStream {
    impl_stateless_widget(input.into()).into()
}

#[proc_macro_derive(StatefulWidget)]
pub fn stateful_widget(input: TokenStream) -> TokenStream {
    impl_stateful_widget(input.into()).into()
}

#[proc_macro_derive(InheritedWidget, attributes(child))]
pub fn inherited_widget(input: TokenStream) -> TokenStream {
    impl_inherited_widget(input.into()).into()
}

#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    builder::build_impl(input.into()).into()
}
