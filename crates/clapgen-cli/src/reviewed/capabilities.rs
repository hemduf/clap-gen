use kdl::{KdlNode, KdlValue};

use super::provenance::SourceDocument;

const CLAP_SDK_PIN: &str = "a47f6badb49d948fd009998f28309cdab78979c9";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stability {
    Stable,
    Draft,
}

// Stable, unversioned extension IDs exposed by the pinned CLAP SDK. These are not part of the
// version-policy validation below, but codegen needs their exact headers to emit minimal includes.
const STABLE_UNVERSIONED: &[(&str, &str)] = &[
    ("clap.audio-ports", "clap/ext/audio-ports.h"),
    ("clap.audio-ports-config", "clap/ext/audio-ports-config.h"),
    ("clap.configurable-audio-ports.draft1", "clap/ext/configurable-audio-ports.h"),
    ("clap.event-registry", "clap/ext/event-registry.h"),
    ("clap.gui", "clap/ext/gui.h"),
    ("clap.latency", "clap/ext/latency.h"),
    ("clap.log", "clap/ext/log.h"),
    ("clap.note-name", "clap/ext/note-name.h"),
    ("clap.note-ports", "clap/ext/note-ports.h"),
    ("clap.params", "clap/ext/params.h"),
    ("clap.posix-fd-support", "clap/ext/posix-fd-support.h"),
    ("clap.render", "clap/ext/render.h"),
    ("clap.state", "clap/ext/state.h"),
    ("clap.tail", "clap/ext/tail.h"),
    ("clap.thread-check", "clap/ext/thread-check.h"),
    ("clap.thread-pool", "clap/ext/thread-pool.h"),
    ("clap.timer-support", "clap/ext/timer-support.h"),
    ("clap.voice-info", "clap/ext/voice-info.h"),
];

const STABLE_VERSIONED: &[(&str, &str)] = &[
    ("clap.ambisonic/3", "clap/ext/ambisonic.h"),
    ("clap.ambisonic.draft/3", "clap/ext/ambisonic.h"),
    ("clap.audio-ports-activation/2", "clap/ext/audio-ports-activation.h"),
    ("clap.audio-ports-activation/draft-2", "clap/ext/audio-ports-activation.h"),
    ("clap.audio-ports-config-info/1", "clap/ext/audio-ports-config.h"),
    ("clap.audio-ports-config-info/draft-0", "clap/ext/audio-ports-config.h"),
    ("clap.configurable-audio-ports/1", "clap/ext/configurable-audio-ports.h"),
    ("clap.context-menu/1", "clap/ext/context-menu.h"),
    ("clap.context-menu.draft/0", "clap/ext/context-menu.h"),
    ("clap.param-indication/4", "clap/ext/param-indication.h"),
    ("clap.param-indication.draft/4", "clap/ext/param-indication.h"),
    ("clap.preset-load/2", "clap/ext/preset-load.h"),
    ("clap.remote-controls/2", "clap/ext/remote-controls.h"),
    ("clap.remote-controls.draft/2", "clap/ext/remote-controls.h"),
    ("clap.state-context/2", "clap/ext/state-context.h"),
    ("clap.surround/4", "clap/ext/surround.h"),
    ("clap.surround.draft/4", "clap/ext/surround.h"),
    ("clap.track-info/1", "clap/ext/track-info.h"),
    ("clap.track-info.draft/1", "clap/ext/track-info.h"),
];

