use std::fmt;
use std::path::Path;

use crate::metadata::ParsedMetadata;

#[path = "reviewed/capabilities.rs"]
mod capabilities;
#[path = "codegen/mod.rs"]
pub(crate) mod codegen;
#[path = "reviewed/provenance.rs"]
mod provenance;
#[path = "reviewed/mod.rs"]
mod reviewed;

pub(crate) use provenance::SourceEntry;
#[allow(unused_imports)]
pub(crate) use reviewed::{
    AudioPortIr, Direction, ExtensionIr, FactoryIr, GuiApiIr, NoteNameIr, NotePortIr, ParameterIr,
    PluginIr, PresetFormatIr, PresetLocationIr, ProcessorIr, ResourceIr, StateFieldIr,
};

#[derive(Debug)]
pub(crate) struct ExtensionSet(usize);

impl ExtensionSet {
    pub(crate) const fn len(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistentIdIr {
    pub(crate) kind: String,
    pub(crate) key: String,
    pub(crate) value: u32,
}

#[allow(dead_code)]
pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
    pub(crate) stable_extensions: ExtensionSet,
    pub(crate) draft_extensions: ExtensionSet,
    semantic: reviewed::CanonicalIr,
    typed: reviewed::TypedIr,
    persistent_ids: Vec<PersistentIdIr>,
    dependencies: Vec<String>,
    sources: Vec<SourceEntry>,
}

impl fmt::Debug for CanonicalIr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CanonicalIr")
            .field("version", &self.version)
            .field("plugin", &self.typed.plugin.id)
            .field("parameters", &self.typed.parameters.len())
            .field("audio_ports", &self.typed.audio_ports.len())
            .field("note_ports", &self.typed.note_ports.len())
            .field("stable_extensions", &self.stable_extensions.len())
            .field("draft_extensions", &self.draft_extensions.len())
            .field("persistent_ids", &self.persistent_ids.len())
            .field("dependencies", &self.dependencies.len())
            .field("sources", &self.sources.len())
            .finish_non_exhaustive()
    }
}

pub(crate) fn build_ir(
    path: &Path,
    source: &str,
    metadata: &ParsedMetadata,
) -> Result<CanonicalIr, String> {
    let bundle = provenance::collect(path, source, metadata)?;
    capabilities::validate(&bundle.documents)?;

    let semantic = reviewed::build_ir(path, source, metadata)?;
    let typed = reviewed::typed_ir(&semantic);
    validate_descriptor_c_strings(path, source, &typed.plugin)?;
    let version = semantic.version;
    let stable_extensions = ExtensionSet(semantic.stable_extensions.len());
    let draft_extensions = ExtensionSet(semantic.draft_extensions.len());
    let mut dependencies = bundle.dependencies;
    let persistent_ids = load_persistent_ids(path, &mut dependencies)?;

    Ok(CanonicalIr {
        version,
        stable_extensions,
        draft_extensions,
        semantic,
        typed,
        persistent_ids,
        dependencies,
        sources: bundle.sources,
    })
}

pub(crate) fn descriptor_c_string_violation(plugin: &PluginIr) -> Option<&'static str> {
    for (field, value) in [
        ("id", Some(plugin.id.as_str())),
        ("name", Some(plugin.name.as_str())),
        ("vendor", Some(plugin.vendor.as_str())),
        ("version", Some(plugin.version.as_str())),
        ("url", plugin.url.as_deref()),
        ("manual-url", plugin.manual_url.as_deref()),
        ("support-url", plugin.support_url.as_deref()),
        ("description", plugin.description.as_deref()),
    ] {
        if value.is_some_and(|value| value.contains('\0')) {
            return Some(field);
        }
    }

    plugin.features.iter().any(|feature| feature.contains('\0')).then_some("feature")
}

fn validate_descriptor_c_strings(
    path: &Path,
    source: &str,
    plugin: &PluginIr,
) -> Result<(), String> {
    let Some(subject) = descriptor_c_string_violation(plugin) else {
        return Ok(());
    };
    Err(descriptor_nul_diagnostic(path, source, subject))
}

