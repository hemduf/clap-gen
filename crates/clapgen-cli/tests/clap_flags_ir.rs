use std::env;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn current_clap_parameter_and_audio_flags_are_symbolic_ir_inputs() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let directory = env::temp_dir().join(format!("clapgen-flags-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&directory).expect("temporary directory should be created");
    let manifest = directory.join("plugin.kdl");
    fs::write(
        &manifest,
        r#"clapgen schema="1.0.0"
plugin id="com.example.flags" name="Flags" vendor="Example" version="1.0.0"
processor class="FlagsProcessor"
parameters {
    param "all" id="all" min=0.0 max=1.0 default=0.0 flags="stepped,periodic,hidden,readonly,bypass,automatable,automatable-per-note-id,automatable-per-key,automatable-per-channel,automatable-per-port,modulatable,modulatable-per-note-id,modulatable-per-key,modulatable-per-channel,modulatable-per-port,requires-process,enum"
}
audio-ports {
    input "main" id="in" channels=2 flags="main,supports-64bits,prefers-64bits,requires-common-sample-size"
    output "main" id="out" channels=2 flags="main,supports-64bits,prefers-64bits,requires-common-sample-size"
}
note-ports {}
state {}
gui {}
presets {}
factories {}
extensions {}
"#,
    )
    .expect("manifest should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .arg("validate")
        .arg(&manifest)
        .output()
        .expect("clapgen should run");

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fs::remove_dir_all(directory).expect("temporary directory should be removed");
}
