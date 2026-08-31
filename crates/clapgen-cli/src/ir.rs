use std::fmt;
use std::path::Path;

use crate::metadata::ParsedMetadata;

#[path = "reviewed/capabilities.rs"]
mod capabilities;
#[path = "reviewed/provenance.rs"]
mod provenance;
#[path = "reviewed/mod.rs"]
mod reviewed;

#[allow(unused_imports)]
pub(crate) use reviewed::{
    AudioPortIr, Direction, ExtensionIr, FactoryIr, GuiApiIr, NoteNameIr, NotePortIr, ParameterIr,
    PluginIr, PresetFormatIr, PresetLocationIr, ProcessorIr, ResourceIr, StateFieldIr,
};
pub(crate) use provenance::SourceEntry;

#[derive(Debug)]
pub(crate) struct ExtensionSet(usize);

impl ExtensionSet {
    pub(crate) const fn len(&self) -> usize {
        self.0
    }
}

#[allow(dead_code)]
pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
    pub(crate) stable_extensions: ExtensionSet,
    pub(crate) draft_extensions: ExtensionSet,
    semantic: reviewed::CanonicalIr,
    typed: reviewed::TypedIr,
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
    let version = semantic.version;
    let stable_extensions = ExtensionSet(semantic.stable_extensions.len());
    let draft_extensions = ExtensionSet(semantic.draft_extensions.len());

    Ok(CanonicalIr {
        version,
        stable_extensions,
        draft_extensions,
        semantic,
        typed,
        dependencies: bundle.dependencies,
        sources: bundle.sources,
    })
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