fn descriptor_nul_diagnostic(path: &Path, source: &str, subject: &str) -> String {
    let node = if subject == "feature" { "feature" } else { "plugin" };
    let line = source
        .lines()
        .enumerate()
        .find_map(|(index, line)| {
            let line = line.trim_start();
            line.strip_prefix(node)
                .is_some_and(|rest| {
                    rest.is_empty()
                        || rest.starts_with('{')
                        || rest.chars().next().is_some_and(char::is_whitespace)
                })
                .then_some(index + 1)
        })
        .unwrap_or(1);
    let subject =
        if subject == "feature" { "feature".to_owned() } else { format!("field `{subject}`") };
    format!(
        "{}:{line}: plugin descriptor {subject} contains an embedded NUL character\nhint: remove U+0000 because CLAP descriptor fields are C strings and cannot represent embedded NUL bytes",
        path.display()
    )
}

fn load_persistent_ids(
    metadata_path: &Path,
    dependencies: &mut Vec<String>,
) -> Result<Vec<PersistentIdIr>, String> {
    let registry_path =
        metadata_path.parent().unwrap_or_else(|| Path::new(".")).join("plugin.ids.kdl");
    let Some(entries) = crate::ids::read_entries(&registry_path)? else {
        return Ok(Vec::new());
    };

    let dependency = if metadata_path.is_relative() {
        registry_path.to_string_lossy().replace('\\', "/")
    } else {
        "plugin.ids.kdl".to_owned()
    };
    dependencies.push(dependency);

    let mut ids = entries
        .into_iter()
        .filter(|entry| !entry.tombstone)
        .map(|entry| PersistentIdIr { kind: entry.kind, key: entry.key, value: entry.value })
        .collect::<Vec<_>>();
    ids.sort_by(|left, right| {
        (left.value, &left.kind, &left.key).cmp(&(right.value, &right.kind, &right.key))
    });
    Ok(ids)
}

pub(crate) fn serialize_ir_kdl(ir: &CanonicalIr) -> String {
    reviewed::serialize_ir_kdl(&ir.semantic)
}

pub(crate) fn capability_report_kdl(ir: &CanonicalIr) -> String {
    reviewed::capability_report_kdl(&ir.semantic)
}

#[allow(dead_code)]
impl CanonicalIr {
    pub(crate) const fn plugin(&self) -> &PluginIr {
        &self.typed.plugin
    }

    pub(crate) const fn processor(&self) -> &ProcessorIr {
        &self.typed.processor
    }

    pub(crate) fn parameters(&self) -> &[ParameterIr] {
        &self.typed.parameters
    }

    pub(crate) fn audio_ports(&self) -> &[AudioPortIr] {
        &self.typed.audio_ports
    }

    pub(crate) fn note_ports(&self) -> &[NotePortIr] {
        &self.typed.note_ports
    }

    pub(crate) fn note_names(&self) -> &[NoteNameIr] {
        &self.typed.note_names
    }

    pub(crate) fn state_fields(&self) -> &[StateFieldIr] {
        &self.typed.state_fields
    }

    pub(crate) fn gui_apis(&self) -> &[GuiApiIr] {
        &self.typed.gui_apis
    }

    pub(crate) fn resources(&self) -> &[ResourceIr] {
        &self.typed.resources
    }

    pub(crate) fn preset_locations(&self) -> &[PresetLocationIr] {
        &self.typed.preset_locations
    }

    pub(crate) fn preset_formats(&self) -> &[PresetFormatIr] {
        &self.typed.preset_formats
    }

    pub(crate) fn factories(&self) -> &[FactoryIr] {
        &self.typed.factories
    }

    pub(crate) fn stable_extension_items(&self) -> &[ExtensionIr] {
        &self.typed.stable_extensions
    }

    pub(crate) fn draft_extension_items(&self) -> &[ExtensionIr] {
        &self.typed.draft_extensions
    }

    pub(crate) fn persistent_ids(&self) -> &[PersistentIdIr] {
        &self.persistent_ids
    }

    pub(crate) fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub(crate) fn sources(&self) -> &[SourceEntry] {
        &self.sources
    }
}

#[cfg(test)]
#[path = "reviewed/contracts.rs"]
mod contracts;
