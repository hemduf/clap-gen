use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kdl::{KdlNode, KdlValue};

use crate::metadata::ParsedMetadata;

const IR_VERSION: u32 = 1;
const PARAM_FLAGS: &[&str] = &[
    "automatable",
    "bypass",
    "enum",
    "hidden",
    "modulatable",
    "periodic",
    "readonly",
    "stepped",
];
const AUDIO_PORT_FLAGS: &[&str] = &["main", "requires-common-sample-size"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
    pub(crate) plugin: PluginIr,
    pub(crate) processor: ProcessorIr,
    pub(crate) parameters: Vec<ParameterIr>,
    pub(crate) audio_ports: Vec<AudioPortIr>,
    pub(crate) note_ports: Vec<NotePortIr>,
    pub(crate) note_names: Vec<NoteNameIr>,
    pub(crate) state_fields: Vec<StateFieldIr>,
    pub(crate) gui_apis: Vec<GuiApiIr>,
    pub(crate) gui_resources: Vec<ResourceIr>,
    pub(crate) preset_locations: Vec<PresetLocationIr>,
    pub(crate) preset_formats: Vec<PresetFormatIr>,
    pub(crate) factories: Vec<FactoryIr>,
    pub(crate) stable_extensions: Vec<ExtensionIr>,
    pub(crate) draft_extensions: Vec<ExtensionIr>,
    pub(crate) imports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIr {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) vendor: String,
    pub(crate) version: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Direction {
    Input,
    Output,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
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
    pub(crate) draft: bool,
}

pub(crate) fn build_ir(
    path: &Path,
    source: &str,
    metadata: &ParsedMetadata,
) -> Result<CanonicalIr, String> {
    let document = &metadata.document;
    let plugin_node = document.get("plugin").ok_or_else(|| {
        semantic_error(path, source, "plugin", "missing plugin descriptor", "add a `plugin` node")
    })?;
    let processor_node = document.get("processor").ok_or_else(|| {
        semantic_error(
            path,
            source,
            "processor",
            "missing processor declaration",
            "add `processor class=\"ProcessorClass\"`",
        )
    })?;

    let plugin = PluginIr {
        id: required_string_property(path, source, plugin_node, "id")?,
        name: required_string_property(path, source, plugin_node, "name")?,
        vendor: required_string_property(path, source, plugin_node, "vendor")?,
        version: required_string_property(path, source, plugin_node, "version")?,
        features: child_string_values(plugin_node, "feature"),
    };
    let processor = ProcessorIr {
        class: required_string_property(path, source, processor_node, "class")?,
        features: optional_named_list(path, source, processor_node, "features", "processor")?,
    };

    let mut parameters = build_parameters(path, source, document.get("parameters"))?;
    let mut audio_ports = build_audio_ports(path, source, document.get("audio-ports"))?;
    let (mut note_ports, mut note_names) =
        build_note_ports(path, source, document.get("note-ports"))?;
    let mut state_fields = build_state(path, source, document.get("state"))?;
    let (mut gui_apis, mut gui_resources) = build_gui(path, source, document.get("gui"))?;
    let (mut preset_locations, mut preset_formats) =
        build_presets(path, source, document.get("presets"))?;
    let mut factories = build_factories(path, source, document.get("factories"))?;
    let (mut stable_extensions, mut draft_extensions) =
        build_extensions(path, source, document.get("extensions"))?;

    sort_and_validate_unique(path, source, &mut parameters, |value| &value.id, "parameter")?;
    audio_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    validate_unique_ids(path, source, audio_ports.iter().map(|value| value.id.as_str()), "audio port")?;
    note_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    validate_unique_ids(path, source, note_ports.iter().map(|value| value.id.as_str()), "note port")?;
    note_names.sort_by(|a, b| (&a.name, a.key, a.channel, &a.port).cmp(&(&b.name, b.key, b.channel, &b.port)));
    state_fields.sort_by(|a, b| a.name.cmp(&b.name));
    validate_unique_ids(path, source, state_fields.iter().map(|value| value.name.as_str()), "state field")?;
    gui_apis.sort_by(|a, b| a.name.cmp(&b.name));
    gui_resources.sort_by(|a, b| a.path.cmp(&b.path));
    preset_locations.sort_by(|a, b| a.name.cmp(&b.name));
    preset_formats.sort_by(|a, b| a.name.cmp(&b.name));
    factories.sort_by(|a, b| a.id.cmp(&b.id));
    validate_unique_ids(path, source, factories.iter().map(|value| value.id.as_str()), "factory")?;
    stable_extensions.sort_by(|a, b| a.id.cmp(&b.id));
    draft_extensions.sort_by(|a, b| a.id.cmp(&b.id));

    validate_audio_references(path, source, &audio_ports)?;
    validate_note_references(path, source, &note_ports, &note_names)?;
    validate_capability_dependencies(
        path,
        source,
        &stable_extensions,
        &draft_extensions,
        &note_ports,
        &gui_apis,
    )?;

    let mut imports = metadata
        .imports
        .iter()
        .map(|value| normalize_path(&value.to_string_lossy()))
        .collect::<Vec<_>>();
    imports.sort();
    imports.dedup();

    Ok(CanonicalIr {
        version: IR_VERSION,
        plugin,
        processor,
        parameters,
        audio_ports,
        note_ports,
        note_names,
        state_fields,
        gui_apis,
        gui_resources,
        preset_locations,
        preset_formats,
        factories,
        stable_extensions,
        draft_extensions,
        imports,
    })
}

fn build_parameters(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<ParameterIr>, String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(Vec::new());
    };
    children
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "param")
        .map(|node| {
            let id = required_string_property(path, source, node, "id")?;
            let name = first_string_argument(node)
                .or_else(|| string_property(node, "name"))
                .unwrap_or(&id)
                .trim()
                .to_owned();
            let min = required_number_property(path, source, node, "min", &id)?;
            let max = required_number_property(path, source, node, "max", &id)?;
            let default = required_number_property(path, source, node, "default", &id)?;
            if min > max || default < min || default > max {
                return Err(semantic_error(
                    path,
                    source,
                    "param",
                    &format!(
                        "parameter `{id}` has invalid range/default: min={min}, default={default}, max={max}"
                    ),
                    "require min <= default <= max",
                ));
            }
            let flags = named_flags(path, source, node, "flags", &id, PARAM_FLAGS)?;
            let unit = string_property(node, "unit").map(normalize_token);
            let steps = optional_integer_property(path, source, node, "steps", &id)?;
            if steps.is_some_and(|value| value < 1) {
                return Err(semantic_error(
                    path,
                    source,
                    "param",
                    &format!("parameter `{id}` has invalid `steps`"),
                    "use a positive integer step count",
                ));
            }
            Ok(ParameterIr {
                id,
                name,
                min,
                max,
                default,
                flags,
                unit,
                steps,
            })
        })
        .collect()
}

