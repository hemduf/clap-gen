use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode, KdlValue};

use crate::ir::{CanonicalIr, serialize_ir_kdl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    Compatible,
    Sensitive,
    Forbidden,
}

impl Class {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Sensitive => "sensitive",
            Self::Forbidden => "forbidden",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    class: Class,
    subject: String,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Report {
    changes: Vec<Change>,
}

impl Report {
    pub(crate) fn has_forbidden(&self) -> bool {
        self.changes.iter().any(|change| change.class == Class::Forbidden)
    }

    pub(crate) fn text(&self) -> String {
        if self.changes.is_empty() {
            return "compatible: no semantic compatibility changes".to_owned();
        }
        self.changes
            .iter()
            .map(|change| format!("{} {}: {}", change.class.as_str(), change.subject, change.detail))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn json(&self) -> String {
        let body = self
            .changes
            .iter()
            .map(|change| {
                format!(
                    "{{\"class\":{},\"subject\":{},\"detail\":{}}}",
                    json_string(change.class.as_str()),
                    json_string(&change.subject),
                    json_string(&change.detail)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"changes\":[{body}],\"forbidden\":{}}}", self.has_forbidden())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Parameter {
    min: String,
    max: String,
    default: String,
    flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Port {
    direction: String,
    channels: Option<String>,
    dialects: Option<String>,
    preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Snapshot {
    plugin_id: String,
    parameters: BTreeMap<String, Parameter>,
    audio_ports: BTreeMap<String, Port>,
    note_ports: BTreeMap<String, Port>,
    state_tags: BTreeMap<String, Option<String>>,
    draft_extensions: BTreeMap<String, String>,
}

pub(crate) fn compare(baseline: &CanonicalIr, current: &CanonicalIr) -> Result<Report, String> {
    let baseline = snapshot(baseline)?;
    let current = snapshot(current)?;
    let mut changes = Vec::new();

    if baseline.plugin_id != current.plugin_id {
        changes.push(Change {
            class: Class::Forbidden,
            subject: "plugin.id".to_owned(),
            detail: format!("changed from `{}` to `{}`", baseline.plugin_id, current.plugin_id),
        });
    }

    compare_parameters(&baseline.parameters, &current.parameters, &mut changes);
    compare_ports("audio-port", &baseline.audio_ports, &current.audio_ports, &mut changes);
    compare_ports("note-port", &baseline.note_ports, &current.note_ports, &mut changes);
    compare_state(&baseline.state_tags, &current.state_tags, &mut changes);
    compare_drafts(&baseline.draft_extensions, &current.draft_extensions, &mut changes);

    changes.sort_by(|a, b| (&a.subject, a.class, &a.detail).cmp(&(&b.subject, b.class, &b.detail)));
    Ok(Report { changes })
}

fn compare_parameters(
    baseline: &BTreeMap<String, Parameter>,
    current: &BTreeMap<String, Parameter>,
    changes: &mut Vec<Change>,
) {
    for (id, old) in baseline {
        let Some(new) = current.get(id) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("parameter.{id}"),
                detail: "released parameter was removed".to_owned(),
            });
            continue;
        };
        if old.min != new.min || old.max != new.max {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.range"),
                detail: format!("changed from {}..{} to {}..{}", old.min, old.max, new.min, new.max),
            });
        }
        if old.default != new.default {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.default"),
                detail: format!("changed from {} to {}", old.default, new.default),
            });
        }
        if old.flags != new.flags {
            changes.push(Change {
                class: Class::Sensitive,
                subject: format!("parameter.{id}.flags"),
                detail: format!("changed from `{}` to `{}`", old.flags, new.flags),
            });
        }
    }
    for id in current.keys().filter(|id| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("parameter.{id}"),
            detail: "new parameter added".to_owned(),
        });
    }
}

fn compare_ports(
    kind: &str,
    baseline: &BTreeMap<String, Port>,
    current: &BTreeMap<String, Port>,
    changes: &mut Vec<Change>,
) {
    for (id, old) in baseline {
        let Some(new) = current.get(id) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{kind}.{id}"),
                detail: "released port was removed".to_owned(),
            });
            continue;
        };
        if old != new {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("{kind}.{id}"),
                detail: "released port topology changed".to_owned(),
            });
        }
    }
    for id in current.keys().filter(|id| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("{kind}.{id}"),
            detail: "new port changes host-visible topology".to_owned(),
        });
    }
}

