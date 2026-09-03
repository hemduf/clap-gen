use std::fmt::Write as _;

pub(crate) fn string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            character if character.is_control() && u32::from(character) <= 0o377 => {
                write!(&mut output, "\\{:03o}", u32::from(character))
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(crate) fn optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "nullptr".to_owned(), string)
}

pub(crate) fn utf8_c_string(value: &str) -> String {
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

pub(crate) fn optional_utf8_c_string(value: Option<&str>) -> String {
    value.map_or_else(|| "nullptr".to_owned(), utf8_c_string)
}

#[cfg(test)]
mod tests {
    use super::{optional_string, optional_utf8_c_string, string, utf8_c_string};

    #[test]
    fn preserves_existing_cpp_literal_rendering_for_metadata() {
        assert_eq!("\"plain\"", string("plain"));
        assert_eq!("\"quote\\\"slash\\\\line\\n\"", string("quote\"slash\\line\n"));
        assert_eq!("\"\\b\\f\\001\"", string("\u{0008}\u{000C}\u{0001}"));
        assert_eq!("\"café\"", string("café"));
        assert_eq!("nullptr", optional_string(None));
        assert_eq!("\"value\"", optional_string(Some("value")));
    }

    #[test]
    fn renders_portable_utf8_c_string_literals_for_abi_surfaces() {
        assert_eq!("\"plain\"", utf8_c_string("plain"));
        assert_eq!("\"quote\\\"slash\\\\line\\n\"", utf8_c_string("quote\"slash\\line\n"));
        assert_eq!("\"\\b\\f\\001\"", utf8_c_string("\u{0008}\u{000C}\u{0001}"));
        assert_eq!("\"caf\\303\\251\"", utf8_c_string("café"));
        assert_eq!("nullptr", optional_utf8_c_string(None));
        assert_eq!("\"value\"", optional_utf8_c_string(Some("value")));
    }
}
