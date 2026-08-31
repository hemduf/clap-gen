#[path = "reviewed/mod.rs"]
mod reviewed;

pub(crate) use reviewed::{
    CanonicalIr, ExtensionSet, build_ir, capability_report_kdl, serialize_ir_kdl,
};
