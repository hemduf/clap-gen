use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::metadata::{ParsedMetadata, parse_metadata};

mod legacy {
    include!("ir.rs");
}

const CLAP_SDK_PIN: &str = "a47f6badb49d948fd009998f28309cdab78979c9";
const PINNED_DRAFT_EXTENSIONS: &[&str] = &[
    "clap.background-activation/1",
    "clap.background-progress/1",
    "clap.background-state-context/1",
    "clap.extensible-audio-ports/1",
    "clap.flush-events/1",
    "clap.gain-adjustment-metering/0",
    "clap.mini-curve-display/3",
    "clap.octave-number/1",
    "clap.param-hovered/1",
    "clap.params-origin/1",
    "clap.preset-load.draft/2",
    "clap.project-location/2",
    "clap.resource-directory/1",
    "clap.scratch-memory/1",
    "clap.transport-control/2",
    "clap.triggers/1",
    "clap.tuning/2",
    "clap.undo/4",
    "clap.undo_context/4",
    "clap.undo_delta/4",
    "clap.webview/3",
];
const SECTIONS: &[&str] = &[
    "parameters", "audio-ports", "note-ports", "state", "gui", "presets", "factories",
    "extensions",
];

#[derive(Debug)]
pub(crate) struct ExtensionSet(usize);
impl ExtensionSet {
    pub(crate) const fn len(&self) -> usize { self.0 }
}

pub(crate) struct CanonicalIr {
    pub(crate) version: u32,
    pub(crate) stable_extensions: ExtensionSet,
    pub(crate) draft_extensions: ExtensionSet,
    inner: legacy::CanonicalIr,
    replacements: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
struct LoadedImport { metadata: ParsedMetadata }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PortDirection { Input, Output }
impl PortDirection {
    fn from_node(name: &str) -> Option<Self> {
        match name { "input" => Some(Self::Input), "output" => Some(Self::Output), _ => None }
    }
    const fn as_str(self) -> &'static str { match self { Self::Input => "input", Self::Output => "output" } }
    const fn opposite(self) -> Self { match self { Self::Input => Self::Output, Self::Output => Self::Input } }
}

#[derive(Default)]
struct RewritePlan {
    audio_ids: BTreeMap<(PortDirection, String), String>,
    note_ids: BTreeMap<(PortDirection, String), String>,
    stable_extension_ids: BTreeMap<String, String>,
    replacements: Vec<(String, String)>,
}

pub(crate) fn build_ir(path: &Path, source: &str, metadata: &ParsedMetadata) -> Result<CanonicalIr, String> {
    let merged_source = merge_semantic_imports(path, source, metadata)?;
    let merged_metadata = parse_metadata(path, &merged_source)?;
    validate_parameter_semantics(path, &merged_source, &merged_metadata.document)?;
    validate_main_audio_ports(path, &merged_source, &merged_metadata.document)?;
    validate_pinned_extension_contract(path, &merged_source, &merged_metadata.document)?;
    let plan = build_rewrite_plan(path, &merged_source, &merged_metadata.document)?;
    let legacy_source = render_for_legacy(&merged_metadata.document, &plan);
    let legacy_metadata = parse_metadata(path, &legacy_source)?;
    let inner = legacy::build_ir(path, &legacy_source, &legacy_metadata)
        .map_err(|error| restore_text(error, &plan.replacements))?;
    Ok(CanonicalIr {
        version: inner.version,
        stable_extensions: ExtensionSet(inner.stable_extensions.len()),
        draft_extensions: ExtensionSet(inner.draft_extensions.len()),
        inner,
        replacements: plan.replacements,
    })
}

pub(crate) fn serialize_ir_kdl(ir: &CanonicalIr) -> String {
    restore_text(legacy::serialize_ir_kdl(&ir.inner), &ir.replacements)
}
pub(crate) fn capability_report_kdl(ir: &CanonicalIr) -> String {
    restore_text(legacy::capability_report_kdl(&ir.inner), &ir.replacements)
}

fn merge_semantic_imports(path: &Path, _source: &str, metadata: &ParsedMetadata) -> Result<String, String> {
    let mut loaded = Vec::new();
    let mut loaded_paths = BTreeSet::new();
    let mut stack = BTreeSet::new();
    if let Ok(root) = fs::canonicalize(path) { stack.insert(root); }
    load_imports(path, metadata, &mut loaded, &mut loaded_paths, &mut stack)?;
    let mut out = String::new();
    append_required_root_node(&mut out, &metadata.document, "clapgen")?;
    append_optional_root_node(&mut out, &metadata.document, "plugin");
    append_optional_root_node(&mut out, &metadata.document, "processor");
    for node in metadata.document.nodes().iter().filter(|node| node.name().value() == "import") { append_top_node(&mut out, node); }
    for section in SECTIONS {
        out.push_str(section); out.push_str(" {\n");
        append_section_children(&mut out, &metadata.document, section);
        for import in &loaded { append_section_children(&mut out, &import.metadata.document, section); }
        out.push_str("}\n");
    }
    Ok(out)
}

fn load_imports(owner_path: &Path, metadata: &ParsedMetadata, loaded: &mut Vec<LoadedImport>, loaded_paths: &mut BTreeSet<PathBuf>, stack: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    for node in metadata.document.nodes().iter().filter(|node| node.name().value() == "import") {
        let Some(relative) = first_string(node) else { continue; };
        let optional = bool_prop(node, "optional").unwrap_or(false);
        let candidate = owner_path.parent().unwrap_or_else(|| Path::new(".")).join(relative);
        let canonical = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(_) if optional => continue,
            Err(error) => return Err(format!("{}:1: failed to resolve imported metadata `{}`: {error}\nhint: fix the import path or mark it optional", owner_path.display(), candidate.display())),
        };
        if stack.contains(&canonical) { return Err(format!("{}:1: semantic import cycle reaches `{}`\nhint: remove the cyclic import", owner_path.display(), canonical.display())); }
        if loaded_paths.contains(&canonical) { continue; }
        let source = fs::read_to_string(&canonical).map_err(|error| format!("{}:1: failed to read imported metadata `{}`: {error}", owner_path.display(), canonical.display()))?;
        let parsed = parse_metadata(&canonical, &source)?;
        if parsed.document.get("plugin").is_some() || parsed.document.get("processor").is_some() {
            return Err(format!("{}:1: imported metadata may not redefine `plugin` or `processor`\nhint: keep descriptors in the root manifest and import semantic fragments only", canonical.display()));
        }
        stack.insert(canonical.clone());
        load_imports(&canonical, &parsed, loaded, loaded_paths, stack)?;
        stack.remove(&canonical);
        loaded_paths.insert(canonical);
        loaded.push(LoadedImport { metadata: parsed });
    }
    Ok(())
}

