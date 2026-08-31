use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use kdl::{KdlNode, KdlValue};

use crate::metadata::{ParsedMetadata, parse_metadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceEntry {
    pub(crate) key: String,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct SourceDocument {
    pub(crate) display_path: String,
    pub(crate) source: String,
    pub(crate) metadata: ParsedMetadata,
}

#[derive(Debug, Clone)]
pub(crate) struct SourceBundle {
    pub(crate) documents: Vec<SourceDocument>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) sources: Vec<SourceEntry>,
}

pub(crate) fn collect(
    root_path: &Path,
    root_source: &str,
    root_metadata: &ParsedMetadata,
) -> Result<SourceBundle, String> {
    let root_display = root_path.file_name().map_or_else(
        || normalize_display_path(root_path),
        |value| value.to_string_lossy().into_owned(),
    );
    let root_document = SourceDocument {
        display_path: root_display.clone(),
        source: root_source.to_owned(),
        metadata: root_metadata.clone(),
    };

    let mut imports = Vec::new();
    let mut loaded = BTreeSet::new();
    let mut stack = BTreeSet::new();
    if let Ok(root) = fs::canonicalize(root_path) {
        stack.insert(root);
    }
    load_imports(root_path, &root_display, root_metadata, &mut imports, &mut loaded, &mut stack)?;

    imports.sort_by(|a, b| a.display_path.cmp(&b.display_path));

    let mut documents = Vec::with_capacity(imports.len() + 1);
    documents.push(root_document);
    documents.extend(imports);

    let mut dependencies = vec![root_display];
    dependencies.extend(documents.iter().skip(1).map(|document| document.display_path.clone()));

    let mut sources = Vec::new();
    for document in &documents {
        collect_document_sources(document, &mut sources);
    }
    sources.sort_by(|a, b| {
        (&a.key, &a.path, a.line, a.column).cmp(&(&b.key, &b.path, b.line, b.column))
    });

    Ok(SourceBundle { documents, dependencies, sources })
}

fn load_imports(
    owner_path: &Path,
    owner_display: &str,
    metadata: &ParsedMetadata,
    loaded_documents: &mut Vec<SourceDocument>,
    loaded: &mut BTreeSet<PathBuf>,
    stack: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    for node in metadata.document.nodes().iter().filter(|node| node.name().value() == "import") {
        let Some(relative) = first_string(node) else {
            continue;
        };
        let optional = bool_prop(node, "optional").unwrap_or(false);
        let candidate = owner_path.parent().unwrap_or_else(|| Path::new(".")).join(relative);
        let canonical = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(_) if optional => continue,
            Err(error) => {
                return Err(format!(
                    "{}:1: failed to resolve imported metadata `{}`: {error}\nhint: fix the import path or mark it optional",
                    owner_path.display(),
                    candidate.display()
                ));
            }
        };
        if stack.contains(&canonical) {
            return Err(format!(
                "{}:1: semantic import cycle reaches `{}`\nhint: remove the cyclic import",
                owner_path.display(),
                canonical.display()
            ));
        }
        if loaded.contains(&canonical) {
            continue;
        }

        let source = fs::read_to_string(&canonical).map_err(|error| {
            format!(
                "{}:1: failed to read imported metadata `{}`: {error}",
                owner_path.display(),
                canonical.display()
            )
        })?;
        let parsed = parse_metadata(&canonical, &source)?;
        if parsed.document.get("plugin").is_some() || parsed.document.get("processor").is_some() {
            return Err(format!(
                "{}:1: imported metadata may not redefine `plugin` or `processor`\nhint: keep descriptors in the root manifest and import semantic fragments only",
                canonical.display()
            ));
        }

        let display = join_display_path(owner_display, relative);
        loaded.insert(canonical.clone());
        stack.insert(canonical.clone());
        load_imports(&canonical, &display, &parsed, loaded_documents, loaded, stack)?;
        stack.remove(&canonical);

        loaded_documents.push(SourceDocument { display_path: display, source, metadata: parsed });
    }
    Ok(())
}

