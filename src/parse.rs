use std::str;

pub fn attr<'a>(input: &'a str) -> Result<Vec<String>, String> {
    fn remove_whitespace(i: &[u8]) -> String {
        let ident = i.iter().filter(|c| **c != b' ').cloned().collect();
        String::from_utf8(ident).expect("non-utf8 string")
    }

    fn ident(i: &[u8]) -> (String, &[u8]) {
        if let Some(end) = i.iter().position(|c| *c == b',') {
            (remove_whitespace(&i[..end]), &i[end..])
        }
        else {
            (remove_whitespace(i), &[])
        }
    }

    fn attr_inner<'a>(rest: &'a [u8], traits: &mut Vec<String>) -> Result<(), String> {
        match rest.len() {
            0 => Ok(()),
            _ => {
                match rest[0] as char {
                    // Parse a borrowed reference
                    '&' => {
                        let (ident, rest) = ident(rest);
                        traits.push(ident);

                        attr_inner(rest, traits)
                    },
                    // Parse a regular ident
                    c if c.is_alphabetic() => {
                        let (ident, rest) = ident(rest);
                        traits.push(ident);

                        attr_inner(rest, traits)
                    },
                    _ => attr_inner(&rest[1..], traits)
                }
            }
        }
    }

    let mut traits = vec![];
    let input = input.as_bytes();

    let open = input.iter().position(|c| *c == b'(');
    let close = input.iter().position(|c| *c == b')');

    match (open, close) {
        (Some(open), Some(close)) => attr_inner(&input[open + 1 .. close], &mut traits)?,
        _ => Err("attribute format should be `#[auto_impl(a, b, c)]`")?
    }

    Ok(traits)
}