fn build_audio_ports(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<AudioPortIr>, String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(Vec::new());
    };
    children
        .nodes()
        .iter()
        .filter_map(|node| match node.name().value() {
            "input" => Some((node, Direction::Input)),
            "output" => Some((node, Direction::Output)),
            _ => None,
        })
        .map(|(node, direction)| {
            let id = required_string_property(path, source, node, "id")?;
            let name = first_string_argument(node)
                .or_else(|| string_property(node, "name"))
                .unwrap_or(&id)
                .trim()
                .to_owned();
            let channels = required_integer_property(path, source, node, "channels", &id)?;
            if channels < 1 {
                return Err(semantic_error(
                    path,
                    source,
                    direction.as_str(),
                    &format!("audio port `{id}` has non-positive channel count `{channels}`"),
                    "use channels >= 1",
                ));
            }
            Ok(AudioPortIr {
                id: id.clone(),
                name,
                direction,
                channels,
                port_type: string_property(node, "type").map(normalize_token),
                flags: named_flags(path, source, node, "flags", &id, AUDIO_PORT_FLAGS)?,
                in_place_pair: string_property(node, "in-place-pair").map(str::to_owned),
            })
        })
        .collect()
}

fn build_note_ports(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<(Vec<NotePortIr>, Vec<NoteNameIr>), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut ports = Vec::new();
    let mut names = Vec::new();
    for node in children.nodes() {
        match node.name().value() {
            "input" | "output" => {
                let direction = if node.name().value() == "input" {
                    Direction::Input
                } else {
                    Direction::Output
                };
                let id = required_string_property(path, source, node, "id")?;
                let name = first_string_argument(node)
                    .or_else(|| string_property(node, "name"))
                    .unwrap_or(&id)
                    .trim()
                    .to_owned();
                let dialects = optional_named_list(path, source, node, "dialects", &id)?;
                let preferred = string_property(node, "preferred").map(normalize_token);
                if let Some(value) = &preferred {
                    if !dialects.contains(value) {
                        return Err(semantic_error(
                            path,
                            source,
                            direction.as_str(),
                            &format!(
                                "note port `{id}` prefers dialect `{value}` but it is not in supported dialects"
                            ),
                            "include the preferred dialect in `dialects`",
                        ));
                    }
                }
                ports.push(NotePortIr {
                    id,
                    name,
                    direction,
                    dialects,
                    preferred,
                });
            }
            "note-name" => {
                let name = first_string_argument(node)
                    .ok_or_else(|| {
                        semantic_error(
                            path,
                            source,
                            "note-name",
                            "note-name requires a string name argument",
                            "use `note-name \"C4\" ...`",
                        )
                    })?
                    .to_owned();
                names.push(NoteNameIr {
                    name,
                    key: integer_property(node, "key"),
                    channel: integer_property(node, "channel"),
                    port: string_property(node, "port").map(str::to_owned),
                });
            }
            _ => {}
        }
    }
    Ok((ports, names))
}

