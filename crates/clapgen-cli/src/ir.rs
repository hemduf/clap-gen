use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Arguments, Write as _};
use std::path::Path;

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::metadata::ParsedMetadata;

const IR_VERSION: u32 = 1;
const PARAM_FLAGS: &[&str] =
    &["automatable", "bypass", "enum", "hidden", "modulatable", "periodic", "readonly", "stepped"];
const AUDIO_FLAGS: &[&str] = &["main", "requires-common-sample-size"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
    pub(crate) plugin: PluginIr,
    pub(crate) processor: ProcessorIr,
    pub(crate) parameters: Vec<ParameterIr>,
    pub(crate) audio_ports: Vec<AudioPortIr>,
    pub(crate) note_ports: Vec<NotePortIr>,
    pub(crate) state_fields: Vec<StateFieldIr>,
    pub(crate) gui: GuiIr,
    pub(crate) presets: PresetsIr,
    pub(crate) factories: Vec<FactoryIr>,
    pub(crate) stable_extensions: Vec<ExtensionIr>,
    pub(crate) draft_extensions: Vec<ExtensionIr>,
    pub(crate) imports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIr {
    id: String,
    name: String,
    vendor: String,
    version: String,
    features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessorIr {
    class: String,
    features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParameterIr {
    id: String,
    name: String,
    min: f64,
    max: f64,
    default: f64,
    flags: Vec<String>,
    unit: Option<String>,
    steps: Option<i128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    Input,
    Output,
}

impl Direction {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AudioPortIr {
    id: String,
    name: String,
    direction: Direction,
    channels: i128,
    port_type: Option<String>,
    flags: Vec<String>,
    in_place_pair: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotePortIr {
    id: String,
    name: String,
    direction: Direction,
    dialects: Vec<String>,
    preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateFieldIr {
    name: String,
    field_type: String,
    default: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GuiIr {
    apis: Vec<GuiApiIr>,
    resources: Vec<ResourceIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuiApiIr {
    name: String,
    floating: bool,
    embedded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceIr {
    path: String,
    mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PresetsIr {
    locations: Vec<PresetLocationIr>,
    formats: Vec<PresetFormatIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresetLocationIr {
    name: String,
    kind: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresetFormatIr {
    name: String,
    extension: Option<String>,
    mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FactoryIr {
    id: String,
    kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionIr {
    id: String,
    version: Option<String>,
}

pub(crate) fn build_ir(
    path: &Path,
    source: &str,
    metadata: &ParsedMetadata,
) -> Result<CanonicalIr, String> {
    let document = &metadata.document;
    let plugin = build_plugin(path, source, document)?;
    let processor = build_processor(path, source, document)?;
    let mut parameters = build_parameters(path, source, document.get("parameters"))?;
    let mut audio_ports = build_audio_ports(path, source, document.get("audio-ports"))?;
    let mut note_ports = build_note_ports(path, source, document.get("note-ports"))?;
    let mut state_fields = build_state(path, source, document.get("state"))?;
    let mut gui = build_gui(document.get("gui"));
    let mut presets = build_presets(document.get("presets"));
    let mut factories = build_factories(path, source, document.get("factories"))?;
    let (mut stable_extensions, mut draft_extensions) =
        build_extensions(path, source, document.get("extensions"))?;

    canonicalize(
        path,
        source,
        &mut parameters,
        &mut audio_ports,
        &mut note_ports,
        &mut state_fields,
        &mut gui,
        &mut presets,
        &mut factories,
        &mut stable_extensions,
        &mut draft_extensions,
    )?;
    validate_audio_references(path, source, &audio_ports)?;
    validate_dependencies(
        path,
        source,
        &stable_extensions,
        &draft_extensions,
        &note_ports,
        &gui,
    )?;

    let imports = canonical_imports(metadata);
    Ok(CanonicalIr {
        version: IR_VERSION,
        plugin,
        processor,
        parameters,
        audio_ports,
        note_ports,
        state_fields,
        gui,
        presets,
        factories,
        stable_extensions,
        draft_extensions,
        imports,
    })
}

fn build_plugin(path: &Path, source: &str, document: &KdlDocument) -> Result<PluginIr, String> {
    let node = document.get("plugin").ok_or_else(|| {
        diagnostic(path, source, "plugin", "missing plugin descriptor", "add a `plugin` node")
    })?;
    Ok(PluginIr {
        id: required_string(path, source, node, "id")?,
        name: required_string(path, source, node, "name")?,
        vendor: required_string(path, source, node, "vendor")?,
        version: required_string(path, source, node, "version")?,
        features: child_values(node, "feature"),
    })
}

fn build_processor(
    path: &Path,
    source: &str,
    document: &KdlDocument,
) -> Result<ProcessorIr, String> {
    let node = document.get("processor").ok_or_else(|| {
        diagnostic(
            path,
            source,
            "processor",
            "missing processor declaration",
            "add `processor class=\"ProcessorClass\"`",
        )
    })?;
    Ok(ProcessorIr {
        class: required_string(path, source, node, "class")?,
        features: named_list(path, source, node, "features", "processor")?,
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
        .map(|node| build_parameter(path, source, node))
        .collect()
}

fn build_parameter(path: &Path, source: &str, node: &KdlNode) -> Result<ParameterIr, String> {
    let id = required_string(path, source, node, "id")?;
    let name = first_string(node)
        .or_else(|| string_prop(node, "name"))
        .unwrap_or(&id)
        .trim()
        .to_owned();
    let min = required_number(path, source, node, "min", &id)?;
    let max = required_number(path, source, node, "max", &id)?;
    let default = required_number(path, source, node, "default", &id)?;
    if min > max || default < min || default > max {
        return Err(diagnostic(
            path,
            source,
            "param",
            &format!("parameter `{id}` has invalid range/default"),
            "require min <= default <= max",
        ));
    }
    let steps = optional_integer(path, source, node, "steps", &id)?;
    if steps.is_some_and(|value| value < 1) {
        return Err(diagnostic(
            path,
            source,
            "param",
            &format!("parameter `{id}` has invalid `steps`"),
            "use a positive integer step count",
        ));
    }
    Ok(ParameterIr {
        id: id.clone(),
        name,
        min,
        max,
        default,
        flags: named_flags(path, source, node, "flags", &id, PARAM_FLAGS)?,
        unit: string_prop(node, "unit").map(token),
        steps,
    })
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
        .map(|(node, direction)| build_audio_port(path, source, node, direction))
        .collect()
}

fn build_audio_port(
    path: &Path,
    source: &str,
    node: &KdlNode,
    direction: Direction,
) -> Result<AudioPortIr, String> {
    let id = required_string(path, source, node, "id")?;
    let channels = required_integer(path, source, node, "channels", &id)?;
    if channels < 1 {
        return Err(diagnostic(
            path,
            source,
            direction.as_str(),
            &format!("audio port `{id}` has non-positive channel count `{channels}`"),
            "use channels >= 1",
        ));
    }
    Ok(AudioPortIr {
        name: first_string(node)
            .or_else(|| string_prop(node, "name"))
            .unwrap_or(&id)
            .trim()
            .to_owned(),
        id: id.clone(),
        direction,
        channels,
        port_type: string_prop(node, "type").map(token),
        flags: named_flags(path, source, node, "flags", &id, AUDIO_FLAGS)?,
        in_place_pair: string_prop(node, "in-place-pair").map(str::to_owned),
    })
}

fn build_note_ports(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<NotePortIr>, String> {
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
        .map(|(node, direction)| build_note_port(path, source, node, direction))
        .collect()
}

fn build_note_port(
    path: &Path,
    source: &str,
    node: &KdlNode,
    direction: Direction,
) -> Result<NotePortIr, String> {
    let id = required_string(path, source, node, "id")?;
    let dialects = named_list(path, source, node, "dialects", &id)?;
    let preferred = string_prop(node, "preferred").map(token);
    if preferred.as_ref().is_some_and(|value| !dialects.contains(value)) {
        return Err(diagnostic(
            path,
            source,
            direction.as_str(),
            &format!("note port `{id}` preferred dialect is not supported by that port"),
            "include the preferred dialect in `dialects`",
        ));
    }
    Ok(NotePortIr {
        name: first_string(node)
            .or_else(|| string_prop(node, "name"))
            .unwrap_or(&id)
            .trim()
            .to_owned(),
        id,
        direction,
        dialects,
        preferred,
    })
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
            let name = first_string(node)
                .or_else(|| string_prop(node, "name"))
                .ok_or_else(|| {
                    diagnostic(
                        path,
                        source,
                        "field",
                        "state field is missing a name",
                        "provide a string argument or `name=` property",
                    )
                })?
                .to_owned();
            let field_type = string_prop(node, "type").map(token).ok_or_else(|| {
                diagnostic(
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
                default: prop(node, "default").map(value_text),
                tag: string_prop(node, "tag").map(str::to_owned),
            })
        })
        .collect()
}

fn build_gui(root: Option<&KdlNode>) -> GuiIr {
    let Some(children) = root.and_then(KdlNode::children) else {
        return GuiIr::default();
    };
    let mut gui = GuiIr::default();
    for node in children.nodes() {
        match node.name().value() {
            "api" => gui.apis.push(GuiApiIr {
                name: first_string(node)
                    .or_else(|| string_prop(node, "name"))
                    .unwrap_or("default")
                    .to_owned(),
                floating: bool_prop(node, "floating").unwrap_or(false),
                embedded: bool_prop(node, "embedded").unwrap_or(true),
            }),
            "resource" => {
                if let Some(path) = first_string(node).or_else(|| string_prop(node, "path")) {
                    gui.resources.push(ResourceIr {
                        path: normalize_path(path),
                        mime: string_prop(node, "mime").map(token),
                    });
                }
            }
            _ => {}
        }
    }
    gui
}

fn build_presets(root: Option<&KdlNode>) -> PresetsIr {
    let Some(children) = root.and_then(KdlNode::children) else {
        return PresetsIr::default();
    };
    let mut presets = PresetsIr::default();
    for node in children.nodes() {
        match node.name().value() {
            "location" => presets.locations.push(PresetLocationIr {
                name: first_string(node).unwrap_or("default").to_owned(),
                kind: string_prop(node, "kind").map(token),
                path: string_prop(node, "path").map(normalize_path),
            }),
            "format" => presets.formats.push(PresetFormatIr {
                name: first_string(node).unwrap_or("default").to_owned(),
                extension: string_prop(node, "extension")
                    .map(|value| value.trim_start_matches('.').to_ascii_lowercase()),
                mime: string_prop(node, "mime").map(token),
            }),
            _ => {}
        }
    }
    presets
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
            let id = first_string(node)
                .or_else(|| string_prop(node, "id"))
                .ok_or_else(|| {
                    diagnostic(
                        path,
                        source,
                        "factory",
                        "factory is missing an ID",
                        "provide the factory ID as a string argument",
                    )
                })?
                .to_owned();
            Ok(FactoryIr {
                id,
                kind: string_prop(node, "kind")
                    .map(token)
                    .unwrap_or_else(|| "plugin".to_owned()),
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
        let id = first_string(node)
            .or_else(|| string_prop(node, "id"))
            .ok_or_else(|| {
                diagnostic(
                    path,
                    source,
                    "enable",
                    "extension enable node is missing an extension ID",
                    "use `enable \"clap.extension-id\"`",
                )
            })?
            .trim()
            .to_owned();
        let version = string_prop(node, "version").map(str::to_owned);
        let is_draft = bool_prop(node, "draft").unwrap_or(false);
        if is_draft && !has_exact_draft_pin(&id, version.as_deref()) {
            return Err(diagnostic(
                path,
                source,
                "enable",
                &format!("draft extension `{id}` must declare an exact ABI ID and matching `version` pin"),
                "use an exact ID such as `clap.webview/3` with `version=\"3\" draft=#true`",
            ));
        }
        let extension = ExtensionIr { id, version };
        if is_draft {
            draft.push(extension);
        } else {
            stable.push(extension);
        }
    }
    Ok((stable, draft))
}

fn has_exact_draft_pin(id: &str, version: Option<&str>) -> bool {
    let Some(version) = version else {
        return false;
    };
    id.rsplit_once('/')
        .is_some_and(|(_, abi)| !abi.is_empty() && abi == version)
}

#[allow(clippy::too_many_arguments)]
fn canonicalize(
    path: &Path,
    source: &str,
    parameters: &mut Vec<ParameterIr>,
    audio_ports: &mut Vec<AudioPortIr>,
    note_ports: &mut Vec<NotePortIr>,
    state_fields: &mut Vec<StateFieldIr>,
    gui: &mut GuiIr,
    presets: &mut PresetsIr,
    factories: &mut Vec<FactoryIr>,
    stable_extensions: &mut Vec<ExtensionIr>,
    draft_extensions: &mut Vec<ExtensionIr>,
) -> Result<(), String> {
    parameters.sort_by(|a, b| a.id.cmp(&b.id));
    audio_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    note_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    state_fields.sort_by(|a, b| a.name.cmp(&b.name));
    gui.apis.sort_by(|a, b| a.name.cmp(&b.name));
    gui.resources.sort_by(|a, b| a.path.cmp(&b.path));
    presets.locations.sort_by(|a, b| a.name.cmp(&b.name));
    presets.formats.sort_by(|a, b| a.name.cmp(&b.name));
    factories.sort_by(|a, b| a.id.cmp(&b.id));
    stable_extensions.sort_by(|a, b| a.id.cmp(&b.id));
    draft_extensions.sort_by(|a, b| a.id.cmp(&b.id));

    unique(path, source, parameters.iter().map(|value| value.id.as_str()), "parameter")?;
    unique(path, source, audio_ports.iter().map(|value| value.id.as_str()), "audio port")?;
    unique(path, source, note_ports.iter().map(|value| value.id.as_str()), "note port")?;
    unique(path, source, state_fields.iter().map(|value| value.name.as_str()), "state field")?;
    unique(path, source, factories.iter().map(|value| value.id.as_str()), "factory")?;
    unique(
        path,
        source,
        stable_extensions.iter().map(|value| value.id.as_str()),
        "stable extension",
    )?;
    unique(
        path,
        source,
        draft_extensions.iter().map(|value| value.id.as_str()),
        "draft extension",
    )
}

fn unique<'a>(
    path: &Path,
    source: &str,
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(diagnostic(
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

fn validate_audio_references(
    path: &Path,
    source: &str,
    ports: &[AudioPortIr],
) -> Result<(), String> {
    let by_id = ports.iter().map(|port| (port.id.as_str(), port)).collect::<BTreeMap<_, _>>();
    for port in ports {
        let Some(target_id) = port.in_place_pair.as_deref() else {
            continue;
        };
        let Some(target) = by_id.get(target_id) else {
            return Err(diagnostic(
                path,
                source,
                port.direction.as_str(),
                &format!("audio port `{}` has in-place-pair reference to missing target `{target_id}`", port.id),
                "reference an existing opposite-direction audio port ID",
            ));
        };
        if target.direction == port.direction || target.channels != port.channels {
            return Err(diagnostic(
                path,
                source,
                port.direction.as_str(),
                &format!("audio port `{}` has incompatible in-place-pair target `{target_id}`", port.id),
                "in-place pairs require opposite directions and matching channel counts",
            ));
        }
    }
    Ok(())
}

fn validate_dependencies(
    path: &Path,
    source: &str,
    stable: &[ExtensionIr],
    draft: &[ExtensionIr],
    note_ports: &[NotePortIr],
    gui: &GuiIr,
) -> Result<(), String> {
    let ids = stable.iter().chain(draft).map(|extension| extension.id.as_str()).collect::<BTreeSet<_>>();
    if ids.contains("clap.note-expression") && note_ports.is_empty() {
        return Err(diagnostic(
            path,
            source,
            "enable",
            "extension `clap.note-expression` requires at least one note port",
            "declare a note input or output before enabling note expressions",
        ));
    }
    if ids.iter().any(|id| id.starts_with("clap.webview/")) && gui.apis.is_empty() {
        return Err(diagnostic(
            path,
            source,
            "enable",
            "draft Webview capability requires at least one GUI API declaration",
            "add a `gui { api ... }` declaration",
        ));
    }
    Ok(())
}

fn canonical_imports(metadata: &ParsedMetadata) -> Vec<String> {
    let mut imports = metadata
        .imports
        .iter()
        .map(|value| normalize_path(&value.to_string_lossy()))
        .collect::<Vec<_>>();
    imports.sort();
    imports.dedup();
    imports
}

pub(crate) fn serialize_ir_kdl(ir: &CanonicalIr) -> String {
    let mut out = String::new();
    line(&mut out, format_args!("ir version={}", ir.version));
    line(
        &mut out,
        format_args!(
            "plugin id={} name={} vendor={} version={}{}",
            quote(&ir.plugin.id),
            quote(&ir.plugin.name),
            quote(&ir.plugin.vendor),
            quote(&ir.plugin.version),
            list_prop("features", &ir.plugin.features)
        ),
    );
    line(
        &mut out,
        format_args!(
            "processor class={}{}",
            quote(&ir.processor.class),
            list_prop("features", &ir.processor.features)
        ),
    );
    write_parameters(&mut out, &ir.parameters);
    write_audio_ports(&mut out, &ir.audio_ports);
    write_note_ports(&mut out, &ir.note_ports);
    write_state(&mut out, &ir.state_fields);
    write_gui(&mut out, &ir.gui);
    write_presets(&mut out, &ir.presets);
    write_factories(&mut out, &ir.factories);
    write_extensions(&mut out, &ir.stable_extensions, &ir.draft_extensions);
    write_imports(&mut out, &ir.imports);
    out.push_str(&capability_report_kdl(ir));
    out
}

fn write_parameters(out: &mut String, parameters: &[ParameterIr]) {
    out.push_str("parameters {\n");
    for value in parameters {
        line(
            out,
            format_args!(
                "    param id={} name={} min={} max={} default={}{}{}{}",
                quote(&value.id),
                quote(&value.name),
                value.min,
                value.max,
                value.default,
                list_prop("flags", &value.flags),
                option_prop("unit", value.unit.as_deref()),
                value.steps.map(|steps| format!(" steps={steps}")).unwrap_or_default()
            ),
        );
    }
    out.push_str("}\n");
}

fn write_audio_ports(out: &mut String, ports: &[AudioPortIr]) {
    out.push_str("audio-ports {\n");
    for value in ports {
        line(
            out,
            format_args!(
                "    {} id={} name={} channels={}{}{}{}",
                value.direction.as_str(),
                quote(&value.id),
                quote(&value.name),
                value.channels,
                option_prop("type", value.port_type.as_deref()),
                list_prop("flags", &value.flags),
                option_prop("in-place-pair", value.in_place_pair.as_deref())
            ),
        );
    }
    out.push_str("}\n");
}

fn write_note_ports(out: &mut String, ports: &[NotePortIr]) {
    out.push_str("note-ports {\n");
    for value in ports {
        line(
            out,
            format_args!(
                "    {} id={} name={}{}{}",
                value.direction.as_str(),
                quote(&value.id),
                quote(&value.name),
                list_prop("dialects", &value.dialects),
                option_prop("preferred", value.preferred.as_deref())
            ),
        );
    }
    out.push_str("}\n");
}

fn write_state(out: &mut String, fields: &[StateFieldIr]) {
    out.push_str("state {\n");
    for value in fields {
        line(
            out,
            format_args!(
                "    field {} type={}{}{}",
                quote(&value.name),
                quote(&value.field_type),
                value.default.as_ref().map(|default| format!(" default={default}")).unwrap_or_default(),
                option_prop("tag", value.tag.as_deref())
            ),
        );
    }
    out.push_str("}\n");
}

fn write_gui(out: &mut String, gui: &GuiIr) {
    out.push_str("gui {\n");
    for value in &gui.apis {
        line(
            out,
            format_args!(
                "    api {} floating={} embedded={}",
                quote(&value.name),
                bool_text(value.floating),
                bool_text(value.embedded)
            ),
        );
    }
    for value in &gui.resources {
        line(
            out,
            format_args!("    resource {}{}", quote(&value.path), option_prop("mime", value.mime.as_deref())),
        );
    }
    out.push_str("}\n");
}

fn write_presets(out: &mut String, presets: &PresetsIr) {
    out.push_str("presets {\n");
    for value in &presets.locations {
        line(
            out,
            format_args!(
                "    location {}{}{}",
                quote(&value.name),
                option_prop("kind", value.kind.as_deref()),
                option_prop("path", value.path.as_deref())
            ),
        );
    }
    for value in &presets.formats {
        line(
            out,
            format_args!(
                "    format {}{}{}",
                quote(&value.name),
                option_prop("extension", value.extension.as_deref()),
                option_prop("mime", value.mime.as_deref())
            ),
        );
    }
    out.push_str("}\n");
}

fn write_factories(out: &mut String, factories: &[FactoryIr]) {
    out.push_str("factories {\n");
    for value in factories {
        line(out, format_args!("    factory {} kind={}", quote(&value.id), quote(&value.kind)));
    }
    out.push_str("}\n");
}

fn write_extensions(out: &mut String, stable: &[ExtensionIr], draft: &[ExtensionIr]) {
    out.push_str("extensions {\n");
    for value in stable {
        line(
            out,
            format_args!("    stable {}{}", quote(&value.id), option_prop("version", value.version.as_deref())),
        );
    }
    for value in draft {
        line(
            out,
            format_args!(
                "    draft {} version={}",
                quote(&value.id),
                quote(value.version.as_deref().unwrap_or_default())
            ),
        );
    }
    out.push_str("}\n");
}

fn write_imports(out: &mut String, imports: &[String]) {
    out.push_str("imports {\n");
    for value in imports {
        line(out, format_args!("    import {}", quote(value)));
    }
    out.push_str("}\n");
}

pub(crate) fn capability_report_kdl(ir: &CanonicalIr) -> String {
    let mut out = String::from("capabilities {\n");
    line(&mut out, format_args!("    parameters count={}", ir.parameters.len()));
    line(&mut out, format_args!("    audio-ports count={}", ir.audio_ports.len()));
    line(&mut out, format_args!("    note-ports count={}", ir.note_ports.len()));
    line(&mut out, format_args!("    state-fields count={}", ir.state_fields.len()));
    line(&mut out, format_args!("    gui-apis count={}", ir.gui.apis.len()));
    line(&mut out, format_args!("    factories count={}", ir.factories.len()));
    for value in &ir.stable_extensions {
        line(
            &mut out,
            format_args!(
                "    extension {} stability=\"stable\"{}",
                quote(&value.id),
                option_prop("version", value.version.as_deref())
            ),
        );
    }
    for value in &ir.draft_extensions {
        line(
            &mut out,
            format_args!(
                "    extension {} stability=\"draft\" version={}",
                quote(&value.id),
                quote(value.version.as_deref().unwrap_or_default())
            ),
        );
    }
    out.push_str("}\n");
    out
}

fn line(out: &mut String, arguments: Arguments<'_>) {
    out.write_fmt(arguments).expect("writing to a String cannot fail");
    out.push('\n');
}

fn required_string(path: &Path, source: &str, node: &KdlNode, key: &str) -> Result<String, String> {
    string_prop(node, key)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            diagnostic(
                path,
                source,
                node.name().value(),
                &format!("missing string property `{key}`"),
                "declare the required semantic field as a string",
            )
        })
}

fn required_number(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<f64, String> {
    prop(node, key).and_then(number).ok_or_else(|| {
        diagnostic(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` requires numeric property `{key}`"),
            "use a finite KDL number",
        )
    })
}

fn required_integer(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<i128, String> {
    integer_prop(node, key).ok_or_else(|| {
        diagnostic(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` requires integer property `{key}`"),
            "use a KDL integer",
        )
    })
}

fn optional_integer(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<Option<i128>, String> {
    let Some(value) = prop(node, key) else {
        return Ok(None);
    };
    match value {
        KdlValue::Integer(value) => Ok(Some(*value)),
        _ => Err(diagnostic(
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
    let values = named_list(path, source, node, key, subject)?;
    for value in &values {
        if !allowed.contains(&value.as_str()) {
            return Err(diagnostic(
                path,
                source,
                node.name().value(),
                &format!("`{subject}` has unknown named flag `{value}`"),
                "use a supported symbolic CLAP flag",
            ));
        }
    }
    Ok(values)
}

fn named_list(
    path: &Path,
    source: &str,
    node: &KdlNode,
    key: &str,
    subject: &str,
) -> Result<Vec<String>, String> {
    let Some(value) = prop(node, key) else {
        return Ok(Vec::new());
    };
    let KdlValue::String(value) = value else {
        return Err(diagnostic(
            path,
            source,
            node.name().value(),
            &format!("`{subject}` property `{key}` must use named symbolic values; raw numeric C bitmasks are not accepted"),
            "use a comma-separated string of symbolic names",
        ));
    };
    let mut values = value.split(',').map(token).filter(|value| !value.is_empty()).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    Ok(values)
}

fn child_values(node: &KdlNode, name: &str) -> Vec<String> {
    let mut values = node
        .children()
        .into_iter()
        .flat_map(|children| children.nodes())
        .filter(|child| child.name().value() == name)
        .filter_map(first_string)
        .map(token)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
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
        KdlValue::Float(value) => value.to_string(),
        KdlValue::Bool(value) => bool_text(*value).to_owned(),
        KdlValue::Null => "#null".to_owned(),
    }
}

fn token(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let absolute = normalized.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
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

fn option_prop(name: &str, value: Option<&str>) -> String {
    value.map(|value| format!(" {name}={}", quote(value))).unwrap_or_default()
}

fn list_prop(name: &str, values: &[String]) -> String {
    if values.is_empty() { String::new() } else { format!(" {name}={}", quote(&values.join(","))) }
}

const fn bool_text(value: bool) -> &'static str {
    if value { "#true" } else { "#false" }
}

fn diagnostic(path: &Path, source: &str, node: &str, message: &str, hint: &str) -> String {
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

    const PREFIX: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"SynthProcessor\"\n";

    fn build(source: &str) -> Result<super::CanonicalIr, String> {
        let path = Path::new("plugin.kdl");
        let parsed = parse_metadata(path, source)?;
        build_ir(path, source, &parsed)
    }

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

    #[test]
    fn serialization_preserves_plugin_features() {
        let source = format!(
            "{PREFIX}plugin-extra\nparameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        )
        .replace(
            "plugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"",
            "plugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\" { feature \"instrument\"; feature \"synthesizer\" }",
        )
        .replace("plugin-extra\n", "");
        let ir = build(&source).expect("feature manifest should build");
        let serialized = serialize_ir_kdl(&ir);
        assert!(serialized.contains("features=\"instrument,synthesizer\""), "{serialized}");
    }

    #[test]
    fn leading_parent_imports_are_preserved_during_normalization() {
        let source = format!(
            "clapgen schema=\"1.0.0\"\nimport \"../shared/common.kdl\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"SynthProcessor\"\nparameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nextensions {{}}\n"
        );
        let ir = build(&source).expect("parent-relative import should build");
        assert!(serialize_ir_kdl(&ir).contains("import \"../shared/common.kdl\""));
    }
}
