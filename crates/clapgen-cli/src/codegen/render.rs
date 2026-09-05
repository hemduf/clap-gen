use std::collections::BTreeSet;
use std::path::Path;

use crate::ir::CanonicalIr;

use super::{
    GeneratedFile, GenerationPlan, OUTPUT_NAMES, depfile, descriptor_cpp, entry_cpp, extension_cpp,
    ids_cpp, instance_backend_cpp, manifest, metadata_cpp, processor_cpp, resources_cpp,
    source_map,
};

pub(crate) fn render(ir: &CanonicalIr) -> GenerationPlan {
    let depfile = depfile::render(ir);
    render_with_depfile(ir, depfile.as_bytes())
}

pub(crate) fn render_for_output(
    ir: &CanonicalIr,
    dependency_base: &Path,
    output_directory: &Path,
) -> GenerationPlan {
    let depfile = depfile::render_for_output(ir, dependency_base, output_directory);
    render_with_depfile(ir, depfile.as_bytes())
}

pub(crate) fn render_for_output_checked(
    ir: &CanonicalIr,
    dependency_base: &Path,
    output_directory: &Path,
) -> Result<GenerationPlan, String> {
    validate_runtime_ids(ir)?;
    Ok(render_for_output(ir, dependency_base, output_directory))
}

pub(crate) fn validate_runtime_ids(ir: &CanonicalIr) -> Result<(), String> {
    let params_enabled =
        ir.stable_extension_items().iter().any(|extension| extension.id == "clap.params");
    if !params_enabled {
        return Ok(());
    }

    let mut numeric_ids = BTreeSet::new();
    for parameter in ir.parameters() {
        let Some(id) = ir
            .persistent_ids()
            .iter()
            .find(|entry| entry.kind == "parameter" && entry.key == parameter.id)
        else {
            return Err(format!(
                "parameter `{}` has no immutable CLAP ID\nhint: run `clapgen ids allocate plugin.ids.kdl parameter {}` before generating the plugin",
                parameter.id, parameter.id
            ));
        };
        if id.value == u32::MAX {
            return Err(format!(
                "parameter `{}` uses CLAP_INVALID_ID ({})\nhint: allocate a different immutable numeric ID",
                parameter.id, id.value
            ));
        }
        if !numeric_ids.insert(id.value) {
            return Err(format!(
                "parameter `{}` collides on immutable CLAP ID {}\nhint: repair plugin.ids.kdl before generating the plugin",
                parameter.id, id.value
            ));
        }
    }
    Ok(())
}

fn render_with_depfile(ir: &CanonicalIr, depfile: &[u8]) -> GenerationPlan {
    let descriptor_header = descriptor_cpp::header(ir).into_bytes();
    let entry_source = entry_cpp::source().into_bytes();
    let extension_header = extension_cpp::header(ir).into_bytes();
    let ids_header = ids_cpp::header(ir).into_bytes();
    let instance_backend_header = instance_backend_cpp::header().into_bytes();
    let instance_backend_source = instance_backend_cpp::source().into_bytes();
    let manifest = manifest::render(ir).into_bytes();
    let metadata_header = metadata_cpp::header(ir).into_bytes();
    let metadata_source = metadata_cpp::source(ir).into_bytes();
    let processor_header = processor_cpp::header().into_bytes();
    let resources_header = resources_cpp::header(ir).into_bytes();
    let sources = source_map::render(ir).into_bytes();
    let files = OUTPUT_NAMES
        .iter()
        .copied()
        .map(|path| GeneratedFile {
            path,
            bytes: match path {
                "clapgen.d" => depfile.to_vec(),
                "clapgen.manifest.kdl" => manifest.clone(),
                "clapgen.sources.kdl" => sources.clone(),
                "clapgen_descriptors.hpp" => descriptor_header.clone(),
                "clapgen_entry.cpp" => entry_source.clone(),
                "clapgen_extensions.hpp" => extension_header.clone(),
                "clapgen_ids.hpp" => ids_header.clone(),
                "clapgen_instance_backend.cpp" => instance_backend_source.clone(),
                "clapgen_instance_backend.hpp" => instance_backend_header.clone(),
                "clapgen_metadata.cpp" => metadata_source.clone(),
                "clapgen_metadata.hpp" => metadata_header.clone(),
                "clapgen_processor.hpp" => processor_header.clone(),
                "clapgen_resources.hpp" => resources_header.clone(),
                _ => Vec::new(),
            },
        })
        .collect();
    GenerationPlan { files }
}
