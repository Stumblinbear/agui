use darling::FromMeta;
use proc_macro::{TokenStream};
use syn::{parse_macro_input, AttributeArgs, ItemFn, Ident};

#[derive(Debug, FromMeta)]
struct MacroArgs {
    #[darling(default)]
    layout: Option<Ident>,
}

pub fn parse_functional_widget(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let _input = parse_macro_input!(input as ItemFn);

    let _args = match MacroArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    todo!()
}