fn append_required_root_node(out: &mut String, document: &KdlDocument, name: &str) -> Result<(), String> {
    let node = document.get(name).ok_or_else(|| format!("missing `{name}` root node"))?; append_top_node(out, node); Ok(())
}
fn append_optional_root_node(out: &mut String, document: &KdlDocument, name: &str) { if let Some(node) = document.get(name) { append_top_node(out, node); } }
fn append_top_node(out: &mut String, node: &KdlNode) { let text = node.to_string(); out.push_str(text.trim_end_matches('\n')); out.push('\n'); }
fn append_section_children(out: &mut String, document: &KdlDocument, section: &str) {
    for root in document.nodes().iter().filter(|node| node.name().value() == section) {
        let Some(children) = root.children() else { continue; };
        for child in children.nodes() { append_indented_text(out, &child.to_string()); }
    }
}
fn append_indented_text(out: &mut String, text: &str) { for line in text.trim_end_matches('\n').lines() { out.push_str("    "); out.push_str(line); out.push('\n'); } }

fn validate_parameter_semantics(path: &Path, source: &str, document: &KdlDocument) -> Result<(), String> {
    let mut bypass_ids = Vec::new();
    for node in section_children(document, "parameters").filter(|node| node.name().value() == "param") {
        let id = string_prop(node, "id").unwrap_or("<unknown>");
        let flags = symbolic_list(node, "flags");
        let stepped = flags.iter().any(|flag| flag == "stepped");
        let bypass = flags.iter().any(|flag| flag == "bypass");
        let enumeration = flags.iter().any(|flag| flag == "enum");
        if bypass && !stepped { return Err(diagnostic(path, source, "param", &format!("bypass parameter `{id}` must also declare the `stepped` flag"), "use flags=\"bypass,stepped\"")); }
        if enumeration && !stepped { return Err(diagnostic(path, source, "param", &format!("enum parameter `{id}` must also declare the `stepped` flag"), "use flags=\"enum,stepped\"")); }
        if bypass {
            bypass_ids.push(id.to_owned());
            if number_prop(node, "min") != Some(0.0) || number_prop(node, "max") != Some(1.0) { return Err(diagnostic(path, source, "param", &format!("bypass parameter `{id}` must use min=0 and max=1"), "CLAP bypass parameters have the fixed [0, 1] range")); }
        }
    }
    if bypass_ids.len() > 1 { return Err(diagnostic(path, source, "param", &format!("only one bypass parameter is allowed; found {}", bypass_ids.join(", ")), "keep a single parameter with the `bypass` flag")); }
    Ok(())
}

