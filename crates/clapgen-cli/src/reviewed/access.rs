#![allow(dead_code)]

use std::cmp::Ordering;

use kdl::{KdlDocument, KdlNode, KdlValue};

use super::capabilities::header_for;
use super::provenance::SourceBundle;

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

pub(crate) fn build(bundle: &SourceBundle) -> Result<TypedIr, String> {
    let root =
        bundle.documents.first().ok_or_else(|| "canonical IR has no root metadata".to_owned())?;
    let plugin_node = root
        .metadata
        .document
        .get("plugin")
        .ok_or_else(|| "canonical IR root is missing plugin descriptor".to_owned())?;
    let processor_node = root
        .metadata
        .document
        .get("processor")
        .ok_or_else(|| "canonical IR root is missing processor declaration".to_owned())?;

    let plugin = PluginIr {
        id: required_string(plugin_node, "id")?,
        name: required_string(plugin_node, "name")?,
        vendor: required_string(plugin_node, "vendor")?,
        version: required_string(plugin_node, "version")?,
        url: string_prop(plugin_node, "url").map(str::to_owned),
        manual_url: string_prop(plugin_node, "manual-url").map(str::to_owned),
        support_url: string_prop(plugin_node, "support-url").map(str::to_owned),
        description: string_prop(plugin_node, "description").map(str::to_owned),
        features: child_values(plugin_node, "feature"),
    };
    let processor = ProcessorIr {
        class: required_string(processor_node, "class")?,
        features: named_list(processor_node, "features"),
    };

    let mut parameters = Vec::new();
    let mut audio_ports = Vec::new();
    let mut note_ports = Vec::new();
    let mut note_names = Vec::new();
    let mut state_fields = Vec::new();
    let mut gui_apis = Vec::new();
    let mut resources = Vec::new();
    let mut preset_locations = Vec::new();
    let mut preset_formats = Vec::new();
    let mut factories = Vec::new();
    let mut stable_extensions = Vec::new();
    let mut draft_extensions = Vec::new();

    for document in &bundle.documents {
        for node in section_children(&document.metadata.document, "parameters")
            .filter(|node| node.name().value() == "param")
        {
            let id = required_string(node, "id")?;
            parameters.push(ParameterIr {
                name: display_name(node, &id),
                min: required_number(node, "min", &id)?,
                max: required_number(node, "max", &id)?,
                default: required_number(node, "default", &id)?,
                flags: named_list(node, "flags"),
                unit: string_prop(node, "unit").map(token),
                steps: integer_prop(node, "steps"),
                id,
            });
        }

        for node in section_children(&document.metadata.document, "audio-ports") {
            let Some(direction) = direction(node) else {
                continue;
            };
            let id = required_string(node, "id")?;
            audio_ports.push(AudioPortIr {
                name: display_name(node, &id),
                direction,
                channels: required_integer(node, "channels", &id)?,
                port_type: string_prop(node, "type").map(token),
                flags: named_list(node, "flags"),
                in_place_pair: string_prop(node, "in-place-pair").map(str::to_owned),
                id,
            });
        }

        for node in section_children(&document.metadata.document, "note-ports") {
            if let Some(direction) = direction(node) {
                let id = required_string(node, "id")?;
                note_ports.push(NotePortIr {
                    name: display_name(node, &id),
                    direction,
                    dialects: named_list(node, "dialects"),
                    preferred: string_prop(node, "preferred").map(token),
                    id,
                });
                continue;
            }
            if node.name().value() == "note-name" {
                let name = first_string(node)
                    .ok_or_else(|| "note-name is missing its display name".to_owned())?;
                note_names.push(NoteNameIr {
                    name: name.to_owned(),
                    key: integer_prop(node, "key"),
                    channel: integer_prop(node, "channel"),
                    port: string_prop(node, "port").map(str::to_owned),
                });
            }
        }

        for node in section_children(&document.metadata.document, "state")
            .filter(|node| node.name().value() == "field")
        {
            let name = first_string(node)
                .or_else(|| string_prop(node, "name"))
                .ok_or_else(|| "state field is missing its name".to_owned())?;
            state_fields.push(StateFieldIr {
                name: name.to_owned(),
                field_type: string_prop(node, "type").map(token).unwrap_or_default(),
                default: prop(node, "default").map(value_text),
                tag: string_prop(node, "tag").map(str::to_owned),
            });
        }

        for node in section_children(&document.metadata.document, "gui") {
            match node.name().value() {
                "api" => gui_apis.push(GuiApiIr {
                    name: first_string(node)
                        .or_else(|| string_prop(node, "name"))
                        .unwrap_or("default")
                        .to_owned(),
                    floating: bool_prop(node, "floating").unwrap_or(false),
                    embedded: bool_prop(node, "embedded").unwrap_or(true),
                }),
                "resource" => {
                    if let Some(path) = first_string(node).or_else(|| string_prop(node, "path")) {
                        resources.push(ResourceIr {
                            path: normalize_path(path),
                            mime: string_prop(node, "mime").map(token),
                        });
                    }
                }
                _ => {}
            }
        }

        for node in section_children(&document.metadata.document, "presets") {
            match node.name().value() {
                "location" => preset_locations.push(PresetLocationIr {
                    name: first_string(node).unwrap_or("default").to_owned(),
                    kind: string_prop(node, "kind").map(token),
                    path: string_prop(node, "path").map(normalize_path),
                }),
                "format" => preset_formats.push(PresetFormatIr {
                    name: first_string(node).unwrap_or("default").to_owned(),
                    extension: string_prop(node, "extension")
                        .map(|value| value.trim_start_matches('.').to_ascii_lowercase()),
                    mime: string_prop(node, "mime").map(token),
                }),
                _ => {}
            }
        }

        for node in section_children(&document.metadata.document, "factories")
            .filter(|node| node.name().value() == "factory")
        {
            if let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) {
                factories.push(FactoryIr {
                    id: id.to_owned(),
                    kind: string_prop(node, "kind").map_or_else(|| "plugin".to_owned(), token),
                });
            }
        }

        for node in section_children(&document.metadata.document, "extensions")
            .filter(|node| node.name().value() == "enable")
        {
            let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) else {
                continue;
            };
            let extension = ExtensionIr {
                id: id.trim().to_owned(),
                version: string_prop(node, "version").map(str::to_owned),
                header: header_for(id),
            };
            if bool_prop(node, "draft").unwrap_or(false) {
                draft_extensions.push(extension);
            } else {
                stable_extensions.push(extension);
            }
        }
    }

    parameters.sort_by(|a, b| a.id.cmp(&b.id));
    audio_ports.sort_by(compare_audio_ports);
    note_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    note_names.sort_by(|a, b| {
        (&a.name, a.key, a.channel, &a.port).cmp(&(&b.name, b.key, b.channel, &b.port))
    });
    state_fields.sort_by(|a, b| a.name.cmp(&b.name));
    gui_apis.sort_by(|a, b| a.name.cmp(&b.name));
    resources.sort_by(|a, b| a.path.cmp(&b.path));
    preset_locations.sort_by(|a, b| a.name.cmp(&b.name));
    preset_formats.sort_by(|a, b| a.name.cmp(&b.name));
    factories.sort_by(|a, b| a.id.cmp(&b.id));
    stable_extensions.sort_by(|a, b| a.id.cmp(&b.id));
    draft_extensions.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(TypedIr {
        plugin,
        processor,
        parameters,
        audio_ports,
        note_ports,
        note_names,
        state_fields,
        gui_apis,
        resources,
        preset_locations,
        preset_formats,
        factories,
        stable_extensions,
        draft_extensions,
    })
}