const DRAFT_VERSIONED: &[(&str, &str)] = &[
    ("clap.background-activation/1", "clap/ext/draft/background-activation.h"),
    ("clap.background-progress/1", "clap/ext/draft/background-progress.h"),
    ("clap.background-state-context/1", "clap/ext/draft/background-state-context.h"),
    ("clap.extensible-audio-ports/1", "clap/ext/draft/extensible-audio-ports.h"),
    ("clap.flush-events/1", "clap/ext/draft/flush-events.h"),
    ("clap.gain-adjustment-metering/0", "clap/ext/draft/gain-adjustment-metering.h"),
    ("clap.mini-curve-display/3", "clap/ext/draft/mini-curve-display.h"),
    ("clap.octave-number/1", "clap/ext/draft/octave-number.h"),
    ("clap.param-hovered/1", "clap/ext/draft/param-hovered.h"),
    ("clap.params-origin/1", "clap/ext/draft/params-origin.h"),
    ("clap.preset-load.draft/2", "clap/ext/preset-load.h"),
    ("clap.project-location/2", "clap/ext/draft/project-location.h"),
    ("clap.resource-directory/1", "clap/ext/draft/resource-directory.h"),
    ("clap.scratch-memory/1", "clap/ext/draft/scratch-memory.h"),
    ("clap.transport-control/2", "clap/ext/draft/transport-control.h"),
    ("clap.triggers/1", "clap/ext/draft/triggers.h"),
    ("clap.tuning/2", "clap/ext/draft/tuning.h"),
    ("clap.undo/4", "clap/ext/draft/undo.h"),
    ("clap.undo_context/4", "clap/ext/draft/undo.h"),
    ("clap.undo_delta/4", "clap/ext/draft/undo.h"),
    ("clap.webview/3", "clap/ext/draft/webview.h"),
];

pub(crate) fn validate(documents: &[SourceDocument]) -> Result<(), String> {
    for document in documents {
        for root in document
            .metadata
            .document
            .nodes()
            .iter()
            .filter(|node| node.name().value() == "extensions")
        {
            let Some(children) = root.children() else {
                continue;
            };
            for node in children.nodes().iter().filter(|node| node.name().value() == "enable") {
                validate_extension(document, node)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn header_for(id: &str) -> Option<&'static str> {
    let id = id.trim();
    STABLE_UNVERSIONED
        .iter()
        .chain(STABLE_VERSIONED)
        .chain(DRAFT_VERSIONED)
        .find_map(|(candidate, header)| (*candidate == id).then_some(*header))
}

fn validate_extension(document: &SourceDocument, node: &KdlNode) -> Result<(), String> {
    let Some(id) = first_string(node).or_else(|| string_prop(node, "id")) else {
        return Ok(());
    };
    if !id.starts_with("clap.") || !id.contains('/') {
        return Ok(());
    }

    let declared_draft = bool_prop(node, "draft").unwrap_or(false);
    let known = lookup(id);
    let Some((stability, header)) = known else {
        return Err(diagnostic(
            document,
            node,
            &format!(
                "official versioned extension `{id}` is not present in pinned CLAP SDK `{CLAP_SDK_PIN}`"
            ),
            "use an exact official extension ID from the pinned SDK or a reverse-URI third-party ID",
        ));
    };

    match (stability, declared_draft) {
        (Stability::Stable, true) => {
            return Err(diagnostic(
                document,
                node,
                &format!("stable extension `{id}` must not be declared as draft"),
                &format!("the pinned SDK exposes this capability from `{header}`"),
            ));
        }
        (Stability::Draft, false) => {
            return Err(diagnostic(
                document,
                node,
                &format!("draft extension `{id}` requires explicit draft opt-in"),
                "add `draft=#true` and the exact matching `version` pin",
            ));
        }
        _ => {}
    }

    if stability == Stability::Draft
        && let Some(version) = string_prop(node, "version")
        && id.rsplit_once('/').is_some_and(|(_, abi)| abi != version)
    {
        return Err(diagnostic(
            document,
            node,
            &format!("draft extension `{id}` has mismatched ABI version pin `{version}`"),
            "make `version` match the extension ID revision",
        ));
    }

    Ok(())
}

fn lookup(id: &str) -> Option<(Stability, &'static str)> {
    if let Some((_, header)) = STABLE_VERSIONED.iter().find(|(candidate, _)| *candidate == id) {
        return Some((Stability::Stable, *header));
    }
    DRAFT_VERSIONED
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, header)| (Stability::Draft, *header))
}

fn diagnostic(document: &SourceDocument, node: &KdlNode, message: &str, hint: &str) -> String {
    let offset = node.span().offset().min(document.source.len());
    let line = document.source[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1;
    format!("{}:{line}: {message}\nhint: {hint}", document.display_path)
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

fn bool_prop(node: &KdlNode, key: &str) -> Option<bool> {
    match prop(node, key)? {
        KdlValue::Bool(value) => Some(*value),
        _ => None,
    }
}
