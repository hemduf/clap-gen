use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "clapgen-issue10-event-capabilities-{}-{nonce}",
        std::process::id()
    ))
}

fn generate(metadata: &Path, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["generate", "--metadata"])
        .arg(metadata)
        .arg("--out")
        .arg(out)
        .output()
        .expect("clapgen generate should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn runtime_rejects_value_and_modulation_events_without_declared_capabilities() {
    let root = temporary_directory();
    fs::create_dir_all(&root).expect("temporary directory");

    let manifest = root.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.issue10-capabilities\" name=\"Issue10 Capabilities\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"Issue10CapabilitiesProcessor\"\nparameters {\n    param id=\"meter\" name=\"Meter\" min=0 max=1 default=0 flags=\"readonly\"\n    param id=\"gain\" name=\"Gain\" min=0 max=2 default=1 flags=\"automatable\"\n}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions { enable \"clap.params\" }\n",
    )
    .expect("manifest");
    fs::write(
        root.join("plugin.ids.kdl"),
        "ids version=1 next=3 {\n    entry kind=\"parameter\" key=\"meter\" value=1 tombstone=#false\n    entry kind=\"parameter\" key=\"gain\" value=2 tombstone=#false\n}\n",
    )
    .expect("registry");

    let out = root.join("generated");
    generate(&manifest, &out);

    let backend = fs::read_to_string(out.join("clapgen_instance_backend.hpp"))
        .expect("generated instance backend");

    let value_case = backend
        .find("case CLAP_EVENT_PARAM_VALUE")
        .expect("value routing case");
    let value_tail = &backend[value_case..];
    let readonly_guard = value_tail
        .find("(spec.flags & CLAP_PARAM_IS_READONLY) != 0u")
        .expect("readonly value-event guard");
    let value_snapshot = value_tail
        .find("parameter_values_[static_cast<std::size_t>(parameter_index)]")
        .expect("global value snapshot update");
    let value_delivery = value_tail
        .find("deliver_parameter_event(header)")
        .expect("value event delivery");
    assert!(
        readonly_guard < value_snapshot && readonly_guard < value_delivery,
        "readonly host value events must be rejected before snapshot mutation or processor delivery"
    );

    let mod_case = backend
        .find("case CLAP_EVENT_PARAM_MOD")
        .expect("modulation routing case");
    let mod_tail = &backend[mod_case..];
    let modulatable_guard = mod_tail
        .find("(spec.flags & CLAP_PARAM_IS_MODULATABLE) == 0u")
        .expect("modulatable capability guard");
    let mod_delivery = mod_tail
        .find("deliver_parameter_event(header)")
        .expect("modulation event delivery");
    assert!(
        modulatable_guard < mod_delivery,
        "modulation events must be rejected before processor delivery when the parameter is not modulatable"
    );

    fs::remove_dir_all(root).expect("temporary directory cleanup");
}
