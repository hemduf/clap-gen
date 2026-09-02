use crate::ir::CanonicalIr;

use super::{GeneratedFile, GenerationPlan, OUTPUT_NAMES};

pub(crate) fn render(_ir: &CanonicalIr) -> GenerationPlan {
    let files = OUTPUT_NAMES
        .iter()
        .copied()
        .map(|path| GeneratedFile { path, bytes: Vec::new() })
        .collect();
    GenerationPlan { files }
}
