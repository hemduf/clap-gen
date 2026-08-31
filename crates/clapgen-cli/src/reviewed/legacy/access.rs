#![allow(dead_code)]

use crate::ir::capabilities::header_for;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIr {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) vendor: String,
    pub(crate) version: String,
    pub(crate) url: Option<String>,
    pub(crate) manual_url: Option<String>,
    pub(crate) support_url: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessorIr {
    pub(crate) class: String,
    pub(crate) features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterIr {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) default: f64,
    pub(crate) flags: Vec<String>,
    pub(crate) unit: Option<String>,
    pub(crate) steps: Option<i128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Direction {
    Input,
    Output,
}

impl Direction {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AudioPortIr {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) direction: Direction,
    pub(crate) channels: i128,
    pub(crate) port_type: Option<String>,
    pub(crate) flags: Vec<String>,
    pub(crate) in_place_pair: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotePortIr {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) direction: Direction,
    pub(crate) dialects: Vec<String>,
    pub(crate) preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteNameIr {
    pub(crate) name: String,
    pub(crate) key: Option<i128>,
    pub(crate) channel: Option<i128>,
    pub(crate) port: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateFieldIr {
    pub(crate) name: String,
    pub(crate) field_type: String,
    pub(crate) default: Option<String>,
    pub(crate) tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuiApiIr {
    pub(crate) name: String,
    pub(crate) floating: bool,
    pub(crate) embedded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceIr {
    pub(crate) path: String,
    pub(crate) mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresetLocationIr {
    pub(crate) name: String,
    pub(crate) kind: Option<String>,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PresetFormatIr {
    pub(crate) name: String,
    pub(crate) extension: Option<String>,
    pub(crate) mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FactoryIr {
    pub(crate) id: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionIr {
    pub(crate) id: String,
    pub(crate) version: Option<String>,
    pub(crate) header: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub(crate) struct TypedIr {
    pub(crate) plugin: PluginIr,
    pub(crate) processor: ProcessorIr,
    pub(crate) parameters: Vec<ParameterIr>,
    pub(crate) audio_ports: Vec<AudioPortIr>,
    pub(crate) note_ports: Vec<NotePortIr>,
    pub(crate) note_names: Vec<NoteNameIr>,
    pub(crate) state_fields: Vec<StateFieldIr>,
    pub(crate) gui_apis: Vec<GuiApiIr>,
    pub(crate) resources: Vec<ResourceIr>,
    pub(crate) preset_locations: Vec<PresetLocationIr>,
    pub(crate) preset_formats: Vec<PresetFormatIr>,
    pub(crate) factories: Vec<FactoryIr>,
    pub(crate) stable_extensions: Vec<ExtensionIr>,
    pub(crate) draft_extensions: Vec<ExtensionIr>,
}

pub(crate) fn build(ir: &super::CanonicalIr, replacements: &[(String, String)]) -> TypedIr {
    TypedIr {
        plugin: PluginIr {
            id: ir.plugin.id.clone(),
            name: ir.plugin.name.clone(),
            vendor: ir.plugin.vendor.clone(),
            version: ir.plugin.version.clone(),
            url: ir.plugin.url.clone(),
            manual_url: ir.plugin.manual_url.clone(),
            support_url: ir.plugin.support_url.clone(),
            description: ir.plugin.description.clone(),
            features: ir.plugin.features.clone(),
        },
        processor: ProcessorIr {
            class: ir.processor.class.clone(),
            features: ir.processor.features.clone(),
        },
        parameters: ir
            .parameters
            .iter()
            .map(|value| ParameterIr {
                id: value.id.clone(),
                name: value.name.clone(),
                min: value.min,
                max: value.max,
                default: value.default,
                flags: value.flags.clone(),
                unit: value.unit.clone(),
                steps: value.steps,
            })
            .collect(),
        audio_ports: ir
            .audio_ports
            .iter()
            .map(|value| AudioPortIr {
                id: restore_id(&value.id, replacements),
                name: value.name.clone(),
                direction: direction(value.direction),
                channels: value.channels,
                port_type: value.port_type.clone(),
                flags: value.flags.clone(),
                in_place_pair: value
                    .in_place_pair
                    .as_deref()
                    .map(|id| restore_id(id, replacements)),
            })
            .collect(),
        note_ports: ir
            .note_ports
            .iter()
            .map(|value| NotePortIr {
                id: restore_id(&value.id, replacements),
                name: value.name.clone(),
                direction: direction(value.direction),
                dialects: value.dialects.clone(),
                preferred: value.preferred.clone(),
            })
            .collect(),
        note_names: ir
            .note_names
            .iter()
            .map(|value| NoteNameIr {
                name: value.name.clone(),
                key: value.key,
                channel: value.channel,
                port: value.port.as_deref().map(|id| restore_id(id, replacements)),
            })
            .collect(),
        state_fields: ir
            .state_fields
            .iter()
            .map(|value| StateFieldIr {
                name: value.name.clone(),
                field_type: value.field_type.clone(),
                default: value.default.clone(),
                tag: value.tag.clone(),
            })
            .collect(),
        gui_apis: ir
            .gui
            .apis
            .iter()
            .map(|value| GuiApiIr {
                name: value.name.clone(),
                floating: value.floating,
                embedded: value.embedded,
            })
            .collect(),
        resources: ir
            .gui
            .resources
            .iter()
            .map(|value| ResourceIr { path: value.path.clone(), mime: value.mime.clone() })
            .collect(),
        preset_locations: ir
            .presets
            .locations
            .iter()
            .map(|value| PresetLocationIr {
                name: value.name.clone(),
                kind: value.kind.clone(),
                path: value.path.clone(),
            })
            .collect(),
        preset_formats: ir
            .presets
            .formats
            .iter()
            .map(|value| PresetFormatIr {
                name: value.name.clone(),
                extension: value.extension.clone(),
                mime: value.mime.clone(),
            })
            .collect(),
        factories: ir
            .factories
            .iter()
            .map(|value| FactoryIr { id: value.id.clone(), kind: value.kind.clone() })
            .collect(),
        stable_extensions: ir
            .stable_extensions
            .iter()
            .map(|value| extension(value, replacements))
            .collect(),
        draft_extensions: ir
            .draft_extensions
            .iter()
            .map(|value| extension(value, replacements))
            .collect(),
    }
}

fn direction(value: super::Direction) -> Direction {
    match value {
        super::Direction::Input => Direction::Input,
        super::Direction::Output => Direction::Output,
    }
}

fn extension(value: &super::ExtensionIr, replacements: &[(String, String)]) -> ExtensionIr {
    let id = restore_id(&value.id, replacements);
    ExtensionIr { header: header_for(&id), id, version: value.version.clone() }
}

fn restore_id(value: &str, replacements: &[(String, String)]) -> String {
    replacements
        .iter()
        .find_map(|(internal, original)| (internal == value).then(|| original.trim().to_owned()))
        .unwrap_or_else(|| value.trim().to_owned())
}
