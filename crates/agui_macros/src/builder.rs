use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{token::Paren, Token};

use crate::utils::resolve_agui_path;

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
        } else if let TokenTree::Punct(punct) = token {
            // Check for `<$name> {`
            if punct.as_char() == '<' {
                match (tokens.next(), tokens.next(), tokens.next()) {
                    (
                        Some(TokenTree::Ident(ident)),
                        Some(TokenTree::Punct(punct)),
                        Some(TokenTree::Group(group)),
                    ) if punct.as_char() == '>' && group.delimiter() == Delimiter::Brace => {
                        parse_widget_struct(ident, group.stream(), output);
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
        } else {
            output.append(token);
        }
    }
}

fn parse_widget_struct(ident: Ident, content: TokenStream2, output: &mut TokenStream2) {
    let span = Span::call_site().located_at(ident.span());

    let mut builder = TokenStream2::new();

    {
        builder.extend(ident.to_token_stream());
        builder.extend(Token![::](span).to_token_stream());
        builder.extend(Ident::new("builder", span).to_token_stream());
        builder.extend(Group::new(Delimiter::Parenthesis, TokenStream2::new()).to_token_stream());

        let mut content = content.into_iter();

        while let Some(token) = content.next() {
            let member = match token {
                TokenTree::Ident(ident) => ident,

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

                    // parse_tree(expr, output)
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

        builder.extend(Token![.](span).to_token_stream());
        builder.extend(Ident::new("build", span).to_token_stream());
        builder.extend(Group::new(Delimiter::Parenthesis, TokenStream2::new()).to_token_stream());
    }

    // Surround the struct with a call to `agui_core::widget::IntoWidget::into_widget(this)`

    output.extend(resolve_agui_path().to_token_stream());
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