fn build_state(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<StateFieldIr>, String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(Vec::new());
    };
    children
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "field")
        .map(|node| {
            let name = first_string_argument(node)
                .or_else(|| string_property(node, "name"))
                .ok_or_else(|| {
                    semantic_error(
                        path,
                        source,
                        "field",
                        "state field is missing a name",
                        "provide a string argument or `name=` property",
                    )
                })?
                .to_owned();
            let field_type = string_property(node, "type")
                .map(normalize_token)
                .ok_or_else(|| {
                    semantic_error(
                        path,
                        source,
                        "field",
                        &format!("state field `{name}` is missing `type`"),
                        "declare a symbolic state type",
                    )
                })?;
            Ok(StateFieldIr {
                name,
                field_type,
                default: property(node, "default").map(canonical_value),
                tag: string_property(node, "tag").map(str::to_owned),
            })
        })
        .collect()
}

fn build_gui(
    _path: &Path,
    _source: &str,
    root: Option<&KdlNode>,
) -> Result<(Vec<GuiApiIr>, Vec<ResourceIr>), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut apis = Vec::new();
    let mut resources = Vec::new();
    for node in children.nodes() {
        match node.name().value() {
            "api" => {
                let name = first_string_argument(node)
                    .or_else(|| string_property(node, "name"))
                    .unwrap_or("default")
                    .to_owned();
                apis.push(GuiApiIr {
                    name,
                    floating: bool_property(node, "floating").unwrap_or(false),
                    embedded: bool_property(node, "embedded").unwrap_or(true),
                });
            }
            "resource" => {
                let Some(path) = first_string_argument(node).or_else(|| string_property(node, "path")) else {
                    continue;
                };
                resources.push(ResourceIr {
                    path: normalize_path(path),
                    mime: string_property(node, "mime").map(normalize_token),
                });
            }
            _ => {}
        }
    }
    Ok((apis, resources))
}

fn build_presets(
    _path: &Path,
    _source: &str,
    root: Option<&KdlNode>,
) -> Result<(Vec<PresetLocationIr>, Vec<PresetFormatIr>), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut locations = Vec::new();
    let mut formats = Vec::new();
    for node in children.nodes() {
        match node.name().value() {
            "location" => {
                let name = first_string_argument(node).unwrap_or("default").to_owned();
                locations.push(PresetLocationIr {
                    name,
                    kind: string_property(node, "kind").map(normalize_token),
                    path: string_property(node, "path").map(normalize_path),
                });
            }
            "format" => {
                let name = first_string_argument(node).unwrap_or("default").to_owned();
                formats.push(PresetFormatIr {
                    name,
                    extension: string_property(node, "extension")
                        .map(|value| value.trim_start_matches('.').to_ascii_lowercase()),
                    mime: string_property(node, "mime").map(normalize_token),
                });
            }
            _ => {}
        }
    }
    Ok((locations, formats))
}

fn build_factories(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<FactoryIr>, String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(Vec::new());
    };
    children
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "factory")
        .map(|node| {
            let id = first_string_argument(node)
                .or_else(|| string_property(node, "id"))
                .ok_or_else(|| {
                    semantic_error(
                        path,
                        source,
                        "factory",
                        "factory is missing an ID",
                        "provide the factory ID as a string argument",
                    )
                })?
                .to_owned();
            Ok(FactoryIr {
                kind: string_property(node, "kind")
                    .map(normalize_token)
                    .unwrap_or_else(|| "plugin".to_owned()),
                id,
            })
        })
        .collect()
}

