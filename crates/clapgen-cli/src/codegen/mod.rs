#![allow(dead_code)]

mod outputs;
mod render;

pub(crate) use outputs::{GeneratedFile, GenerationPlan, OUTPUT_NAMES};

pub(crate) fn render(ir: &crate::ir::CanonicalIr) -> GenerationPlan {
    render::render(ir)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::ir::build_ir;
    use crate::metadata::parse_metadata;

    use super::{OUTPUT_NAMES, render};

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
            &[
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
        let paths = first.files.iter().map(|file| file.path).collect::<Vec<_>>();

        assert_eq!(first, second);
        assert_eq!(paths.as_slice(), OUTPUT_NAMES);
    }
}