fn validate_main_audio_ports(path: &Path, source: &str, document: &KdlDocument) -> Result<(), String> {
    for direction in [PortDirection::Input, PortDirection::Output] {
        let mains = section_children(document, "audio-ports")
            .filter(|node| PortDirection::from_node(node.name().value()) == Some(direction))
            .filter(|node| symbolic_list(node, "flags").iter().any(|flag| flag == "main"))
            .filter_map(|node| string_prop(node, "id")).collect::<Vec<_>>();
        if mains.len() > 1 { return Err(diagnostic(path, source, direction.as_str(), &format!("only one main {} audio port is allowed; found {}", direction.as_str(), mains.join(", ")), "CLAP requires at most one main port per direction")); }
    }
    Ok(())
}

fn validate_pinned_extension_contract(path: &Path, source: &str, document: &KdlDocument) -> Result<(), String> {
    for node in section_children(document, "extensions").filter(|node| node.name().value() == "enable") {
        let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) else { continue; };
        let draft = bool_prop(node, "draft").unwrap_or(false);
        let known_draft = PINNED_DRAFT_EXTENSIONS.contains(&id);
        if draft && !known_draft { return Err(diagnostic(path, source, "enable", &format!("draft extension `{id}` is not present in pinned CLAP SDK `{CLAP_SDK_PIN}`"), "use an exact draft ABI ID from the pinned SDK or update the SDK pin explicitly")); }
        if !draft && known_draft { return Err(diagnostic(path, source, "enable", &format!("draft extension `{id}` requires explicit draft opt-in"), "add `draft=#true` and the exact matching `version` pin")); }
        if draft && !has_exact_version_pin(id, string_prop(node, "version")) { return Err(diagnostic(path, source, "enable", &format!("draft extension `{id}` must use its exact ABI version pin"), "make `version` match the numeric suffix in the extension ID")); }
    }
    Ok(())
}
fn has_exact_version_pin(id: &str, version: Option<&str>) -> bool { version.is_some_and(|version| id.rsplit_once('/').is_some_and(|(_, abi)| !abi.is_empty() && abi == version)) }

fn build_rewrite_plan(path: &Path, source: &str, document: &KdlDocument) -> Result<RewritePlan, String> {
    let mut plan = RewritePlan::default();
    for node in section_children(document, "audio-ports") {
        let Some(direction) = PortDirection::from_node(node.name().value()) else { continue; };
        let Some(id) = string_prop(node, "id") else { continue; };
        let main = symbolic_list(node, "flags").iter().any(|flag| flag == "main");
        let internal = format!("__clapgen_audio_{}_{}_{}", i32::from(!main), direction.as_str(), hex(id));
        plan.audio_ids.insert((direction, id.to_owned()), internal.clone()); plan.replacements.push((internal, id.to_owned()));
    }
    for node in section_children(document, "note-ports") {
        let Some(direction) = PortDirection::from_node(node.name().value()) else { continue; };
        let Some(id) = string_prop(node, "id") else { continue; };
        let internal = format!("__clapgen_note_{}_{}", direction.as_str(), hex(id));
        plan.note_ids.insert((direction, id.to_owned()), internal.clone()); plan.replacements.push((internal, id.to_owned()));
    }
    for node in section_children(document, "note-ports").filter(|node| node.name().value() == "note-name") {
        let Some(target) = string_prop(node, "port") else { continue; };
        if plan.note_ids.keys().filter(|(_, id)| id == target).count() > 1 {
            let name = first_string(node).unwrap_or("<unnamed>");
            return Err(diagnostic(path, source, "note-name", &format!("note-name `{name}` has ambiguous note port target `{target}` because that ID exists in both directions"), "use distinct note port IDs when note-name metadata references a port"));
        }
    }
    for node in section_children(document, "extensions").filter(|node| node.name().value() == "enable") {
        let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) else { continue; };
        if !bool_prop(node, "draft").unwrap_or(false) && id.starts_with("clap.") && id.contains('/') {
            let internal = format!("__clapgen_stable_extension_{}", hex(id));
            plan.stable_extension_ids.insert(id.to_owned(), internal.clone()); plan.replacements.push((internal, id.to_owned()));
        }
    }
    Ok(plan)
}

