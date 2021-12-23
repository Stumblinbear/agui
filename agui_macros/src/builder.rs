use std::ops::Index;

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use proc_macro_error::abort;

const EXPECTED_GROUP: &str = "Expected struct block";
const EXPECTED_FIELD: &str = "Expected struct field. Example: `text: \"string\"`";

pub fn prep_stream(stream: TokenStream) -> Vec<TokenTree> {
    let mut tokens = stream.into_iter().collect::<Vec<_>>();

    // Reverse the vec so we can `.pop()` off of it
    tokens.reverse();

    tokens
}

pub fn consume_tree(tokens: &mut Vec<TokenTree>, out: &mut Vec<TokenTree>) {
    consume_expr(tokens, out);

    // Consume the rest of the iterator
    if let Some(token) = tokens.pop() {
        let span = token.span();

        let mut remaining = vec![token];

        remaining.append(tokens);

        abort! { span, format!("remaining tokens: {:#?}", remaining) };
    }

    // Remove trailing comma from the stream if it exists (this make the generation logic easier to grok)
    if !out.is_empty() {
        let token = out.remove(out.len() - 1);

        if !matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            out.push(token);
        }
    }
}

pub fn consume_expr(tokens: &mut Vec<TokenTree>, out: &mut Vec<TokenTree>) -> bool {
    let mut depth = 0;

    while let Some(token) = tokens.pop() {
        match &token {
            TokenTree::Ident(ident) => {
                let name = ident.to_string();

                let first_char = name.index(0..1);

                // If the first token is an capitalized identifier, check if it's referencing an enum variant or func
                if first_char == first_char.to_uppercase() {
                    if let Some(TokenTree::Punct(punc)) = tokens.last() {
                        if punc.as_char() == ':' {
                            // Handle double colons, so it doesn't try to handle (for example) enum variants as a struct
                            if let Some(TokenTree::Punct(punc)) = tokens.get(tokens.len() - 2) {
                                if punc.as_char() == ':' {
                                    out.push(TokenTree::Ident(ident.clone()));

                                    // Pop off the two colons
                                    out.push(tokens.pop().unwrap());
                                    out.push(tokens.pop().unwrap());

                                    // Add the next token
                                    if let Some(token) = tokens.pop() {
                                        out.push(token);
                                    }

                                    // If we've got a group coming up, consume it.
                                    if let Some(TokenTree::Group(_)) = tokens.last() {
                                        // Push the group as-is
                                        out.push(tokens.pop().unwrap());

                                        continue;
                                    }

                                    // Continue as usual... nothing to see, here...

                                    continue;
                                }
                            }
                        }

                        if punc.as_char() != ',' {
                            // If a Punct follows it (that isn't a comma), then it's not a struct. Bail!

                            continue;
                        }
                    }

                    consume_struct(tokens, out, ident.clone());

                    continue;
                }
            }

            // If the token is a bracket, they're probably constructing an array
            TokenTree::Group(group) => {
                if group.delimiter() == Delimiter::Bracket {
                    let mut subtree = Vec::new();

                    let mut subtokens = prep_stream(group.stream());

                    // `consume_expr` only eats until a comma, which is the delimiter for arrays, so we
                    // need to loop until we consume all of the array elements
                    while !subtokens.is_empty() {
                        consume_expr(&mut subtokens, &mut subtree);

                        subtree.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
                    }

                    out.push(TokenTree::Group(Group::new(
                        Delimiter::Bracket,
                        TokenStream::from_iter(subtree),
                    )));

                    continue;
                } else if group.delimiter() == Delimiter::Brace {
                    // If it's a brace, we need to consume its token tree
                    let mut subtree = Vec::new();

                    let mut subtokens = prep_stream(group.stream());

                    consume_tree(&mut subtokens, &mut subtree);

                    out.push(TokenTree::Group(Group::new(
                        Delimiter::Brace,
                        TokenStream::from_iter(subtree),
                    )));

                    continue;
                }
            }

            // A comma indicates the end of what we should be consuming
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                return true;
            }

            _ => {}
        }

        // Any token not caught can be dumped directly
        out.push(token);
    }

    false
}

fn consume_struct(tokens: &mut Vec<TokenTree>, out: &mut Vec<TokenTree>, ident: Ident) {
    // If there was no token following, pretend like it ended with a comma (for consistency in logic)
    let token = if let Some(token) = tokens.pop() {
        token
    } else {
        TokenTree::Punct(Punct::new(',', Spacing::Joint))
    };

    // If we have no token coming up, or we do and it's a comma, then generate the default struct
    if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
        let span = ident.span();

        // `IDENT::default()`

        out.extend(vec![
            TokenTree::Ident(ident),
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            TokenTree::Ident(Ident::new("default", span)),
            TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new())),
        ]);

        // Add the .into() statement
        out.extend(create_into(span));

        out.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
    } else {
        // Start parsing the `{ .. }` group
        let group = if let TokenTree::Group(group) = token {
            if group.delimiter() == Delimiter::Brace {
                group
            } else {
                abort! { group.span(), EXPECTED_GROUP }
            }
        } else {
            abort! { token.span(), EXPECTED_GROUP }
        };

        let mut tokens = prep_stream(group.stream());

        let mut field_block = Vec::new();

        while let Some(token) = tokens.pop() {
            let field_ident = if let TokenTree::Ident(field_ident) = token {
                field_ident
            } else {
                abort! { token.span(), EXPECTED_FIELD }
            };

            if let Some(token) = tokens.pop() {
                if let TokenTree::Punct(c) = &token {
                    // If it's a colon, then we're setting a field value
                    if *c == ':' {
                        field_block.extend(vec![
                            TokenTree::Ident(field_ident),
                            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
                        ]);

                        // Consume the value, and append a comma if it ended with one
                        if consume_expr(&mut tokens, &mut field_block) {
                            field_block.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
                        }

                        continue;
                    }
                }

                // Any token not caught can be dumped directly
                field_block.push(token);
            }
        }

        if !field_block.is_empty() {
            // If the last token was a comma, destroy it
            if let Some(TokenTree::Punct(punct)) = field_block.last() {
                if punct.as_char() == ',' {
                    field_block.pop();
                }
            }

            field_block.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
        }

        // Append `.. IDENT::default()` to the struct initializer
        field_block.extend(create_default(ident.span(), ident.clone()));

        let group = Group::new(Delimiter::Brace, TokenStream::from_iter(field_block));

        let span = group.span();

        out.extend(vec![TokenTree::Ident(ident), TokenTree::Group(group)]);

        // Add the .into() statement
        out.extend(create_into(span));
    }
}

fn create_default(span: Span, ident: Ident) -> TokenStream {
    let mut tokens = TokenStream::new();

    tokens.extend(vec![
        TokenTree::Punct(Punct::new('.', Spacing::Joint)),
        TokenTree::Punct(Punct::new('.', Spacing::Alone)),
        TokenTree::Ident(ident),
        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
        TokenTree::Punct(Punct::new(':', Spacing::Alone)),
        TokenTree::Ident(Ident::new("default", span)),
        TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new())),
    ]);

    tokens
}

fn create_into(span: Span) -> Vec<TokenTree> {
    vec![
        TokenTree::Punct(Punct::new('.', Spacing::Alone)),
        TokenTree::Ident(Ident::new("into", span)),
        TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new())),
    ]
}
