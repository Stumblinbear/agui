use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, Ident, ItemFn};

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

// #[functional_widget]
// pub fn Button(ctx: &WidgetContext, layout: Ref<Layout>, color: Color, child: WidgetRef) {
//     let hovering = ctx.computed(|ctx| {
//         if let Some(hovering) = ctx.get_global::<Hovering>() {
//             hovering.read().is_hovering(ctx)
//         } else {
//             false
//         }
//     });

//     ctx.set_layout(Ref::clone(&layout));

//     build! {
//         Quad {
//             layout: Layout {
//                 sizing: Sizing::Fill
//             },
//             color: if hovering {
//                 Color::Green
//             }else{
//                 Color::White
//             },
//             child: (&child).into()
//         }
//     }
// }
