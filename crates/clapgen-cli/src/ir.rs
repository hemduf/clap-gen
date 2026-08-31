#[path = "reviewed/mod.rs"]
mod reviewed;

pub(crate) use reviewed::{CanonicalIr, build_ir, capability_report_kdl, serialize_ir_kdl};

#[cfg(test)]
#[path = "reviewed/contracts.rs"]
mod contracts;
