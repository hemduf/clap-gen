use crate::ir::CanonicalIr;

use super::{
    GeneratedFile, GenerationPlan, OUTPUT_NAMES, depfile, manifest, metadata_cpp, resources_cpp,
    source_map,
};

pub(crate) fn render(ir: &CanonicalIr) -> GenerationPlan {
    let depfile = depfile::render(ir).into_bytes();
    let manifest = manifest::render(ir).into_bytes();
    let metadata_header = metadata_cpp::header(ir).into_bytes();
    let metadata_source = metadata_cpp::source(ir).into_bytes();
    let resources_header = resources_cpp::header(ir).into_bytes();
    let sources = source_map::render(ir).into_bytes();
    let files = OUTPUT_NAMES
        .iter()
        .copied()
        .map(|path| GeneratedFile {
            path,
            bytes: match path {
                "clapgen.d" => depfile.clone(),
                "clapgen.manifest.kdl" => manifest.clone(),
                "clapgen.sources.kdl" => sources.clone(),
                "clapgen_metadata.cpp" => metadata_source.clone(),
                "clapgen_metadata.hpp" => metadata_header.clone(),
                "clapgen_resources.hpp" => resources_header.clone(),
                _ => Vec::new(),
            },
        })
        .collect();
    GenerationPlan { files }
}
