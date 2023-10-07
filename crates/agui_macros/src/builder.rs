use proc_macro2::{
    token_stream::IntoIter, Delimiter, Group, Ident, Punct, Span, TokenStream as TokenStream2,
    TokenTree,
};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{token::Paren, Token};

use crate::utils::resolve_package_path;

fn parse_tree(input: TokenStream2, output: &mut TokenStream2) {
    let mut tokens = input.into_iter();

    while let Some(token) = tokens.next() {
        if let TokenTree::Group(group) = token {
            let mut modified_tokens = TokenStream2::new();

            parse_tree(group.stream(), &mut modified_tokens);

            output.append(TokenTree::Group(Group::new(
                group.delimiter(),
                modified_tokens,
            )));
        } else if let TokenTree::Ident(ref ident) = token {
            parse_const_widget(ident.clone(), &mut tokens, output);
        } else if let TokenTree::Punct(punct) = token {
            parse_widget(punct, &mut tokens, output);
        } else {
            output.append(token);
        }
    }
}

fn parse_const_widget(ident: Ident, tokens: &mut IntoIter, output: &mut TokenStream2) -> bool {
    if ident == "const" {
        match tokens.next() {
            Some(TokenTree::Punct(punct)) => {
                let mut widget_output = TokenStream2::new();

                if parse_widget(punct, tokens, &mut widget_output) {
                    let agui_core = resolve_package_path("agui_core");

                    output.extend(
                        quote!(#agui_core::widget::IntoWidget::into_widget((|| #widget_output) as fn() -> Widget)),
                    );

                    return true;
                } else {
                    output.append(TokenTree::Ident(ident.clone()));
                    output.extend(widget_output);
                }
            }

            next_token => {
                output.append(TokenTree::Ident(ident));
                output.extend(next_token);
            }
        }
    } else {
        output.append(TokenTree::Ident(ident));
    }

    false
}

fn parse_widget(punct: Punct, tokens: &mut IntoIter, output: &mut TokenStream2) -> bool {
    // Check for `<$name> {`
    if punct.as_char() == '<' {
        match (tokens.next(), tokens.next(), tokens.next()) {
            (
                Some(TokenTree::Ident(ident)),
                Some(TokenTree::Punct(punct1)),
                Some(TokenTree::Group(group)),
            ) if punct1.as_char() == '>' && group.delimiter() == Delimiter::Brace => {
                parse_widget_init(ident, None, TokenStream2::default(), group.stream(), output);

                return true;
            }

            (
                Some(TokenTree::Ident(ident)),
                Some(TokenTree::Punct(punct1)),
                Some(TokenTree::Punct(punct2)),
            ) if punct1.as_char() == '>' && punct2.as_char() == ':' => {
                match (tokens.next(), tokens.next(), tokens.next(), tokens.next()) {
                    // `<Widget>::new() { .. }`
                    (
                        Some(TokenTree::Punct(punct3)),
                        Some(TokenTree::Ident(init_func)),
                        Some(TokenTree::Group(init_params)),
                        Some(TokenTree::Group(group)),
                    ) if punct3.as_char() == ':' && group.delimiter() == Delimiter::Brace => {
                        parse_widget_init(
                            ident,
                            Some(init_func),
                            init_params.stream(),
                            group.stream(),
                            output,
                        );

                        return true;
                    }

                    // `<Widget>::new()`
                    (
                        Some(TokenTree::Punct(punct3)),
                        Some(TokenTree::Ident(func_ident)),
                        Some(TokenTree::Group(func_params)),
                        trailing,
                    ) if punct3.as_char() == ':' => {
                        parse_widget_init(
                            ident,
                            Some(func_ident),
                            func_params.stream(),
                            TokenStream2::default(),
                            output,
                        );

                        // Make sure to add the trailing token back
                        output.extend(trailing);

                        return true;
                    }

                    _ => {
                        output.extend(
                            syn::Error::new(
                                punct2.span(),
                                format!("expected `<{}>::func(..)`", ident),
                            )
                            .to_compile_error(),
                        );
                    }
                };
            }

            (first, second, third) => {
                output.append(TokenTree::Punct(punct));
                output.extend(first);
                output.extend(second);
                output.extend(third);
            }
        }
    } else {
        output.append(TokenTree::Punct(punct));
    }

    false
}

fn parse_widget_init(
    widget_ident: Ident,
    init_func: Option<Ident>,
    init_params: TokenStream2,
    content: TokenStream2,
    output: &mut TokenStream2,
) {
    let span = Span::call_site().located_at(widget_ident.span());

    let mut builder = TokenStream2::new();

    let is_using_builder = init_func.is_none() || init_func.as_ref().unwrap() == "builder";

    {
        builder.extend(widget_ident.to_token_stream());
        builder.extend(Token![::](span).to_token_stream());
        builder.extend(
            init_func
                .unwrap_or_else(|| Ident::new("builder", span))
                .to_token_stream(),
        );
        builder.extend(Group::new(Delimiter::Parenthesis, init_params).to_token_stream());

        let mut content = content.into_iter();

        while let Some(token) = content.next() {
            let member = match token {
                TokenTree::Ident(ident) => {
                    if !is_using_builder {
                        // If we used a custom init func, then we need to use the `set_<param>()`
                        // instead of `<param()` method.
                        Ident::new(&format!("set_{}", ident), ident.span())
                    } else {
                        ident
                    }
                }

                _ => {
                    output.extend(
                        syn::Error::new(token.span(), "expected identifier").to_compile_error(),
                    );

                    return;
                }
            };

            builder.extend(Token![.](span).to_token_stream());
            builder.extend(member.to_token_stream());

            let mut expr = TokenStream2::new();

            match content.next() {
                Some(TokenTree::Punct(punct)) if punct.as_char() == ':' => {
                    // Parse the value, since it may also contain widget declarations
                    loop {
                        match content.next() {
                            Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => break,
                            Some(token) => expr.append(token),
                            None => break,
                        }
                    }
                }

                Some(other) => {
                    output.extend(
                        syn::Error::new(other.span(), "expected `:` or `,`").to_compile_error(),
                    );

                    return;
                }

                None => {}
            }

            let mut modified_expr = TokenStream2::new();

            parse_tree(expr, &mut modified_expr);

            builder.extend(Group::new(Delimiter::Parenthesis, modified_expr).to_token_stream());
        }

        if is_using_builder {
            builder.extend(Token![.](span).to_token_stream());
            builder.extend(Ident::new("build", span).to_token_stream());
            builder
                .extend(Group::new(Delimiter::Parenthesis, TokenStream2::new()).to_token_stream());
        }
    }

    // Surround the struct with a call to `agui_core::widget::IntoWidget::into_widget(this)`

    output.extend(resolve_package_path("agui_core").to_token_stream());
    output.extend(Token![::](span).to_token_stream());
    output.extend(Ident::new("widget", span).to_token_stream());
    output.extend(Token![::](span).to_token_stream());
    output.extend(Ident::new("IntoWidget", span).to_token_stream());
    output.extend(Token![::](span).to_token_stream());
    output.extend(Ident::new("into_widget", span).to_token_stream());

    Paren::default().surround(output, |tokens| {
        tokens.extend(builder);
    });
}

pub(crate) fn build_impl(tokens: TokenStream2) -> TokenStream2 {
    let mut modified_tokens = TokenStream2::new();

    parse_tree(tokens, &mut modified_tokens);

    modified_tokens
}