fn render_for_legacy(document: &KdlDocument, plan: &RewritePlan) -> String {
    let mut out = String::new();
    for node in document.nodes() {
        match node.name().value() {
            "audio-ports" => render_audio_ports(&mut out, node, plan),
            "note-ports" => render_note_ports(&mut out, node, plan),
            "extensions" => render_extensions(&mut out, node, plan),
            _ => append_top_node(&mut out, node),
        }
    }
    out
}
fn render_audio_ports(out: &mut String, root: &KdlNode, plan: &RewritePlan) {
    out.push_str("audio-ports {\n"); let Some(children) = root.children() else { out.push_str("}\n"); return; };
    for node in children.nodes() {
        let Some(direction) = PortDirection::from_node(node.name().value()) else { append_indented_text(out, &node.to_string()); continue; };
        let id = string_prop(node, "id").unwrap_or(""); let internal = plan.audio_ids.get(&(direction, id.to_owned())).map_or(id, String::as_str);
        out.push_str("    "); out.push_str(direction.as_str());
        if let Some(name) = first_string(node).or_else(|| string_prop(node, "name")) { out.push_str(" name="); out.push_str(&quote(name)); }
        out.push_str(" id="); out.push_str(&quote(internal));
        if let Some(channels) = prop(node, "channels") { out.push_str(" channels="); out.push_str(&value_text(channels)); }
        append_optional_string_prop(out, node, "type"); append_optional_string_prop(out, node, "flags");
        if let Some(target) = string_prop(node, "in-place-pair") { let rewritten = plan.audio_ids.get(&(direction.opposite(), target.to_owned())).map_or(target, String::as_str); out.push_str(" in-place-pair="); out.push_str(&quote(rewritten)); }
        out.push('\n');
    }
    out.push_str("}\n");
}
fn render_note_ports(out: &mut String, root: &KdlNode, plan: &RewritePlan) {
    out.push_str("note-ports {\n"); let Some(children) = root.children() else { out.push_str("}\n"); return; };
    for node in children.nodes() {
        if let Some(direction) = PortDirection::from_node(node.name().value()) {
            let id = string_prop(node, "id").unwrap_or(""); let internal = plan.note_ids.get(&(direction, id.to_owned())).map_or(id, String::as_str);
            out.push_str("    "); out.push_str(direction.as_str());
            if let Some(name) = first_string(node).or_else(|| string_prop(node, "name")) { out.push_str(" name="); out.push_str(&quote(name)); }
            out.push_str(" id="); out.push_str(&quote(internal)); append_optional_string_prop(out, node, "dialects"); append_optional_string_prop(out, node, "preferred"); out.push('\n'); continue;
        }
        if node.name().value() == "note-name" {
            out.push_str("    note-name"); if let Some(name) = first_string(node) { out.push(' '); out.push_str(&quote(name)); }
            append_optional_value_prop(out, node, "key"); append_optional_value_prop(out, node, "channel");
            if let Some(target) = string_prop(node, "port") { let rewritten = plan.note_ids.iter().find(|((_, id), _)| id == target).map_or(target, |(_, internal)| internal.as_str()); out.push_str(" port="); out.push_str(&quote(rewritten)); }
            out.push('\n'); continue;
        }
        append_indented_text(out, &node.to_string());
    }
    out.push_str("}\n");
}
fn render_extensions(out: &mut String, root: &KdlNode, plan: &RewritePlan) {
    out.push_str("extensions {\n"); let Some(children) = root.children() else { out.push_str("}\n"); return; };
    for node in children.nodes() {
        if node.name().value() != "enable" { append_indented_text(out, &node.to_string()); continue; }
        let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) else { append_indented_text(out, &node.to_string()); continue; };
        let rewritten = plan.stable_extension_ids.get(id).map_or(id, String::as_str); out.push_str("    enable "); out.push_str(&quote(rewritten)); append_optional_string_prop(out, node, "version");
        if let Some(draft) = bool_prop(node, "draft") { out.push_str(" draft="); out.push_str(if draft { "#true" } else { "#false" }); }
        out.push('\n');
    }
    out.push_str("}\n");
}
fn append_optional_string_prop(out: &mut String, node: &KdlNode, name: &str) { if let Some(value) = string_prop(node, name) { out.push(' '); out.push_str(name); out.push('='); out.push_str(&quote(value)); } }
fn append_optional_value_prop(out: &mut String, node: &KdlNode, name: &str) { if let Some(value) = prop(node, name) { out.push(' '); out.push_str(name); out.push('='); out.push_str(&value_text(value)); } }
fn section_children<'a>(document: &'a KdlDocument, section: &'a str) -> impl Iterator<Item = &'a KdlNode> { document.nodes().iter().filter(move |node| node.name().value() == section).flat_map(|node| node.children().into_iter().flat_map(KdlDocument::nodes)) }
fn first_string(node: &KdlNode) -> Option<&str> { node.entries().iter().find_map(|entry| { if entry.name().is_some() { return None; } match entry.value() { KdlValue::String(value) => Some(value.as_str()), _ => None } }) }
fn prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlValue> { node.entries().iter().rev().find_map(|entry| { let name = entry.name()?; (name.value() == key).then_some(entry.value()) }) }
fn string_prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> { match prop(node, key)? { KdlValue::String(value) => Some(value.as_str()), _ => None } }
fn bool_prop(node: &KdlNode, key: &str) -> Option<bool> { match prop(node, key)? { KdlValue::Bool(value) => Some(*value), _ => None } }
fn number_prop(node: &KdlNode, key: &str) -> Option<f64> { match prop(node, key)? { KdlValue::Float(value) if value.is_finite() => Some(*value), KdlValue::Integer(value) => value.to_string().parse().ok(), _ => None } }
fn symbolic_list(node: &KdlNode, key: &str) -> Vec<String> { string_prop(node, key).map(|value| value.split(',').map(|item| item.trim().to_ascii_lowercase()).filter(|item| !item.is_empty()).collect()).unwrap_or_default() }
fn value_text(value: &KdlValue) -> String { match value { KdlValue::String(value) => quote(value), KdlValue::Integer(value) => value.to_string(), KdlValue::Float(value) => if *value == 0.0 { "0".to_owned() } else { value.to_string() }, KdlValue::Bool(value) => if *value { "#true".to_owned() } else { "#false".to_owned() }, KdlValue::Null => "#null".to_owned() } }
fn quote(value: &str) -> String { let escaped = value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t"); format!("\"{escaped}\"") }
fn hex(value: &str) -> String { let mut out = String::with_capacity(value.len() * 2); for byte in value.as_bytes() { write!(&mut out, "{byte:02x}").expect("String write cannot fail"); } out }
fn restore_text(mut text: String, replacements: &[(String, String)]) -> String { for (internal, original) in replacements { text = text.replace(internal, original); } text }
fn diagnostic(path: &Path, source: &str, node: &str, message: &str, hint: &str) -> String { let line = source.lines().enumerate().find_map(|(index, line)| { let line = line.trim_start(); line.strip_prefix(node).is_some_and(|rest| rest.is_empty() || rest.starts_with('{') || rest.chars().next().is_some_and(char::is_whitespace)).then_some(index + 1) }).unwrap_or(1); format!("{}:{line}: {message}\nhint: {hint}", path.display()) }

#[cfg(test)]
mod review_tests {
    use super::{CLAP_SDK_PIN, PINNED_DRAFT_EXTENSIONS, has_exact_version_pin};
    #[test]
    fn pinned_draft_registry_is_exact_and_tied_to_the_dependency_pin() {
        assert_eq!("a47f6badb49d948fd009998f28309cdab78979c9", CLAP_SDK_PIN);
        assert!(PINNED_DRAFT_EXTENSIONS.contains(&"clap.webview/3"));
        assert!(!PINNED_DRAFT_EXTENSIONS.contains(&"clap.preset-load/2"));
        assert!(has_exact_version_pin("clap.webview/3", Some("3")));
        assert!(!has_exact_version_pin("clap.webview/3", Some("2")));
    }
}