fn build_extensions(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<(Vec<ExtensionIr>, Vec<ExtensionIr>), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut stable = Vec::new();
    let mut draft = Vec::new();
    for node in children.nodes().iter().filter(|node| node.name().value() == "enable") {
        let id = first_string_argument(node)
            .or_else(|| string_property(node, "id"))
            .ok_or_else(|| {
                semantic_error(
                    path,
                    source,
                    "enable",
                    "extension enable node is missing an extension ID",
                    "use `enable \"clap.extension-id\"`",
                )
            })?
            .trim()
            .to_owned();
        let version = string_property(node, "version").map(str::to_owned);
        let is_draft = bool_property(node, "draft").unwrap_or(false);
        if is_draft {
            let Some(pin) = version.as_deref() else {
                return Err(draft_pin_error(path, source, &id));
            };
            let Some((_, abi_version)) = id.rsplit_once('/') else {
                return Err(draft_pin_error(path, source, &id));
            };
            if abi_version.is_empty() || abi_version != pin {
                return Err(draft_pin_error(path, source, &id));
            }
        }
        let extension = ExtensionIr {
            id,
            version,
            draft: is_draft,
        };
        if is_draft {
            draft.push(extension);
        } else {
            stable.push(extension);
        }
    }
    validate_unique_ids(path, source, stable.iter().map(|value| value.id.as_str()), "stable extension")?;
    validate_unique_ids(path, source, draft.iter().map(|value| value.id.as_str()), "draft extension")?;
    Ok((stable, draft))
}

fn draft_pin_error(path: &Path, source: &str, id: &str) -> String {
    semantic_error(
        path,
        source,
        "enable",
        &format!(
            "draft extension `{id}` must declare an exact ABI ID and matching `version` pin"
        ),
        "use an exact ID such as `clap.webview/3` with `version=\"3\" draft=#true`",
    )
}

fn validate_audio_references(path: &Path, source: &str, ports: &[AudioPortIr]) -> Result<(), String> {
    let by_id = ports
        .iter()
        .map(|port| (port.id.as_str(), port))
        .collect::<BTreeMap<_, _>>();
    for port in ports {
        let Some(target_id) = port.in_place_pair.as_deref() else {
            continue;
        };
        let Some(target) = by_id.get(target_id) else {
            return Err(semantic_error(
                path,
                source,
                port.direction.as_str(),
                &format!(
                    "audio port `{}` has in-place-pair reference to missing target `{target_id}`",
                    port.id
                ),
                "reference an existing opposite-direction audio port ID",
            ));
        };
        if target.direction == port.direction || target.channels != port.channels {
            return Err(semantic_error(
                path,
                source,
                port.direction.as_str(),
                &format!(
                    "audio port `{}` has incompatible in-place-pair target `{target_id}`",
                    port.id
                ),
                "in-place pairs must have opposite directions and matching channel counts",
            ));
        }
    }
    Ok(())
}

fn validate_note_references(
    path: &Path,
    source: &str,
    ports: &[NotePortIr],
    names: &[NoteNameIr],
) -> Result<(), String> {
    let ids = ports.iter().map(|port| port.id.as_str()).collect::<BTreeSet<_>>();
    for name in names {
        if let Some(target) = name.port.as_deref() {
            if !ids.contains(target) {
                return Err(semantic_error(
                    path,
                    source,
                    "note-name",
                    &format!(
                        "note-name `{}` references missing note port target `{target}`",
                        name.name
                    ),
                    "reference an existing note port ID",
                ));
            }
        }
    }
    Ok(())
}

