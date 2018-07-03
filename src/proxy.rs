use std::{
    iter::Peekable,
};

use proc_macro::{
    TokenStream, TokenTree,
    token_stream,
};


/// Types for which a trait can automatically be implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProxyType {
    Arc,
    Rc,
    Box,
    Fn,
    FnMut,
    FnOnce,
    Ref,
    RefMut,
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
                .error(format!("unexpected group. {}", EXPECTED_TEXT))
                .note(NOTE_TEXT)
                .emit();

            return Err(());
        }

        TokenTree::Literal(lit) => {
            lit.span()
                .error(format!("unexpected literal. {}", EXPECTED_TEXT))
                .note(NOTE_TEXT)
                .emit();

            return Err(());
        }

        TokenTree::Punct(punct) => {
            // Only '&' are allowed. Everything else leads to an error.
            if punct.as_char() != '&' {
                let msg = format!("unexpected punctuation '{}'. {}", punct, EXPECTED_TEXT);
                punct.span()
                    .error(msg)
                    .note(NOTE_TEXT)
                    .emit();

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
                        .error(msg)
                        .note(NOTE_TEXT)
                        .emit();

                    return Err(());
                }
            }
        }
    };

    Ok(ty)
}

// Right now, these tests fail because `TokenStream::from_str` panics due to
// an internal error. It is only supposed to be called with a valid session,
// as a "real" macro invocation.
#[cfg(test)]
mod test {
    use std::str::FromStr;
    use proc_macro::TokenStream;

    use super::{ProxyType, parse_types};


    #[test]
    fn empty() {
        assert_eq!(
            parse_types(TokenStream::new()),
            Ok(vec![])
        );
    }

    #[test]
    fn single_ref() {
        assert_eq!(
            parse_types(TokenStream::from_str("&").unwrap()),
            Ok(vec![ProxyType::Ref])
        );
    }
}