fn compare_state(
    baseline: &BTreeMap<String, Option<String>>,
    current: &BTreeMap<String, Option<String>>,
    changes: &mut Vec<Change>,
) {
    for (name, old_tag) in baseline {
        let Some(new_tag) = current.get(name) else {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("state.{name}"),
                detail: "released state field was removed".to_owned(),
            });
            continue;
        };
        if old_tag != new_tag {
            changes.push(Change {
                class: Class::Forbidden,
                subject: format!("state.{name}.tag"),
                detail: format!("changed from {:?} to {:?}", old_tag, new_tag),
            });
        }
    }
    for name in current.keys().filter(|name| !baseline.contains_key(*name)) {
        changes.push(Change {
            class: Class::Compatible,
            subject: format!("state.{name}"),
            detail: "new state field added".to_owned(),
        });
    }
}

fn compare_drafts(
    baseline: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
    changes: &mut Vec<Change>,
) {
    for (id, version) in baseline {
        match current.get(id) {
            Some(current_version) if current_version == version => {}
            Some(current_version) => changes.push(Change {
                class: Class::Forbidden,
                subject: format!("draft.{id}"),
                detail: format!("ABI version changed from `{version}` to `{current_version}`"),
            }),
            None => changes.push(Change {
                class: Class::Forbidden,
                subject: format!("draft.{id}"),
                detail: "released draft ABI capability was removed or replaced".to_owned(),
            }),
        }
    }
    for (id, version) in current.iter().filter(|(id, _)| !baseline.contains_key(*id)) {
        changes.push(Change {
            class: Class::Sensitive,
            subject: format!("draft.{id}"),
            detail: format!("new draft ABI capability version `{version}`"),
        });
    }
}

fn snapshot(ir: &CanonicalIr) -> Result<Snapshot, String> {
    let source = serialize_ir_kdl(ir);
    let document = KdlDocument::parse_v2(&source)
        .map_err(|error| format!("internal canonical IR parse failed: {error}"))?;
    let plugin = document.get("plugin").ok_or_else(|| "internal IR is missing plugin".to_owned())?;
    let plugin_id = string_prop(plugin, "id")
        .ok_or_else(|| "internal IR plugin is missing id".to_owned())?
        .to_owned();

    let mut result = Snapshot { plugin_id, ..Snapshot::default() };
    if let Some(parameters) = document.get("parameters").and_then(KdlNode::children) {
        for node in parameters.nodes().iter().filter(|node| node.name().value() == "param") {
            let id = required_string(node, "id")?;
            result.parameters.insert(
                id,
                Parameter {
                    min: value_prop(node, "min")?,
                    max: value_prop(node, "max")?,
                    default: value_prop(node, "default")?,
                    flags: string_prop(node, "flags").unwrap_or_default().to_owned(),
                },
            );
        }
    }
    collect_ports(document.get("audio-ports"), false, &mut result.audio_ports)?;
    collect_ports(document.get("note-ports"), true, &mut result.note_ports)?;

    if let Some(state) = document.get("state").and_then(KdlNode::children) {
        for node in state.nodes().iter().filter(|node| node.name().value() == "field") {
            let name = first_string(node).ok_or_else(|| "internal state field missing name".to_owned())?;
            result.state_tags.insert(name.to_owned(), string_prop(node, "tag").map(str::to_owned));
        }
    }
    if let Some(extensions) = document.get("extensions").and_then(KdlNode::children) {
        for node in extensions.nodes().iter().filter(|node| node.name().value() == "draft") {
            let id = first_string(node).ok_or_else(|| "internal draft extension missing id".to_owned())?;
            let version = required_string(node, "version")?;
            result.draft_extensions.insert(id.to_owned(), version);
        }
    }
    Ok(result)
}

fn collect_ports(
    root: Option<&KdlNode>,
    note: bool,
    output: &mut BTreeMap<String, Port>,
) -> Result<(), String> {
    let Some(children) = root.and_then(KdlNode::children) else {
        return Ok(());
    };
    for node in children
        .nodes()
        .iter()
        .filter(|node| matches!(node.name().value(), "input" | "output"))
    {
        let id = required_string(node, "id")?;
        output.insert(
            id,
            Port {
                direction: node.name().value().to_owned(),
                channels: (!note).then(|| value_prop(node, "channels")).transpose()?,
                dialects: note.then(|| string_prop(node, "dialects").unwrap_or_default().to_owned()),
                preferred: note.then(|| string_prop(node, "preferred").unwrap_or_default().to_owned()),
            },
        );
    }
    Ok(())
}

