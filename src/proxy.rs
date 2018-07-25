use std::iter::Peekable;

use proc_macro::{token_stream, TokenStream, TokenTree};

use crate::diag::SpanExt;

/// Types for which a trait can automatically be implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProxyType {
    Ref,
    RefMut,
    Arc,
    Rc,
    Box,
    Fn,
    FnMut,
    FnOnce,
}

impl ProxyType {
    pub(crate) fn is_fn(&self) -> bool {
        match *self {
            ProxyType::Fn | ProxyType::FnMut | ProxyType::FnOnce => true,
            _ => false,
        }
    }
}

/// Parses the attribute token stream into a list of proxy types.
///
/// The attribute token stream is the one in `#[auto_impl(...)]`. It is
/// supposed to be a comma-separated list of possible proxy types. Legal values
/// are `&`, `&mut`, `Box`, `Rc`, `Arc`, `Fn`, `FnMut` and `FnOnce`.
///
/// If the given TokenStream is not valid, errors are emitted as appropriate
/// and `Err(())` is returned.
pub(crate) fn parse_types(args: TokenStream) -> Result<Vec<ProxyType>, ()> {
    let mut out = Vec::new();
    let mut iter = args.into_iter().peekable();

    // While there are still tokens left...
    while iter.peek().is_some() {
        // First, we expect one of the proxy types.
        out.push(eat_type(&mut iter)?);

        // If the next token is a comma, we eat it (trailing commas are
        // allowed). If not, nothing happens (in this case, it's probably the
        // end of the stream, otherwise an error will occur later).
        let comma_next = match iter.peek() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => true,
            _ => false,
        };

        if comma_next {
            let _ = iter.next();
        }
    }

    Ok(out)
}

/// Parses one `ProxyType` from the given token iterator. The iterator must not
/// be empty!
fn eat_type(iter: &mut Peekable<token_stream::IntoIter>) -> Result<ProxyType, ()> {
    const NOTE_TEXT: &str = "\
        attribute format should be `#[auto_impl(<types>)]` where `<types>` is \
        a comma-separated list of types. Allowed values for types: `&`, \
        `&mut`, `Box`, `Rc`, `Arc`, `Fn`, `FnMut` and `FnOnce`.\
    ";
    const EXPECTED_TEXT: &str = "Expected '&' or ident.";

    // We can unwrap because this function requires the iterator to be
    // non-empty.
    let ty = match iter.next().unwrap() {
        TokenTree::Group(group) => {
            group.span()
                .err(format!("unexpected group. {}", EXPECTED_TEXT))
                .note(NOTE_TEXT)
                .emit();

            return Err(());
        }

        TokenTree::Literal(lit) => {
            lit.span()
                .err(format!("unexpected literal. {}", EXPECTED_TEXT))
                .note(NOTE_TEXT)
                .emit();

            return Err(());
        }

        TokenTree::Punct(punct) => {
            // Only '&' are allowed. Everything else leads to an error.
            if punct.as_char() != '&' {
                let msg = format!("unexpected punctuation '{}'. {}", punct, EXPECTED_TEXT);
                punct.span().err(msg).note(NOTE_TEXT).emit();

                return Err(());
            }

            // Check if the next token is `mut`. If not, we will ignore it.
            let is_mut_next = match iter.peek() {
                Some(TokenTree::Ident(id)) if id.to_string() == "mut" => true,
                _ => false,
            };

            if is_mut_next {
                // Eat `mut`
                let _ = iter.next();
                ProxyType::RefMut
            } else {
                ProxyType::Ref
            }
        }

        TokenTree::Ident(ident) => {
            match &*ident.to_string() {
                "Box" => ProxyType::Box,
                "Rc" => ProxyType::Rc,
                "Arc" => ProxyType::Arc,
                "Fn" => ProxyType::Fn,
                "FnMut" => ProxyType::FnMut,
                "FnOnce" => ProxyType::FnOnce,
                _ => {
                    let msg = format!("unexpected '{}'. {}", ident, EXPECTED_TEXT);
                    ident.span()
                        .err(msg)
                        .note(NOTE_TEXT)
                        .emit();

                    return Err(());
                }
            }
        }
    };

    Ok(ty)
}


#[cfg(test)]
mod test {
    use proc_macro::TokenStream;

    use super::parse_types;

    #[test]
    fn empty() {
        assert_eq!(
            parse_types(TokenStream::new()),
            Ok(vec![])
        );
    }

    // Right now, we can't really write useful tests. Many functions from
    // `proc_macro` use a compiler internal session. This session is only valid
    // when we were actually called as a proc macro. We need to add tests once
    // this limitation of `proc_macro` is fixed.
}
