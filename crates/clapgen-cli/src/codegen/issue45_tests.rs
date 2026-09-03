use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ids::allocate;
use crate::ir::build_ir;
use crate::metadata::parse_metadata;

use super::{GenerationPlan, render};

const SOURCE: &str = "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.native-contract\" name=\"Native Contract\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"NativeProcessor\"\nparameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 }\naudio-ports { input \"Main In\" id=\"main-in\" channels=2; output \"Main Out\" id=\"main-out\" channels=2 }\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n";

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn build_file(path: &Path) -> crate::ir::CanonicalIr {
    let source = fs::read_to_string(path).expect("metadata should be readable");
    let metadata = parse_metadata(path, &source).expect("metadata should parse");
    build_ir(path, &source, &metadata).expect("canonical IR should build")
}

fn generated_text<'a>(plan: &'a GenerationPlan, path: &str) -> &'a str {
    let file = plan.files.iter().find(|file| file.path == path).expect("generated file");
    std::str::from_utf8(&file.bytes).expect("generated files must be UTF-8")
}

#[test]
fn generated_processor_contract_uses_only_native_clap_types() {
    let directory = temporary_directory("issue45-native-contract");
    fs::create_dir_all(&directory).expect("temporary directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(&manifest, SOURCE).expect("metadata should be writable");

    let plan = render(&build_file(&manifest));
    let contract = generated_text(&plan, "clapgen_processor.hpp");

    for required in [
        "#include <clap/clap.h>",
        "concept NativeProcessor",
        "processor.init()",
        "processor.activate(sample_rate, min_frames, max_frames)",
        "processor.deactivate()",
        "processor.start_processing()",
        "processor.stop_processing()",
        "processor.reset()",
        "processor.process(process)",
        "std::same_as<clap_process_status>",
    ] {
        assert!(contract.contains(required), "missing `{required}`:\n{contract}");
    }

    for forbidden in [
        "ProcessBlock",
        "ProcessStatus",
        "ActivateContext",
        "PluginId",
        "EventWrapper",
        "HostWrapper",
    ] {
        assert!(!contract.contains(forbidden), "unexpected mirror type `{forbidden}`:\n{contract}");
    }

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}

#[test]
fn generated_ids_are_inline_constexpr_clap_ids_from_the_registry() {
    let directory = temporary_directory("issue45-generated-ids");
    fs::create_dir_all(&directory).expect("temporary directory");
    let manifest = directory.join("plugin.kdl");
    fs::write(&manifest, SOURCE).expect("metadata should be writable");
    let registry = directory.join("plugin.ids.kdl");

    assert_eq!(1, allocate(&registry, "parameter", "gain").expect("parameter id"));
    assert_eq!(2, allocate(&registry, "audio-port", "main-in").expect("input id"));
    assert_eq!(3, allocate(&registry, "audio-port", "main-out").expect("output id"));

    let plan = render(&build_file(&manifest));
    let ids = generated_text(&plan, "clapgen_ids.hpp");

    assert!(ids.contains("#include <clap/clap.h>"), "{ids}");
    assert!(ids.contains("namespace kind_parameter"), "{ids}");
    assert!(ids.contains("inline constexpr clap_id id_gain = 1u;"), "{ids}");
    assert!(ids.contains("namespace kind_audio_2Dport"), "{ids}");
    assert!(ids.contains("inline constexpr clap_id id_main_2Din = 2u;"), "{ids}");
    assert!(ids.contains("inline constexpr clap_id id_main_2Dout = 3u;"), "{ids}");
    assert!(!ids.contains("tombstone"), "{ids}");

    let second = render(&build_file(&manifest));
    assert_eq!(plan, second, "ID generation must be deterministic");

    fs::remove_dir_all(directory).expect("temporary directory should be removable");
}