fn validate_capability_dependencies(
    path: &Path,
    source: &str,
    stable: &[ExtensionIr],
    draft: &[ExtensionIr],
    note_ports: &[NotePortIr],
    gui_apis: &[GuiApiIr],
) -> Result<(), String> {
    let ids = stable
        .iter()
        .chain(draft)
        .map(|extension| extension.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids.contains("clap.note-expression") && note_ports.is_empty() {
        return Err(semantic_error(
            path,
            source,
            "enable",
            "extension `clap.note-expression` requires at least one note port",
            "declare a note input or output before enabling note expressions",
        ));
    }
    if ids.iter().any(|id| id.starts_with("clap.webview/")) && gui_apis.is_empty() {
        return Err(semantic_error(
            path,
            source,
            "enable",
            "draft Webview capability requires at least one GUI API declaration",
            "add a `gui { api ... }` declaration",
        ));
    }
    Ok(())
}

fn sort_and_validate_unique<T, F>(
    path: &Path,
    source: &str,
    values: &mut [T],
    key: F,
    kind: &str,
) -> Result<(), String>
where
    F: Fn(&T) -> &String,
{
    values.sort_by(|a, b| key(a).cmp(key(b)));
    validate_unique_ids(path, source, values.iter().map(|value| key(value).as_str()), kind)
}

fn validate_unique_ids<'a>(
    path: &Path,
    source: &str,
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(semantic_error(
                path,
                source,
                kind,
                &format!("duplicate {kind} ID `{id}`"),
                "persistent IDs must be unique",
            ));
        }
    }
    Ok(())
}

pub(crate) fn serialize_ir_kdl(ir: &CanonicalIr) -> String {
    let mut output = String::new();
    output.push_str(&format!("ir version={}\n", ir.version));
    output.push_str(&format!(
        "plugin id={} name={} vendor={} version={}\n",
        quoted(&ir.plugin.id),
        quoted(&ir.plugin.name),
        quoted(&ir.plugin.vendor),
        quoted(&ir.plugin.version)
    ));
    output.push_str(&format!("processor class={}{}\n", quoted(&ir.processor.class), list_prop("features", &ir.processor.features)));
    write_parameters(&mut output, &ir.parameters);
    write_audio_ports(&mut output, &ir.audio_ports);
    write_note_ports(&mut output, &ir.note_ports, &ir.note_names);
    write_state(&mut output, &ir.state_fields);
    write_gui(&mut output, &ir.gui_apis, &ir.gui_resources);
    write_presets(&mut output, &ir.preset_locations, &ir.preset_formats);
    write_factories(&mut output, &ir.factories);
    write_extensions(&mut output, &ir.stable_extensions, &ir.draft_extensions);
    output.push_str("imports {\n");
    for import in &ir.imports {
        output.push_str(&format!("    import {}\n", quoted(import)));
    }
    output.push_str("}\n");
    output.push_str(&capability_report_kdl(ir));
    output
}

fn write_parameters(output: &mut String, parameters: &[ParameterIr]) {
    output.push_str("parameters {\n");
    for parameter in parameters {
        output.push_str(&format!(
            "    param id={} name={} min={} max={} default={}{}{}{}\n",
            quoted(&parameter.id),
            quoted(&parameter.name),
            number_string(parameter.min),
            number_string(parameter.max),
            number_string(parameter.default),
            list_prop("flags", &parameter.flags),
            parameter
                .unit
                .as_ref()
                .map(|value| format!(" unit={}", quoted(value)))
                .unwrap_or_default(),
            parameter
                .steps
                .map(|value| format!(" steps={value}"))
                .unwrap_or_default(),
        ));
    }
    output.push_str("}\n");
}

fn write_audio_ports(output: &mut String, ports: &[AudioPortIr]) {
    output.push_str("audio-ports {\n");
    for port in ports {
        output.push_str(&format!(
            "    {} id={} name={} channels={}{}{}{}\n",
            port.direction.as_str(),
            quoted(&port.id),
            quoted(&port.name),
            port.channels,
            port.port_type
                .as_ref()
                .map(|value| format!(" type={}", quoted(value)))
                .unwrap_or_default(),
            list_prop("flags", &port.flags),
            port.in_place_pair
                .as_ref()
                .map(|value| format!(" in-place-pair={}", quoted(value)))
                .unwrap_or_default(),
        ));
    }
    output.push_str("}\n");
}

fn write_note_ports(output: &mut String, ports: &[NotePortIr], names: &[NoteNameIr]) {
    output.push_str("note-ports {\n");
    for port in ports {
        output.push_str(&format!(
            "    {} id={} name={}{}{}\n",
            port.direction.as_str(),
            quoted(&port.id),
            quoted(&port.name),
            list_prop("dialects", &port.dialects),
            port.preferred
                .as_ref()
                .map(|value| format!(" preferred={}", quoted(value)))
                .unwrap_or_default(),
        ));
    }
    for name in names {
        output.push_str(&format!(
            "    note-name {}{}{}{}\n",
            quoted(&name.name),
            name.key.map(|value| format!(" key={value}")).unwrap_or_default(),
            name.channel
                .map(|value| format!(" channel={value}"))
                .unwrap_or_default(),
            name.port
                .as_ref()
                .map(|value| format!(" port={}", quoted(value)))
                .unwrap_or_default(),
        ));
    }
    output.push_str("}\n");
}

