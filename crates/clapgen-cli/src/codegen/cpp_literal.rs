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

#[cfg(test)]
mod tests {
    use super::{optional_string, string};

    #[test]
    fn renders_portable_cpp_string_literals() {
        assert_eq!("\"plain\"", string("plain"));
        assert_eq!("\"quote\\\"slash\\\\line\\n\"", string("quote\"slash\\line\n"));
        assert_eq!("\"\\b\\f\\001\"", string("\u{0008}\u{000C}\u{0001}"));
        assert_eq!("\"café\"", string("café"));
        assert_eq!("nullptr", optional_string(None));
        assert_eq!("\"value\"", optional_string(Some("value")));
    }
}
