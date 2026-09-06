use std::cmp::Ordering;
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

fn has_flag(parameter: &crate::ir::ParameterIr, flag: &str) -> bool {
    parameter.flags.iter().any(|candidate| candidate == flag)
}

fn exactly_equal(left: f64, right: f64) -> bool {
    left.partial_cmp(&right) == Some(Ordering::Equal)
}

fn integer_plain_value(value: f64) -> Option<i128> {
    if !value.is_finite() || !exactly_equal(value.fract(), 0.0) {
        return None;
    }
    format!("{value:.0}").parse::<i128>().ok()
}

fn validate_parameter_contract(ir: &CanonicalIr) -> Result<(), String> {
    let bypass_count =
        ir.parameters().iter().filter(|parameter| has_flag(parameter, "bypass")).count();
    if bypass_count > 1 {
        return Err(
            "plugin declares more than one bypass parameter; CLAP permits only one CLAP_PARAM_IS_BYPASS parameter"
                .to_owned(),
        );
    }

    for parameter in ir.parameters() {
        if !parameter.min.is_finite()
            || !parameter.max.is_finite()
            || !parameter.default.is_finite()
        {
            return Err(format!(
                "parameter `{}` has a non-finite range/default; CLAP parameter values must be finite",
                parameter.id
            ));
        }

        let stepped = has_flag(parameter, "stepped");
        let enumeration = has_flag(parameter, "enum");
        let bypass = has_flag(parameter, "bypass");
        let readonly = has_flag(parameter, "readonly");
        let automatable = has_flag(parameter, "automatable");
        let modulatable = has_flag(parameter, "modulatable");
        let poly_automatable = [
            "automatable-per-note-id",
            "automatable-per-key",
            "automatable-per-channel",
            "automatable-per-port",
        ]
        .iter()
        .any(|flag| has_flag(parameter, flag));
        let poly_modulatable = [
            "modulatable-per-note-id",
            "modulatable-per-key",
            "modulatable-per-channel",
            "modulatable-per-port",
        ]
        .iter()
        .any(|flag| has_flag(parameter, flag));

        if enumeration && !stepped {
            return Err(format!(
                "parameter `{}` is enum but not stepped; CLAP_PARAM_IS_ENUM requires CLAP_PARAM_IS_STEPPED",
                parameter.id
            ));
        }
        if bypass
            && (!stepped
                || !exactly_equal(parameter.min, 0.0)
                || !exactly_equal(parameter.max, 1.0))
        {
            return Err(format!(
                "parameter `{}` is bypass but does not use the native stepped 0..1 domain",
                parameter.id
            ));
        }
        if poly_automatable && !automatable {
            return Err(format!(
                "parameter `{}` declares per-* automatable flags without the base automatable capability",
                parameter.id
            ));
        }
        if poly_modulatable && !modulatable {
            return Err(format!(
                "parameter `{}` declares per-* modulatable flags without the base modulatable capability",
                parameter.id
            ));
        }
        if readonly && (automatable || modulatable || poly_automatable || poly_modulatable) {
            return Err(format!(
                "parameter `{}` is readonly but also automatable or modulatable",
                parameter.id
            ));
        }
        if stepped {
            let (Some(minimum), Some(maximum), Some(_default)) = (
                integer_plain_value(parameter.min),
                integer_plain_value(parameter.max),
                integer_plain_value(parameter.default),
            ) else {
                return Err(format!(
                    "parameter `{}` is stepped but its min/max/default are not representable integer plain values",
                    parameter.id
                ));
            };
            if let Some(steps) = parameter.steps {
                let expected = maximum
                    .checked_sub(minimum)
                    .and_then(|span| span.checked_add(1))
                    .ok_or_else(|| {
                        format!(
                            "parameter `{}` stepped domain is too large to represent safely",
                            parameter.id
                        )
                    })?;
                if steps != expected {
                    return Err(format!(
                        "parameter `{}` declares {steps} steps but its integer CLAP domain contains {expected} values",
                        parameter.id
                    ));
                }
            }
        } else if parameter.steps.is_some() {
            return Err(format!(
                "parameter `{}` declares `steps` without the stepped flag",
                parameter.id
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_runtime_ids(ir: &CanonicalIr) -> Result<(), String> {
    let params_enabled =
        ir.stable_extension_items().iter().any(|extension| extension.id == "clap.params");
    if params_enabled {
        validate_parameter_contract(ir)?;
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
    }

    let state_enabled =
        ir.stable_extension_items().iter().any(|extension| extension.id == "clap.state");
    if state_enabled {
        let mut numeric_ids = BTreeSet::new();
        for field in ir.state_fields() {
            let key = field.tag.as_deref().unwrap_or(&field.name);
            let Some(id) = ir
                .persistent_ids()
                .iter()
                .find(|entry| entry.kind == "state-field" && entry.key == key)
            else {
                return Err(format!(
                    "state field `{key}` has no immutable state ID\nhint: run `clapgen ids allocate plugin.ids.kdl state-field {key}` before generating the plugin"
                ));
            };
            if id.value == u32::MAX {
                return Err(format!(
                    "state field `{key}` uses CLAP_INVALID_ID ({})\nhint: allocate a different immutable numeric ID",
                    id.value
                ));
            }
            if !numeric_ids.insert(id.value) {
                return Err(format!(
                    "state field `{key}` collides on immutable state ID {}\nhint: repair plugin.ids.kdl before generating the plugin",
                    id.value
                ));
            }
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