fn compare_audio_ports(a: &AudioPortIr, b: &AudioPortIr) -> Ordering {
    let a_main = a.flags.iter().any(|flag| flag == "main");
    let b_main = b.flags.iter().any(|flag| flag == "main");
    (a.direction, !a_main, &a.id).cmp(&(b.direction, !b_main, &b.id))
}

fn section_children<'a>(
    document: &'a KdlDocument,
    section: &'a str,
) -> impl Iterator<Item = &'a KdlNode> {
    document
        .nodes()
        .iter()
        .filter(move |node| node.name().value() == section)
        .flat_map(|node| node.children().into_iter().flat_map(KdlDocument::nodes))
}

fn direction(node: &KdlNode) -> Option<Direction> {
    match node.name().value() {
        "input" => Some(Direction::Input),
        "output" => Some(Direction::Output),
        _ => None,
    }
}

fn required_string(node: &KdlNode, key: &str) -> Result<String, String> {
    string_prop(node, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("node `{}` is missing string property `{key}`", node.name().value()))
}

fn required_number(node: &KdlNode, key: &str, subject: &str) -> Result<f64, String> {
    prop(node, key)
        .and_then(number)
        .ok_or_else(|| format!("`{subject}` is missing numeric property `{key}`"))
}

fn required_integer(node: &KdlNode, key: &str, subject: &str) -> Result<i128, String> {
    integer_prop(node, key)
        .ok_or_else(|| format!("`{subject}` is missing integer property `{key}`"))
}

fn display_name(node: &KdlNode, id: &str) -> String {
    first_string(node).or_else(|| string_prop(node, "name")).unwrap_or(id).trim().to_owned()
}

fn named_list(node: &KdlNode, key: &str) -> Vec<String> {
    let mut values = string_prop(node, key)
        .map(|value| {
            value.split(',').map(token).filter(|value| !value.is_empty()).collect::<Vec<_>>()
        })
        .unwrap_or_default();
    values.sort();
    values.dedup();
    values
}

fn child_values(node: &KdlNode, name: &str) -> Vec<String> {
    let mut values = node
        .children()
        .into_iter()
        .flat_map(KdlDocument::nodes)
        .filter(|child| child.name().value() == name)
        .filter_map(first_string)
        .map(token)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn first_string(node: &KdlNode) -> Option<&str> {
    node.entries().iter().find_map(|entry| {
        if entry.name().is_some() {
            return None;
        }
        match entry.value() {
            KdlValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    })
}

fn prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlValue> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        (name.value() == key).then_some(entry.value())
    })
}

fn string_prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> {
    match prop(node, key)? {
        KdlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn integer_prop(node: &KdlNode, key: &str) -> Option<i128> {
    match prop(node, key)? {
        KdlValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn bool_prop(node: &KdlNode, key: &str) -> Option<bool> {
    match prop(node, key)? {
        KdlValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn number(value: &KdlValue) -> Option<f64> {
    match value {
        KdlValue::Float(value) if value.is_finite() => Some(*value),
        KdlValue::Integer(value) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn value_text(value: &KdlValue) -> String {
    match value {
        KdlValue::String(value) => quote(value),
        KdlValue::Integer(value) => value.to_string(),
        KdlValue::Float(value) => {
            if *value == 0.0 {
                "0".to_owned()
            } else {
                value.to_string()
            }
        }
        KdlValue::Bool(value) => {
            if *value {
                "#true".to_owned()
            } else {
                "#false".to_owned()
            }
        }
        KdlValue::Null => "#null".to_owned(),
    }
}

fn token(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let absolute = normalized.starts_with('/');
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            ".." => parts.push(".."),
            value => parts.push(value),
        }
    }
    let joined = parts.join("/");
    if absolute { format!("/{joined}") } else { joined }
}

fn quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}