fn collect_document_sources(document: &SourceDocument, out: &mut Vec<SourceEntry>) {
    for node in document.metadata.document.nodes() {
        match node.name().value() {
            "plugin" => push_source(out, document, node, "plugin".to_owned()),
            "processor" => push_source(out, document, node, "processor".to_owned()),
            "import" => {
                if let Some(path) = first_string(node) {
                    push_source(out, document, node, format!("import:{}", normalize_path(path)));
                }
            }
            "parameters" => collect_section(document, node, "parameters", out),
            "audio-ports" => collect_section(document, node, "audio-ports", out),
            "note-ports" => collect_section(document, node, "note-ports", out),
            "state" => collect_section(document, node, "state", out),
            "gui" => collect_section(document, node, "gui", out),
            "presets" => collect_section(document, node, "presets", out),
            "factories" => collect_section(document, node, "factories", out),
            "extensions" => collect_section(document, node, "extensions", out),
            _ => {}
        }
    }
}

fn collect_section(
    document: &SourceDocument,
    root: &KdlNode,
    section: &str,
    out: &mut Vec<SourceEntry>,
) {
    let Some(children) = root.children() else {
        return;
    };
    for node in children.nodes() {
        if let Some(key) = semantic_key(section, node) {
            push_source(out, document, node, key);
        }
    }
}

fn semantic_key(section: &str, node: &KdlNode) -> Option<String> {
    match (section, node.name().value()) {
        ("parameters", "param") => string_prop(node, "id").map(|id| format!("parameter:{id}")),
        ("audio-ports", "input" | "output") => {
            string_prop(node, "id").map(|id| format!("audio-port:{}:{id}", node.name().value()))
        }
        ("note-ports", "input" | "output") => {
            string_prop(node, "id").map(|id| format!("note-port:{}:{id}", node.name().value()))
        }
        ("note-ports", "note-name") => first_string(node).map(|name| {
            let key =
                integer_prop(node, "key").map_or_else(|| "*".to_owned(), |value| value.to_string());
            let channel = integer_prop(node, "channel")
                .map_or_else(|| "*".to_owned(), |value| value.to_string());
            let port = string_prop(node, "port").unwrap_or("*");
            format!("note-name:{name}:{key}:{channel}:{port}")
        }),
        ("state", "field") => first_string(node)
            .or_else(|| string_prop(node, "name"))
            .map(|name| format!("state-field:{name}")),
        ("gui", "api") => first_string(node)
            .or_else(|| string_prop(node, "name"))
            .map(|name| format!("gui-api:{name}")),
        ("gui", "resource") => first_string(node)
            .or_else(|| string_prop(node, "path"))
            .map(|path| format!("resource:{}", normalize_path(path))),
        ("presets", "location") => first_string(node).map(|name| format!("preset-location:{name}")),
        ("presets", "format") => first_string(node).map(|name| format!("preset-format:{name}")),
        ("factories", "factory") => {
            first_string(node).or_else(|| string_prop(node, "id")).map(|id| format!("factory:{id}"))
        }
        ("extensions", "enable") => first_string(node)
            .or_else(|| string_prop(node, "id"))
            .map(|id| format!("extension:{id}")),
        _ => None,
    }
}

fn push_source(out: &mut Vec<SourceEntry>, document: &SourceDocument, node: &KdlNode, key: String) {
    let (line, column) = line_column(&document.source, node);
    out.push(SourceEntry { key, path: document.display_path.clone(), line, column });
}

fn line_column(source: &str, node: &KdlNode) -> (usize, usize) {
    let offset = node.span().offset().min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit_once('\n').map_or(prefix, |(_, tail)| tail).chars().count() + 1;
    (line, column)
}

fn join_display_path(owner_display: &str, relative: &str) -> String {
    let owner = Path::new(owner_display);
    let parent = owner.parent().unwrap_or_else(|| Path::new(""));
    normalize_display_path(&parent.join(relative))
}

fn normalize_display_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|part| part != "..") {
                    parts.pop();
                } else {
                    parts.push("..".to_owned());
                }
            }
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::RootDir => parts.clear(),
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().into_owned());
            }
        }
    }
    parts.join("/")
}

fn normalize_path(value: &str) -> String {
    normalize_display_path(Path::new(&value.replace('\\', "/")))
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
