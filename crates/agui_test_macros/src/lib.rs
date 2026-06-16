use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitFloat, LitInt, Path, parse_macro_input};

/// Turns a function that returns a widget into a golden test rendered through one or more renderers.
///
/// The function body builds and returns a widget; the macro lays it out, renders it through each named
/// renderer, and compares each result to its golden image, asserting the renderers also agree with each
/// other. Goldens live at `tests/goldens/<renderer>/<fn-name>.png` and are written on first run.
///
/// ```ignore
/// #[golden(renderers(VelloHeadless, DcompCapture), width = 200, height = 100, tolerance = 0.01)]
/// fn orange_half_pane() -> impl Widget<Render: RenderBox> {
///     FractionallySizedBox::new().width_factor(0.5).child(ColoredBox::new(ORANGE))
/// }
/// ```
///
/// Arguments: `renderers(..)` (required, one or more renderer types implementing `GoldenRenderer` and
/// `Default`), `width`/`height` (default 256), and `tolerance` (the fraction of pixels allowed to
/// differ from the golden, default 0).
#[proc_macro_attribute]
pub fn golden(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    let mut width: u32 = 256;
    let mut height: u32 = 256;
    let mut tolerance: f32 = 0.0;
    let mut renderers: Vec<Path> = Vec::new();

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("width") {
            width = meta.value()?.parse::<LitInt>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("height") {
            height = meta.value()?.parse::<LitInt>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("tolerance") {
            tolerance = meta.value()?.parse::<LitFloat>()?.base10_parse()?;
            Ok(())
        } else if meta.path.is_ident("renderers") {
            meta.parse_nested_meta(|nested| {
                renderers.push(nested.path);
                Ok(())
            })
        } else {
            Err(meta.error(
                "unknown `golden` argument; expected `renderers`, `width`, `height`, or `tolerance`",
            ))
        }
    });
    parse_macro_input!(attr with parser);

    if renderers.is_empty() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "`golden` requires at least one renderer, e.g. `#[golden(renderers(VelloHeadless))]`",
        )
        .to_compile_error()
        .into();
    }

    let attrs = &func.attrs;
    let vis = &func.vis;
    let name = &func.sig.ident;
    let name_str = name.to_string();
    let output = &func.sig.output;
    let block = &func.block;

    let renderer_exprs = renderers.iter().map(|path| {
        quote! {
            ::std::boxed::Box::new(<#path as ::core::default::Default>::default())
                as ::std::boxed::Box<dyn ::agui_test::golden::GoldenRenderer>
        }
    });

    quote! {
        #(#attrs)*
        #[test]
        #vis fn #name() {
            fn __golden_widget() #output #block

            let mut __renderers: ::std::vec::Vec<
                ::std::boxed::Box<dyn ::agui_test::golden::GoldenRenderer>,
            > = ::std::vec![ #(#renderer_exprs),* ];

            ::agui_test::golden::run_golden(
                #name_str,
                ::core::concat!(::core::env!("CARGO_MANIFEST_DIR"), "/tests/goldens"),
                #width,
                #height,
                #tolerance,
                __golden_widget(),
                &mut __renderers,
            );
        }
    }
    .into()
}