fn prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlValue> {
    node.entries().iter().rev().find_map(|entry| {
        let name = entry.name()?;
        (name.value() == key).then_some(entry.value())
    })
}

fn required_string(node: &KdlNode, key: &str) -> Result<String, String> {
    string_prop(node, key)
        .map(str::to_owned)
        .ok_or_else(|| format!("internal IR node `{}` missing `{key}`", node.name().value()))
}

fn string_prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a str> {
    match prop(node, key)? {
        KdlValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn value_prop(node: &KdlNode, key: &str) -> Result<String, String> {
    match prop(node, key) {
        Some(KdlValue::Integer(value)) => Ok(value.to_string()),
        Some(KdlValue::Float(value)) => Ok(value.to_string()),
        Some(value) => Err(format!("internal IR `{key}` has unexpected value `{value:?}`")),
        None => Err(format!("internal IR node `{}` missing `{key}`", node.name().value())),
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

fn json_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ir::build_ir;
    use crate::metadata::parse_metadata;

    use super::{Class, compare};

    fn ir(parameter: &str, audio: &str, state: &str, extensions: &str) -> crate::ir::CanonicalIr {
        let source = format!(
            "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.synth\" name=\"Synth\" vendor=\"Example\" version=\"1.0\"\nprocessor class=\"P\"\nparameters {{ {parameter} }}\naudio-ports {{ {audio} }}\nnote-ports {{}}\nstate {{ {state} }}\ngui {{ api \"web\" }}\npresets {{}}\nfactories {{}}\nextensions {{ {extensions} }}\n"
        );
        let path = Path::new("plugin.kdl");
        let metadata = parse_metadata(path, &source).expect("metadata");
        build_ir(path, &source, &metadata).expect("IR")
    }

    #[test]
    fn classifies_parameter_changes_deterministically() {
        let baseline = ir(
            "param \"Gain\" id=\"gain\" min=0.0 max=1.0 default=0.5 flags=\"automatable\"",
            "",
            "",
            "",
        );
        let current = ir(
            "param \"Gain\" id=\"gain\" min=0.0 max=2.0 default=0.75 flags=\"automatable,modulatable\"; param \"Mix\" id=\"mix\" min=0.0 max=1.0 default=1.0",
            "",
            "",
            "",
        );
        let report = compare(&baseline, &current).expect("compare");
        let text = report.text();
        assert!(text.contains("sensitive parameter.gain.range"), "{text}");
        assert!(text.contains("sensitive parameter.gain.default"), "{text}");
        assert!(text.contains("sensitive parameter.gain.flags"), "{text}");
        assert!(text.contains("compatible parameter.mix"), "{text}");
        assert!(!report.has_forbidden());
        assert_eq!(report.json(), report.json(), "JSON must be deterministic");
    }

    #[test]
    fn released_port_state_and_draft_abi_breaks_are_forbidden() {
        let baseline = ir(
            "",
            "output \"Main\" id=\"out\" channels=2",
            "field \"preset\" type=\"string\" tag=\"preset-v1\"",
            "enable \"clap.webview/3\" version=\"3\" draft=#true",
        );
        let current = ir("", "", "field \"preset\" type=\"string\" tag=\"preset-v2\"", "");
        let report = compare(&baseline, &current).expect("compare");
        assert!(report.has_forbidden());
        assert!(report.text().contains("forbidden audio-port.out"));
        assert!(report.text().contains("forbidden state.preset.tag"));
        assert!(report.text().contains("forbidden draft.clap.webview/3"));
    }

    #[test]
    fn plugin_id_change_is_forbidden() {
        let baseline = ir("", "", "", "");
        let source = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.other\" name=\"Synth\" vendor=\"Example\" version=\"1.0\"\nprocessor class=\"P\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";
        let path = Path::new("other.kdl");
        let metadata = parse_metadata(path, source).expect("metadata");
        let current = build_ir(path, source, &metadata).expect("IR");
        let report = compare(&baseline, &current).expect("compare");
        assert!(report.changes.iter().any(|change| change.class == Class::Forbidden));
        assert!(report.text().contains("forbidden plugin.id"));
    }
}