use std::path::Path;

use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, OUTPUT_NAMES, render};

const REPRESENTATIVE_SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue54\" name=\"Issue54\" vendor=\"Example\" version=\"1.0.0\" features=\"instrument,synthesizer\"\nprocessor class=\"Issue54Processor\"\nparameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 flags=\"automatable\"; param \"Tone\" id=\"tone\" min=20 max=20000 default=1000 flags=\"automatable\" }\naudio-ports { input \"Input\" id=\"input\" channels=2 flags=\"main\"; output \"Output\" id=\"output\" channels=2 flags=\"main\" }\nnote-ports { input \"Notes\" id=\"notes\" dialects=\"clap,midi\" preferred=\"clap\" }\nstate { field \"mode\" type=\"integer\" default=0 tag=\"mode\" }\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.params\" }\n";

fn ir_at(path: &Path) -> crate::ir::CanonicalIr {
    let metadata = parse_metadata(path, REPRESENTATIVE_SOURCE).expect("metadata should parse");
    build_ir(path, REPRESENTATIVE_SOURCE, &metadata).expect("canonical IR should build")
}

fn generated_code(plan: &GenerationPlan) -> impl Iterator<Item = (&'static str, &str)> {
    plan.files
        .iter()
        .filter(|file| {
            Path::new(file.path).extension().and_then(|extension| extension.to_str()).is_some_and(
                |extension| {
                    extension.eq_ignore_ascii_case("cpp") || extension.eq_ignore_ascii_case("hpp")
                },
            )
        })
        .map(|file| {
            (file.path, std::str::from_utf8(&file.bytes).expect("generated C++ must be UTF-8"))
        })
}

#[test]
fn issue54_representative_runtime_generation_is_byte_identical() {
    let ir = ir_at(Path::new("project/plugin.kdl"));
    let first = render(&ir);
    let second = render(&ir);

    assert_eq!(first, second, "the same canonical IR must render byte-identically");
    assert_eq!(
        first.files.iter().map(|file| file.path).collect::<Vec<_>>(),
        OUTPUT_NAMES,
        "deterministic generation also requires fixed output ordering"
    );
}

#[test]
fn issue54_generated_cpp_is_independent_of_machine_root_and_wall_clock_data() {
    let first = render(&ir_at(Path::new("host-a/work/clap-gen/plugin.kdl")));
    let second = render(&ir_at(Path::new("host-b/other/clap-gen/plugin.kdl")));

    let first_code = generated_code(&first).collect::<Vec<_>>();
    let second_code = generated_code(&second).collect::<Vec<_>>();
    assert_eq!(first_code, second_code, "generated C++ must not depend on source checkout root");

    for (path, source) in first_code {
        for forbidden in [
            "host-a/",
            "host-b/",
            "Generated at",
            "generated_at",
            "timestamp",
            "__DATE__",
            "__TIME__",
            "std::chrono",
            "SystemTime",
            "random_device",
            "uuid",
        ] {
            assert!(!source.contains(forbidden), "`{forbidden}` leaked into {path}:\n{source}");
        }
    }
}

#[test]
fn issue54_generated_clap_callback_tables_require_no_unsafe_casts() {
    let plan = render(&ir_at(Path::new("project/plugin.kdl")));
    for (path, source) in generated_code(&plan) {
        for forbidden in [
            "reinterpret_cast<",
            "reinterpret_cast (",
            "reinterpret_cast(",
            "(clap_plugin_t::",
            "(clap_plugin_entry_t::",
            "(clap_plugin_factory_t::",
        ] {
            assert!(
                !source.contains(forbidden),
                "unsafe callback cast `{forbidden}` in {path}:\n{source}"
            );
        }
    }
}

#[test]
fn issue54_native_abi_contract_is_a_strict_cross_platform_compile_gate() {
    let cmake = include_str!("../../../../CMakeLists.txt");
    assert!(
        cmake.contains("tests/codegen/issue54/Issue54.cmake"),
        "issue54 ABI compile gate must be registered:\n{cmake}"
    );

    let ci = include_str!("../../../../.github/workflows/ci.yml");
    assert!(
        ci.contains("tests/codegen/issue54/native_abi_contract.cpp"),
        "issue54 ABI contract must participate in C++ formatting/static-analysis CI:\n{ci}"
    );
}