fn write_state(output: &mut String, fields: &[StateFieldIr]) {
    output.push_str("state {\n");
    for field in fields {
        output.push_str(&format!(
            "    field {} type={}{}{}\n",
            quoted(&field.name),
            quoted(&field.field_type),
            field
                .default
                .as_ref()
                .map(|value| format!(" default={value}"))
                .unwrap_or_default(),
            field
                .tag
                .as_ref()
                .map(|value| format!(" tag={}", quoted(value)))
                .unwrap_or_default(),
        ));
    }
    output.push_str("}\n");
}

fn write_gui(output: &mut String, apis: &[GuiApiIr], resources: &[ResourceIr]) {
    output.push_str("gui {\n");
    for api in apis {
        output.push_str(&format!(
            "    api {} floating={} embedded={}\n",
            quoted(&api.name),
            bool_string(api.floating),
            bool_string(api.embedded)
        ));
    }
    for resource in resources {
        output.push_str(&format!(
            "    resource {}{}\n",
            quoted(&resource.path),
            resource
                .mime
                .as_ref()
                .map(|value| format!(" mime={}", quoted(value)))
                .unwrap_or_default()
        ));
    }
    output.push_str("}\n");
}

fn write_presets(output: &mut String, locations: &[PresetLocationIr], formats: &[PresetFormatIr]) {
    output.push_str("presets {\n");
    for location in locations {
        output.push_str(&format!(
            "    location {}{}{}\n",
            quoted(&location.name),
            location
                .kind
                .as_ref()
                .map(|value| format!(" kind={}", quoted(value)))
                .unwrap_or_default(),
            location
                .path
                .as_ref()
                .map(|value| format!(" path={}", quoted(value)))
                .unwrap_or_default()
        ));
    }
    for format in formats {
        output.push_str(&format!(
            "    format {}{}{}\n",
            quoted(&format.name),
            format
                .extension
                .as_ref()
                .map(|value| format!(" extension={}", quoted(value)))
                .unwrap_or_default(),
            format
                .mime
                .as_ref()
                .map(|value| format!(" mime={}", quoted(value)))
                .unwrap_or_default()
        ));
    }
    output.push_str("}\n");
}

fn write_factories(output: &mut String, factories: &[FactoryIr]) {
    output.push_str("factories {\n");
    for factory in factories {
        output.push_str(&format!(
            "    factory {} kind={}\n",
            quoted(&factory.id),
            quoted(&factory.kind)
        ));
    }
    output.push_str("}\n");
}

fn write_extensions(output: &mut String, stable: &[ExtensionIr], draft: &[ExtensionIr]) {
    output.push_str("extensions {\n");
    for extension in stable {
        output.push_str(&format!(
            "    stable {}{}\n",
            quoted(&extension.id),
            extension
                .version
                .as_ref()
                .map(|value| format!(" version={}", quoted(value)))
                .unwrap_or_default()
        ));
    }
    for extension in draft {
        output.push_str(&format!(
            "    draft {} version={}\n",
            quoted(&extension.id),
            quoted(extension.version.as_deref().unwrap_or_default())
        ));
    }
    output.push_str("}\n");
}

pub(crate) fn capability_report_kdl(ir: &CanonicalIr) -> String {
    let mut output = String::from("capabilities {\n");
    output.push_str(&format!("    parameters count={}\n", ir.parameters.len()));
    output.push_str(&format!("    audio-ports count={}\n", ir.audio_ports.len()));
    output.push_str(&format!("    note-ports count={}\n", ir.note_ports.len()));
    output.push_str(&format!("    state-fields count={}\n", ir.state_fields.len()));
    output.push_str(&format!("    gui-apis count={}\n", ir.gui_apis.len()));
    output.push_str(&format!("    factories count={}\n", ir.factories.len()));
    for extension in &ir.stable_extensions {
        output.push_str(&format!(
            "    extension {} stability=\"stable\"{}\n",
            quoted(&extension.id),
            extension
                .version
                .as_ref()
                .map(|value| format!(" version={}", quoted(value)))
                .unwrap_or_default()
        ));
    }
    for extension in &ir.draft_extensions {
        output.push_str(&format!(
            "    extension {} stability=\"draft\" version={}\n",
            quoted(&extension.id),
            quoted(extension.version.as_deref().unwrap_or_default())
        ));
    }
    output.push_str("}\n");
    output
}

