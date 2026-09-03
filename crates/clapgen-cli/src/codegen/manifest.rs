use std::fmt::Write as _;

use crate::ir::CanonicalIr;

use super::OUTPUT_NAMES;
use super::dependency;
use super::source_map::kdl_string;

pub(crate) fn render(ir: &CanonicalIr) -> String {
    let mut output = String::from("generation-manifest version=1 {\n");
    for path in OUTPUT_NAMES {
        writeln!(&mut output, "    output {}", kdl_string(path))
            .expect("writing to String cannot fail");
    }
    for path in dependency::collect(ir) {
        writeln!(&mut output, "    dependency {}", kdl_string(&path))
            .expect("writing to String cannot fail");
    }
    output.push_str("}\n");
    output
}
