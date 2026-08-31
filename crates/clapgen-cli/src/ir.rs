use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
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
    plugin: PluginIr,
    processor: ProcessorIr,
    parameters: Vec<ParameterIr>,
    audio_ports: Vec<AudioPortIr>,
    note_ports: Vec<NotePortIr>,
    note_names: Vec<NoteNameIr>,
    state_fields: Vec<StateFieldIr>,
    gui: GuiIr,
    presets: PresetsIr,
    factories: Vec<FactoryIr>,
    pub(crate) stable_extensions: Vec<ExtensionIr>,
    pub(crate) draft_extensions: Vec<ExtensionIr>,
    imports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PluginIr {
    id: String,
    name: String,
    vendor: String,
    version: String,
    url: Option<String>,
    manual_url: Option<String>,
    support_url: Option<String>,
    description: Option<String>,
    features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessorIr {
    class: String,
    features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ParameterIr {
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
struct AudioPortIr {
    id: String,
    name: String,
    direction: Direction,
    channels: i128,
    port_type: Option<String>,
    flags: Vec<String>,
    in_place_pair: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NotePortIr {
    id: String,
    name: String,
    direction: Direction,
    dialects: Vec<String>,
    preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoteNameIr {
    name: String,
    key: Option<i128>,
    channel: Option<i128>,
    port: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StateFieldIr {
    name: String,
    field_type: String,
    default: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct GuiIr {
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
struct PresetsIr {
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
struct FactoryIr {
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
    let mut ir = CanonicalIr {
        version: IR_VERSION,
        plugin: build_plugin(path, source, document)?,
        processor: build_processor(path, source, document)?,
        parameters: build_parameters(path, source, document.get("parameters"))?,
        audio_ports: build_audio_ports(path, source, document.get("audio-ports"))?,
        note_ports: build_note_ports(path, source, document.get("note-ports"))?,
        note_names: build_note_names(path, source, document.get("note-ports"))?,
        state_fields: build_state(path, source, document.get("state"))?,
        gui: build_gui(path, source, document.get("gui"))?,
        presets: build_presets(document.get("presets")),
        factories: build_factories(path, source, document.get("factories"))?,
        stable_extensions: Vec::new(),
        draft_extensions: Vec::new(),
        imports: canonical_imports(metadata),
    };
    (ir.stable_extensions, ir.draft_extensions) =
        build_extensions(path, source, document.get("extensions"))?;

    canonicalize(&mut ir);
    validate_unique_ids(path, source, &ir)?;
    validate_audio_references(path, source, &ir.audio_ports)?;
    validate_note_name_references(path, source, &ir.note_ports, &ir.note_names)?;
    validate_dependencies(path, source, &ir)?;
    Ok(ir)
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
        url: string_prop(node, "url").map(str::to_owned),
        manual_url: string_prop(node, "manual-url").map(str::to_owned),
        support_url: string_prop(node, "support-url").map(str::to_owned),
        description: string_prop(node, "description").map(str::to_owned),
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
    children_named(root, "param")
        .map(|node| {
            let id = required_string(path, source, node, "id")?;
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
                name: display_name(node, &id),
                flags: named_flags(path, source, node, "flags", &id, PARAM_FLAGS)?,
                unit: string_prop(node, "unit").map(token),
                id,
                min,
                max,
                default,
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
    children(root)
        .filter_map(|node| match node.name().value() {
            "input" => Some((node, Direction::Input)),
            "output" => Some((node, Direction::Output)),
            _ => None,
        })
        .map(|(node, direction)| {
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
                name: display_name(node, &id),
                port_type: string_prop(node, "type").map(token),
                flags: named_flags(path, source, node, "flags", &id, AUDIO_FLAGS)?,
                in_place_pair: string_prop(node, "in-place-pair").map(str::to_owned),
                id,
                direction,
                channels,
            })
        })
        .collect()
}

fn build_note_ports(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<NotePortIr>, String> {
    children(root)
        .filter_map(|node| match node.name().value() {
            "input" => Some((node, Direction::Input)),
            "output" => Some((node, Direction::Output)),
            _ => None,
        })
        .map(|(node, direction)| {
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
            Ok(NotePortIr { name: display_name(node, &id), id, direction, dialects, preferred })
        })
        .collect()
}

fn build_note_names(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<NoteNameIr>, String> {
    children_named(root, "note-name")
        .map(|node| {
            let name = first_string(node).ok_or_else(|| {
                diagnostic(
                    path,
                    source,
                    "note-name",
                    "note-name requires a string name argument",
                    "use `note-name \"C4\" ...`",
                )
            })?;
            Ok(NoteNameIr {
                name: name.to_owned(),
                key: integer_prop(node, "key"),
                channel: integer_prop(node, "channel"),
                port: string_prop(node, "port").map(str::to_owned),
            })
        })
        .collect()
}

fn build_state(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<Vec<StateFieldIr>, String> {
    children_named(root, "field")
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

fn build_gui(path: &Path, source: &str, root: Option<&KdlNode>) -> Result<GuiIr, String> {
    let mut gui = GuiIr::default();
    for node in children(root) {
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
                let resource_path =
                    first_string(node).or_else(|| string_prop(node, "path")).ok_or_else(|| {
                        diagnostic(
                            path,
                            source,
                            "resource",
                            "GUI resource is missing a path",
                            "provide a string argument or `path=` property",
                        )
                    })?;
                gui.resources.push(ResourceIr {
                    path: normalize_path(resource_path),
                    mime: string_prop(node, "mime").map(token),
                });
            }
            _ => {}
        }
    }
    Ok(gui)
}

fn build_presets(root: Option<&KdlNode>) -> PresetsIr {
    let mut presets = PresetsIr::default();
    for node in children(root) {
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
    children_named(root, "factory")
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
                kind: string_prop(node, "kind").map_or_else(|| "plugin".to_owned(), token),
            })
        })
        .collect()
}

fn build_extensions(
    path: &Path,
    source: &str,
    root: Option<&KdlNode>,
) -> Result<(Vec<ExtensionIr>, Vec<ExtensionIr>), String> {
    let mut stable = Vec::new();
    let mut draft = Vec::new();
    for node in children_named(root, "enable") {
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
                &format!(
                    "draft extension `{id}` must declare an exact ABI ID and matching `version` pin"
                ),
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
    version.is_some_and(|version| {
        id.rsplit_once('/').is_some_and(|(_, abi)| !abi.is_empty() && abi == version)
    })
}

fn canonicalize(ir: &mut CanonicalIr) {
    ir.parameters.sort_by(|a, b| a.id.cmp(&b.id));
    ir.audio_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    ir.note_ports.sort_by(|a, b| (a.direction, &a.id).cmp(&(b.direction, &b.id)));
    ir.note_names.sort_by(|a, b| {
        (&a.name, a.key, a.channel, &a.port).cmp(&(&b.name, b.key, b.channel, &b.port))
    });
    ir.state_fields.sort_by(|a, b| a.name.cmp(&b.name));
    ir.gui.apis.sort_by(|a, b| a.name.cmp(&b.name));
    ir.gui.resources.sort_by(|a, b| a.path.cmp(&b.path));
    ir.presets.locations.sort_by(|a, b| a.name.cmp(&b.name));
    ir.presets.formats.sort_by(|a, b| a.name.cmp(&b.name));
    ir.factories.sort_by(|a, b| a.id.cmp(&b.id));
    ir.stable_extensions.sort_by(|a, b| a.id.cmp(&b.id));
    ir.draft_extensions.sort_by(|a, b| a.id.cmp(&b.id));
    ir.imports.sort();
    ir.imports.dedup();
}

fn validate_unique_ids(path: &Path, source: &str, ir: &CanonicalIr) -> Result<(), String> {
    unique(path, source, ir.parameters.iter().map(|value| value.id.as_str()), "parameter")?;
    unique(path, source, ir.audio_ports.iter().map(|value| value.id.as_str()), "audio port")?;
    unique(path, source, ir.note_ports.iter().map(|value| value.id.as_str()), "note port")?;
    unique(path, source, ir.state_fields.iter().map(|value| value.name.as_str()), "state field")?;
    unique(path, source, ir.factories.iter().map(|value| value.id.as_str()), "factory")?;
    unique(
        path,
        source,
        ir.stable_extensions.iter().chain(&ir.draft_extensions).map(|value| value.id.as_str()),
        "extension",
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
                &format!(
                    "audio port `{}` has in-place-pair reference to missing target `{target_id}`",
                    port.id
                ),
                "reference an existing opposite-direction audio port ID",
            ));
        };
        if target.direction == port.direction || target.channels != port.channels {
            return Err(diagnostic(
                path,
                source,
                port.direction.as_str(),
                &format!(
                    "audio port `{}` has incompatible in-place-pair target `{target_id}`",
                    port.id
                ),
                "in-place pairs require opposite directions and matching channel counts",
            ));
        }
    }
    Ok(())
}

fn validate_note_name_references(
    path: &Path,
    source: &str,
    ports: &[NotePortIr],
    names: &[NoteNameIr],
) -> Result<(), String> {
    let ids = ports.iter().map(|port| port.id.as_str()).collect::<BTreeSet<_>>();
    for name in names {
        if let Some(port) = name.port.as_deref()
            && !ids.contains(port)
        {
            return Err(diagnostic(
                path,
                source,
                "note-name",
                &format!("note-name `{}` references missing note port target `{port}`", name.name),
                "reference an existing note port ID",
            ));
        }
    }
    Ok(())
}

fn validate_dependencies(path: &Path, source: &str, ir: &CanonicalIr) -> Result<(), String> {
    let ids = ir
        .stable_extensions
        .iter()
        .chain(&ir.draft_extensions)
        .map(|extension| extension.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids.contains("clap.note-expression") && ir.note_ports.is_empty() {
        return Err(diagnostic(
            path,
            source,
            "enable",
            "extension `clap.note-expression` requires at least one note port",
            "declare a note input or output before enabling note expressions",
        ));
    }
    if ids.iter().any(|id| id.starts_with("clap.webview/")) && ir.gui.apis.is_empty() {
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
    metadata.imports.iter().map(|value| normalize_path(&value.to_string_lossy())).collect()
}

pub(crate) fn serialize_ir_kdl(ir: &CanonicalIr) -> String {
    let mut out = String::new();
    writeln!(&mut out, "ir version={}", ir.version).expect("String write cannot fail");
    write_plugin(&mut out, &ir.plugin);
    writeln!(
        &mut out,
        "processor class={}{}",
        quote(&ir.processor.class),
        list_prop("features", &ir.processor.features)
    )
    .expect("String write cannot fail");
    write_parameters(&mut out, &ir.parameters);
    write_audio_ports(&mut out, &ir.audio_ports);
    write_note_ports(&mut out, &ir.note_ports, &ir.note_names);
    write_state(&mut out, &ir.state_fields);
    write_gui(&mut out, &ir.gui);
    write_presets(&mut out, &ir.presets);
    write_factories(&mut out, &ir.factories);
    write_extensions(&mut out, &ir.stable_extensions, &ir.draft_extensions);
    write_imports(&mut out, &ir.imports);
    out.push_str(&capability_report_kdl(ir));
    out
}

fn write_plugin(out: &mut String, plugin: &PluginIr) {
    writeln!(
        out,
        "plugin id={} name={} vendor={} version={}{}{}{}{}{}",
        quote(&plugin.id),
        quote(&plugin.name),
        quote(&plugin.vendor),
        quote(&plugin.version),
        option_prop("url", plugin.url.as_deref()),
        option_prop("manual-url", plugin.manual_url.as_deref()),
        option_prop("support-url", plugin.support_url.as_deref()),
        option_prop("description", plugin.description.as_deref()),
        list_prop("features", &plugin.features)
    )
    .expect("String write cannot fail");
}

fn write_parameters(out: &mut String, parameters: &[ParameterIr]) {
    out.push_str("parameters {\n");
    for value in parameters {
        writeln!(
            out,
            "    param id={} name={} min={} max={} default={}{}{}{}",
            quote(&value.id),
            quote(&value.name),
            number_text(value.min),
            number_text(value.max),
            number_text(value.default),
            list_prop("flags", &value.flags),
            option_prop("unit", value.unit.as_deref()),
            value.steps.map(|steps| format!(" steps={steps}")).unwrap_or_default()
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_audio_ports(out: &mut String, ports: &[AudioPortIr]) {
    out.push_str("audio-ports {\n");
    for value in ports {
        writeln!(
            out,
            "    {} id={} name={} channels={}{}{}{}",
            value.direction.as_str(),
            quote(&value.id),
            quote(&value.name),
            value.channels,
            option_prop("type", value.port_type.as_deref()),
            list_prop("flags", &value.flags),
            option_prop("in-place-pair", value.in_place_pair.as_deref())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_note_ports(out: &mut String, ports: &[NotePortIr], names: &[NoteNameIr]) {
    out.push_str("note-ports {\n");
    for value in ports {
        writeln!(
            out,
            "    {} id={} name={}{}{}",
            value.direction.as_str(),
            quote(&value.id),
            quote(&value.name),
            list_prop("dialects", &value.dialects),
            option_prop("preferred", value.preferred.as_deref())
        )
        .expect("String write cannot fail");
    }
    for value in names {
        writeln!(
            out,
            "    note-name {}{}{}{}",
            quote(&value.name),
            value.key.map(|key| format!(" key={key}")).unwrap_or_default(),
            value.channel.map(|channel| format!(" channel={channel}")).unwrap_or_default(),
            option_prop("port", value.port.as_deref())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_state(out: &mut String, fields: &[StateFieldIr]) {
    out.push_str("state {\n");
    for value in fields {
        writeln!(
            out,
            "    field {} type={}{}{}",
            quote(&value.name),
            quote(&value.field_type),
            value.default.as_ref().map(|default| format!(" default={default}")).unwrap_or_default(),
            option_prop("tag", value.tag.as_deref())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_gui(out: &mut String, gui: &GuiIr) {
    out.push_str("gui {\n");
    for value in &gui.apis {
        writeln!(
            out,
            "    api {} floating={} embedded={}",
            quote(&value.name),
            bool_text(value.floating),
            bool_text(value.embedded)
        )
        .expect("String write cannot fail");
    }
    for value in &gui.resources {
        writeln!(
            out,
            "    resource {}{}",
            quote(&value.path),
            option_prop("mime", value.mime.as_deref())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_presets(out: &mut String, presets: &PresetsIr) {
    out.push_str("presets {\n");
    for value in &presets.locations {
        writeln!(
            out,
            "    location {}{}{}",
            quote(&value.name),
            option_prop("kind", value.kind.as_deref()),
            option_prop("path", value.path.as_deref())
        )
        .expect("String write cannot fail");
    }
    for value in &presets.formats {
        writeln!(
            out,
            "    format {}{}{}",
            quote(&value.name),
            option_prop("extension", value.extension.as_deref()),
            option_prop("mime", value.mime.as_deref())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_factories(out: &mut String, factories: &[FactoryIr]) {
    out.push_str("factories {\n");
    for value in factories {
        writeln!(out, "    factory {} kind={}", quote(&value.id), quote(&value.kind))
            .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_extensions(out: &mut String, stable: &[ExtensionIr], draft: &[ExtensionIr]) {
    out.push_str("extensions {\n");
    for value in stable {
        writeln!(
            out,
            "    stable {}{}",
            quote(&value.id),
            option_prop("version", value.version.as_deref())
        )
        .expect("String write cannot fail");
    }
    for value in draft {
        writeln!(
            out,
            "    draft {} version={}",
            quote(&value.id),
            quote(value.version.as_deref().unwrap_or_default())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
}

fn write_imports(out: &mut String, imports: &[String]) {
    out.push_str("imports {\n");
    for value in imports {
        writeln!(out, "    import {}", quote(value)).expect("String write cannot fail");
    }
    out.push_str("}\n");
}

pub(crate) fn capability_report_kdl(ir: &CanonicalIr) -> String {
    let mut out = String::from("capabilities {\n");
    writeln!(&mut out, "    parameters count={}", ir.parameters.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    audio-ports count={}", ir.audio_ports.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    note-ports count={}", ir.note_ports.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    note-names count={}", ir.note_names.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    state-fields count={}", ir.state_fields.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    gui-apis count={}", ir.gui.apis.len())
        .expect("String write cannot fail");
    writeln!(&mut out, "    factories count={}", ir.factories.len())
        .expect("String write cannot fail");
    for value in &ir.stable_extensions {
        writeln!(
            &mut out,
            "    extension {} stability=\"stable\"{}",
            quote(&value.id),
            option_prop("version", value.version.as_deref())
        )
        .expect("String write cannot fail");
    }
    for value in &ir.draft_extensions {
        writeln!(
            &mut out,
            "    extension {} stability=\"draft\" version={}",
            quote(&value.id),
            quote(value.version.as_deref().unwrap_or_default())
        )
        .expect("String write cannot fail");
    }
    out.push_str("}\n");
    out
}

fn children(root: Option<&KdlNode>) -> impl Iterator<Item = &KdlNode> {
    root.and_then(KdlNode::children).into_iter().flat_map(KdlDocument::nodes)
}

fn children_named<'a>(
    root: Option<&'a KdlNode>,
    name: &'a str,
) -> impl Iterator<Item = &'a KdlNode> {
    children(root).filter(move |node| node.name().value() == name)
}

fn display_name(node: &KdlNode, id: &str) -> String {
    first_string(node).or_else(|| string_prop(node, "name")).unwrap_or(id).trim().to_owned()
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
            &format!(
                "`{subject}` property `{key}` must use named symbolic values; raw numeric C bitmasks are not accepted"
            ),
            "use a comma-separated string of symbolic names",
        ));
    };
    let mut values =
        value.split(',').map(token).filter(|value| !value.is_empty()).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    Ok(values)
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
        KdlValue::Float(value) => number_text(*value),
        KdlValue::Bool(value) => bool_text(*value).to_owned(),
        KdlValue::Null => "#null".to_owned(),
    }
}

fn number_text(value: f64) -> String {
    if value == 0.0 { "0".to_owned() } else { value.to_string() }
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
            "{PREFIX}parameters {{\n    param \"cutoff\" id=\"cutoff\" min=20.0 max=20000.0 default=1000.0 flags=\"modulatable,automatable\" unit=\"Hz\"\n    param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=\"automatable\"\n}}\naudio-ports {{ input \"main\" id=\"in\" channels=2; output \"main\" id=\"out\" channels=2 }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );
        let b = format!(
            "{PREFIX}parameters {{\n    param \"gain\" default=0.5 max=1.0 min=0.0 id=\"gain\" flags=\"automatable\"\n    param \"cutoff\" default=1000.0 max=20000.0 min=20.0 id=\"cutoff\" unit=\"hz\" flags=\"automatable, modulatable\"\n}}\naudio-ports {{ output \"main\" channels=2 id=\"out\"; input \"main\" channels=2 id=\"in\" }}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.params\" }}\n"
        );
        let a = build(&a).expect("first manifest should build");
        let b = build(&b).expect("equivalent manifest should build");
        assert_eq!(serialize_ir_kdl(&a), serialize_ir_kdl(&b));
    }

    #[test]
    fn rejects_raw_numeric_clap_flag_bitmasks() {
        let source = format!(
            "{PREFIX}parameters {{ param \"gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=3 }}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("raw C bitmask must be rejected");
        assert!(error.contains("flags"), "{error}");
        assert!(error.contains("named"), "{error}");
        assert!(error.contains("gain"), "{error}");
    }

    #[test]
    fn cross_reference_error_identifies_source_and_missing_target() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{\n    input \"main\" id=\"in\" channels=2\n    output \"main\" id=\"out\" channels=2 in-place-pair=\"missing-input\"\n}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{}}\n"
        );
        let error = build(&source).expect_err("missing target must fail");
        assert!(error.contains("out"), "{error}");
        assert!(error.contains("missing-input"), "{error}");
        assert!(error.contains("in-place-pair"), "{error}");
    }

    #[test]
    fn draft_extensions_require_exact_abi_id_and_version_pin() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.webview\" draft=#true }}\n"
        );
        let error = build(&source).expect_err("unpinned draft must fail");
        assert!(error.contains("draft"), "{error}");
        assert!(error.contains("exact ABI"), "{error}");
        assert!(error.contains("version"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{ api \"web\" }}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.webview/3\" version=\"3\" draft=#true }}\n"
        );
        build(&valid).expect("exact draft ABI should be accepted");
    }

    #[test]
    fn capability_dependencies_are_validated_and_reported() {
        let invalid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let error = build(&invalid).expect_err("note expression requires note ports");
        assert!(error.contains("clap.note-expression"), "{error}");
        assert!(error.contains("note port"), "{error}");

        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{ input \"notes\" id=\"notes-in\" dialects=\"clap\" preferred=\"clap\" }}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{ enable \"clap.note-expression\" }}\n"
        );
        let ir = build(&valid).expect("dependency should be satisfied");
        let report = capability_report_kdl(&ir);
        assert!(report.contains("clap.note-expression"));
        assert!(report.contains("stable"));
    }

    #[test]
    fn ir_serialization_has_a_versioned_compatibility_marker() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{}}\n"
        );
        let ir = build(&source).expect("manifest should build");
        let serialized = serialize_ir_kdl(&ir);
        assert!(serialized.starts_with("ir version=1\n"), "{serialized}");
        assert_eq!(serialized, serialize_ir_kdl(&ir));
    }

    #[test]
    fn serialization_preserves_descriptor_fields_and_plugin_features() {
        let source = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{}}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{}}\n"
        )
        .replace(
            "plugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"",
            "plugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\" url=\"https://example.test\" description=\"Demo\" { feature \"instrument\"; feature \"synthesizer\" }",
        );
        let ir = build(&source).expect("descriptor manifest should build");
        let serialized = serialize_ir_kdl(&ir);
        assert!(serialized.contains("url=\"https://example.test\""), "{serialized}");
        assert!(serialized.contains("description=\"Demo\""), "{serialized}");
        assert!(serialized.contains("features=\"instrument,synthesizer\""), "{serialized}");
    }

    #[test]
    fn note_names_are_preserved_and_cross_referenced() {
        let valid = format!(
            "{PREFIX}parameters {{}}\naudio-ports {{}}\nnote-ports {{ input \"notes\" id=\"notes-in\" dialects=\"clap\" preferred=\"clap\"; note-name \"Kick\" key=36 port=\"notes-in\" }}\nstate {{}}\ngui {{}}\npresets {{}}\nfactories {{}}\nextensions {{}}\n"
        );
        let ir = build(&valid).expect("note-name reference should resolve");
        assert!(
            serialize_ir_kdl(&ir).contains("note-name \"Kick\" key=36 port=\"notes-in\""),
            "{}",
            serialize_ir_kdl(&ir)
        );

        let invalid = valid.replace("port=\"notes-in\"", "port=\"missing\"");
        let error = build(&invalid).expect_err("missing note port should fail");
        assert!(error.contains("Kick"), "{error}");
        assert!(error.contains("missing"), "{error}");
    }

    #[test]
    fn leading_parent_imports_are_preserved_during_normalization() {
        let source = "clapgen schema=\"1.0.0\"\nimport \"../shared/common.kdl\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"SynthProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n".to_owned();
        let ir = build(&source).expect("parent-relative import should build");
        assert!(serialize_ir_kdl(&ir).contains("import \"../shared/common.kdl\""));
    }
}
