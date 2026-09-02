use std::fmt::Write as _;

use crate::ir::CanonicalIr;

pub(crate) fn render(ir: &CanonicalIr) -> String {
    let mut output = String::from("source-map version=1 {\n");
    for source in ir.sources() {
        writeln!(
            &mut output,
            "    source key={} path={} line={} column={}",
            kdl_string(&source.key),
            kdl_string(&source.path),
            source.line,
            source.column
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("}\n");
    output
}

fn kdl_string(value: &str) -> String {
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
            character if character.is_control() => {
                write!(&mut output, "\\u{{{:x}}}", u32::from(character))
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
