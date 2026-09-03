use std::fmt::Write as _;

pub(crate) fn string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for byte in value.as_bytes() {
        match *byte {
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0x08 => output.push_str("\\b"),
            0x0C => output.push_str("\\f"),
            byte if byte.is_ascii_graphic() || byte == b' ' => output.push(char::from(byte)),
            byte => {
                write!(&mut output, "\\{byte:03o}").expect("writing to String cannot fail");
            }
        }
    }
    output.push('"');
    output
}

pub(crate) fn optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "nullptr".to_owned(), string)
}

#[cfg(test)]
mod tests {
    use super::{optional_string, string};

    #[test]
    fn renders_portable_cpp_string_literals() {
        assert_eq!("\"plain\"", string("plain"));
        assert_eq!("\"quote\\\"slash\\\\line\\n\"", string("quote\"slash\\line\n"));
        assert_eq!("\"\\b\\f\\001\"", string("\u{0008}\u{000C}\u{0001}"));
        assert_eq!("\"caf\\303\\251\"", string("café"));
        assert_eq!("nullptr", optional_string(None));
        assert_eq!("\"value\"", optional_string(Some("value")));
    }
}
