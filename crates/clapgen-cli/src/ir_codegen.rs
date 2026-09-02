#![allow(dead_code)]

use super::CanonicalIr;

pub(crate) const OUTPUT_NAMES: &[&str] = &[
    "clapgen.d",
    "clapgen.manifest.kdl",
    "clapgen.sources.kdl",
    "clapgen_metadata.cpp",
    "clapgen_metadata.hpp",
    "clapgen_resources.hpp",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedFile {
    pub(crate) path: &'static str,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationPlan {
    pub(crate) files: Vec<GeneratedFile>,
}

pub(crate) fn render(_ir: &CanonicalIr) -> GenerationPlan {
    let files = OUTPUT_NAMES
        .iter()
        .copied()
        .map(|path| GeneratedFile { path, bytes: Vec::new() })
        .collect();
    GenerationPlan { files }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ir::build_ir;
    use crate::metadata::parse_metadata;

    use super::{render, OUTPUT_NAMES};

    const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.codegen\" name=\"Codegen\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CodegenProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

    fn ir() -> crate::ir::CanonicalIr {
        let path = Path::new("plugin.kdl");
        let metadata = parse_metadata(path, SOURCE).expect("metadata should parse");
        build_ir(path, SOURCE, &metadata).expect("canonical IR should build")
    }

    #[test]
    fn fixed_output_contract_is_stable() {
        assert_eq!(
            OUTPUT_NAMES,
            [
                "clapgen.d",
                "clapgen.manifest.kdl",
                "clapgen.sources.kdl",
                "clapgen_metadata.cpp",
                "clapgen_metadata.hpp",
                "clapgen_resources.hpp",
            ]
        );
        assert!(OUTPUT_NAMES.iter().all(|name| !name.contains("entry")));
        assert!(OUTPUT_NAMES.iter().all(|name| !name.contains("factory")));
        assert!(OUTPUT_NAMES.iter().all(|name| !name.contains("plugin.cpp")));
    }

    #[test]
    fn canonical_ir_renders_to_a_pure_deterministic_plan() {
        let ir = ir();
        let first = render(&ir);
        let second = render(&ir);

        assert_eq!(first, second);
        assert_eq!(
            first.files.iter().map(|file| file.path).collect::<Vec<_>>(),
            OUTPUT_NAMES
        );
        assert!(first.files.iter().all(|file| file.bytes.is_empty()));
    }
}