fn required_string_property(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
) -> Result<String, String> {
    string_property(node, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            semantic_error(
                path,
                source,
                node.name().value(),
                &format!("missing string property `{key}`"),
                "declare the required semantic field as a string",
            )
        })
}

fn required_number_property(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<f64, String> {
    property(node, key)
        .and_then(number_value)
        .ok_or_else(|| {
            semantic_error(
                path,
                source,
                node.name().value(),
                &format!("`{subject}` requires numeric property `{key}`"),
                "use a finite KDL number",
            )
        })
}

fn required_integer_property(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<i128, String> {
    integer_property(node, key).ok_or_else(|| {
        semantic_error(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` requires integer property `{key}`"),
            "use a KDL integer",
        )
    })
}

fn optional_integer_property(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<Option<i128>, String> {
    let Some(value) = property(node, key) else {
        return Ok(None);
    };
    match value {
        KdlValue::Integer(value) => Ok(Some(*value)),
        _ => Err(semantic_error(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` property `{key}` must be an integer"),
            "use a KDL integer",
        )),
    }
}

fn named_flags(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
    allowed: &[&str],
) -> Result<Vec<String>, String> {
    let Some(value) = property(node, key) else {
        return Ok(Vec::new());
    };
    let KdlValue::String(value) = value else {
        return Err(semantic_error(
            path,
            source,
            node.name().value(),
            &format!(
                "`{subject}` property `{key}` must use named CLAP flags; raw numeric C bitmasks are not accepted"
            ),
            "use a comma-separated string of symbolic flag names",
        ));
    };
    let mut flags = split_named_list(value);
    for flag in &flags {
        if !allowed.contains(&flag.as_str()) {
            return Err(semantic_error(
                path,
                source,
                node.name().value(),
                &format!("`{subject}` has unknown named flag `{flag}`"),
                "use a supported symbolic CLAP flag",
            ));
        }
    }
    flags.sort();
    flags.dedup();
    Ok(flags)
}

fn optional_named_list(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = property(node, key) else {
        return Ok(Vec::new());
    };
    let KdlValue::String(value) = value else {
        return Err(semantic_error(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` property `{key}` must be a symbolic string list"),
            "use comma-separated symbolic names",
        ));
    };
    Ok(split_named_list(value))
}

fn split_named_list(value: &str) -> Vec<String> {
    let mut values = value
        .split(',')
        .map(normalize_token)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn child_string_values(node: &KdlNode, child_name: &str) -> Vec<String> {
    let mut values = node
        .children()
        .into_iter()
        .flat_map(|children| children.nodes())
        .filter(|child| child.name().value() == child_name)
        .filter_map(first_string_argument)
        .map(normalize_token)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn property<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlValue> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        (name.value() == key).then_some(entry.value())
    })
}

fn string_property<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> {
    match property(node, key)? {
        KdlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn integer_property(node: &KdlNode, key: &str) -> Option<i128> {
    match property(node, key)? {
        KdlValue::Integer(value) => Some(*value),
        _ => None,
    }
}

fn bool_property(node: &KdlNode, key: &str) -> Option<bool> {
    match property(node, key)? {
        KdlValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn number_value(value: &KdlValue) -> Option<f64> {
    match value {
        KdlValue::Float(value) if value.is_finite() => Some(*value),
        KdlValue::Integer(value) => value.to_string().parse().ok(),
        _ => None,
    }
}

fn first_string_argument(node: &KdlNode) -> Option<&str> {
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

fn canonical_value(value: &KdlValue) -> String {
    match value {
        KdlValue::String(value) => quoted(value),
        KdlValue::Integer(value) => value.to_string(),
        KdlValue::Float(value) => number_string(*value),
        KdlValue::Bool(value) => bool_string(*value).to_owned(),
        KdlValue::Null => "#null".to_owned(),
    }
}

fn normalize_token(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_path(value: &str) -> String {
    let mut parts = Vec::new();
    for part in value.replace('\\', "/").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    parts.join("/")
}

fn quoted(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

fn list_prop(name: &str, values: &[String]) -> String {
    if values.is_empty() {
        String::new()
    } else {
        format!(" {name}={}", quoted(&values.join(",")))
    }
}

fn number_string(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

fn bool_string(value: bool) -> &'static str {
    if value { "#true" } else { "#false" }
}

fn semantic_error(path: &Path, source: &str, node: &str, message: &str, hint: &str) -> String {
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
    format!("{}:{line}: {message}\nhint: {hint}", path.display())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::metadata::parse_metadata;

    use super::{build_ir, capability_report_kdl, serialize_ir_kdl};

    fn build(source: &str) -> Result<super::CanonicalIr, String> {
        let path = Path::new("plugin.kdl");
        let parsed = parse_metadata(path, source)?;
        build_ir(path, source, &parsed)
    }

    const PREFIX: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"SynthProcessor\"\n";

    #[test]
    fn semantic_equivalence_produces_identical_canonical_ir() {
        let a = format!(
            "{PREFIX}parameters {{\n    param \"cutoff\" id=\"cutoff\" min=20.0 max=20000.0 default=1000.0 flags=\"modulatable,automatable\" unit=\"Hz\"\n    param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=\"automatable\"\n}}\naudio-ports {{ input \"main\" id=\"in\" channels=2; output \"main\" id=\"out\" channels=2 }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );
        let b = format!(
            "{PREFIX}parameters {{\n    param \"gain\" default=0.5 max=1.0 min=0.0 id=\"gain\" flags=\"automatable\"\n    param \"cutoff\" default=1000.0 max=20000.0 min=20.0 id=\"cutoff\" unit=\"hz\" flags=\"automatable, modulatable\"\n}}\naudio-ports {{ output \"main\" channels=2 id=\"out\"; input \"main\" channels=2 id=\"in\" }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );

        let a = build(&a).expect("first manifest should build");
        let b = build(&b).expect("equivalent manifest should build");
        assert_eq!(serialize_ir_kdl(&a), serialize_ir_kdl(&b));
    }

    #[test]
    fn rejects_raw_numeric_clap_flag_bitmasks() {
        let source = format!(
            "{PREFIX}parameters {{ param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=3 }}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("raw C bitmask must be rejected");
        assert!(error.contains("flags"), "{error}");
        assert!(error.contains("named"), "{error}");
        assert!(error.contains("gain"), "{error}");
    }

    #[test]
    fn cross_reference_error_identifies_source_and_missing_target() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{\n    input \"main\" id=\"in\" channels=2\n    output \"main\" id=\"out\" channels=2 in-place-pair=\"missing-input\"\n}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("missing target must fail");
        assert!(error.contains("out"), "{error}");
        assert!(error.contains("missing-input"), "{error}");
        assert!(error.contains("in-place-pair"), "{error}");
    }

    #[test]
    fn draft_extensions_require_exact_abi_id_and_version_pin() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.webview\" draft=#true }}\n"
        );
        let error = build(&source).expect_err("unpinned draft must fail");
        assert!(error.contains("draft"), "{error}");
        assert!(error.contains("exact ABI"), "{error}");
        assert!(error.contains("version"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{ api \"web\" }}\npresets {{}}\nextensions {{ enable \"clap.webview/3\" version=\"3\" draft=#true }}\n"
        );
        build(&valid).expect("exact draft ABI should be accepted");
    }

    #[test]
    fn capability_dependencies_are_validated_and_reported() {
        let invalid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let error = build(&invalid).expect_err("note expression requires note ports");
        assert!(error.contains("clap.note-expression"), "{error}");
        assert!(error.contains("note port"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{ input \"notes\" id=\"notes-in\" dialects=\"clap\" preferred=\"clap\" }}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let ir = build(&valid).expect("dependency should be satisfied");
        let report = capability_report_kdl(&ir);
        assert!(report.contains("clap.note-expression"));
        assert!(report.contains("stable"));
    }

    #[test]
    fn ir_serialization_has_a_versioned_compatibility_marker() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let ir = build(&source).expect("manifest should build");
        let serialized = serialize_ir_kdl(&ir);
        assert!(serialized.starts_with("ir version=1\n"), "{serialized}");
        assert_eq!(serialized, serialize_ir_kdl(&ir));
    }
}
