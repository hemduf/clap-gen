use kdl::{KdlDocument, KdlNode, KdlValue};

const SCHEMA: &str = include_str!("../../../schemas/clapgen-1.0.0.kdl");

#[test]
fn published_schema_uses_kdl_schema_validation_nodes() {
    let document = KdlDocument::parse_v2(SCHEMA).expect("schema must parse as KDL 2.0");
    let root = document.get("document").expect("schema requires a document root");

    visit(root, &mut |node| {
        if node.name().value() != "prop" && node.name().value() != "value" {
            return;
        }

        for entry in node.entries() {
            let Some(name) = entry.name() else {
                continue;
            };
            assert_ne!(
                "type",
                name.value(),
                "KDL Schema 1.0 expresses `type` as a child validation node"
            );
            assert_ne!(
                "required",
                name.value(),
                "KDL Schema 1.0 expresses `required` as a child node"
            );
        }
    });

    let schema_prop = find_prop(root, "schema").expect("clapgen.schema must be described");
    let children = schema_prop
        .children()
        .expect("clapgen.schema must have validation children");
    assert!(children.get("type").is_some());
    assert!(children.get("required").is_some());
}

fn visit(node: &KdlNode, visitor: &mut impl FnMut(&KdlNode)) {
    visitor(node);
    if let Some(children) = node.children() {
        for child in children.nodes() {
            visit(child, visitor);
        }
    }
}

fn find_prop<'a>(node: &'a KdlNode, key: &str) -> Option<&'a KdlNode> {
    if node.name().value() == "prop" && first_string_argument(node) == Some(key) {
        return Some(node);
    }

    node.children()?
        .nodes()
        .iter()
        .find_map(|child| find_prop(child, key))
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
