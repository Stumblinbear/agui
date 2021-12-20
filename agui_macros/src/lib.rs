extern crate proc_macro;

use functional_widget::parse_functional_widget;
use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use widget_derive::parse_widget_derive;

mod builder;
mod functional_widget;
mod layout;
mod widget_derive;

#[proc_macro_attribute]
pub fn functional_widget(args: TokenStream, input: TokenStream) -> TokenStream {
    parse_functional_widget(args, input)
}

#[proc_macro_derive(Widget, attributes(widget))]
pub fn widget_derive(input: TokenStream) -> TokenStream {
    parse_widget_derive(input)
}

#[proc_macro_error]
#[proc_macro]
pub fn build(input: TokenStream) -> TokenStream {
    let mut out = Vec::new();

    let mut tokens = builder::prep_stream(input);

    builder::consume_tree(&mut tokens, &mut out);

    if out.is_empty() {
        abort_call_site! { "cannot build nothing" };
    }

    // panic!("{:#?}", out);

    TokenStream::from_iter(out)
}
