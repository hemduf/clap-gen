use crate::ir::CanonicalIr;

use super::{GeneratedFile, GenerationPlan, OUTPUT_NAMES, metadata_cpp};

pub(crate) fn render(ir: &CanonicalIr) -> GenerationPlan {
    let metadata_header = metadata_cpp::header(ir).into_bytes();
    let metadata_source = metadata_cpp::source(ir).into_bytes();
    let files = OUTPUT_NAMES
        .iter()
        .copied()
        .map(|path| GeneratedFile {
            path,
            bytes: match path {
                "clapgen_metadata.cpp" => metadata_source.clone(),
                "clapgen_metadata.hpp" => metadata_header.clone(),
                _ => Vec::new(),
            },
        })
        .collect();
    GenerationPlan { files }
}
