use crate::ir::CanonicalIr;

use super::dependency;

const TARGET: &str = "clapgen.manifest.kdl";

pub(crate) fn render(ir: &CanonicalIr) -> String {
    let dependencies = dependency::collect(ir)
        .iter()
        .map(|path| dependency::depfile_escape(path))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{TARGET}: {dependencies}\n")
}
