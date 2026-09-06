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
        "clapgen-issue11-static-ports-{}-{nonce}",
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

fn generated_sources(out: &Path) -> String {
    let mut paths = fs::read_dir(out)
        .expect("generated output directory")
        .map(|entry| entry.expect("generated entry").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("hpp" | "cpp")
            )
        })
        .collect::<Vec<_>>();
    paths.sort();

    let mut output = String::new();
    for path in paths {
        output.push_str(&fs::read_to_string(path).expect("generated source should be UTF-8"));
        output.push('\n');
    }
    output
}

#[test]
fn generates_native_static_audio_note_and_note_name_extensions_from_immutable_port_ids() {
    let root = temporary_directory();
    fs::create_dir_all(&root).expect("temporary directory");

    let manifest = root.join("plugin.kdl");
    fs::write(
        &manifest,
        concat!(
            "clapgen schema=\"1.0.0\"\n",
            "plugin id=\"com.example.issue11\" name=\"Issue11\" vendor=\"Example\" version=\"1.0.0\"\n",
            "processor class=\"Issue11Processor\"\n",
            "parameters {}\n",
            "audio-ports {\n",
            "    input \"Main In\" id=\"main-in\" channels=2 type=\"stereo\" flags=\"main\" in-place-pair=\"main-out\"\n",
            "    input \"Sidechain\" id=\"sidechain\" channels=1\n",
            "    output \"Main Out\" id=\"main-out\" channels=2 type=\"stereo\" flags=\"main\" in-place-pair=\"main-in\"\n",
            "}\n",
            "note-ports {\n",
            "    input \"Notes In\" id=\"notes-in\" dialects=\"clap,midi1,midi2\" preferred=\"clap\"\n",
            "    output \"Notes Out\" id=\"notes-out\" dialects=\"clap\" preferred=\"clap\"\n",
            "    note-name \"Kick\" key=36 channel=0 port=\"notes-in\"\n",
            "}\n",
            "state {}\n",
            "gui {}\n",
            "presets {}\n",
            "factories {}\n",
            "extensions {\n",
            "    enable \"clap.audio-ports\"\n",
            "    enable \"clap.note-ports\"\n",
            "    enable \"clap.note-name\"\n",
            "}\n",
        ),
    )
    .expect("manifest");

    fs::write(
        root.join("plugin.ids.kdl"),
        concat!(
            "ids version=1 next=6 {\n",
            "    entry kind=\"audio-port\" key=\"main-in\" value=1 tombstone=#false\n",
            "    entry kind=\"audio-port\" key=\"main-out\" value=2 tombstone=#false\n",
            "    entry kind=\"audio-port\" key=\"sidechain\" value=3 tombstone=#false\n",
            "    entry kind=\"note-port\" key=\"notes-in\" value=4 tombstone=#false\n",
            "    entry kind=\"note-port\" key=\"notes-out\" value=5 tombstone=#false\n",
            "}\n",
        ),
    )
    .expect("registry");

    let out = root.join("generated");
    generate(&manifest, &out);
    let generated = generated_sources(&out);

    assert!(
        generated.contains("clap_audio_port_info_t"),
        "#11 must generate native clap_audio_port_info_t descriptors rather than a mirror ABI:\n{generated}"
    );
    assert!(
        generated.contains("clap_note_port_info_t"),
        "#11 must generate native clap_note_port_info_t descriptors:\n{generated}"
    );
    assert!(
        generated.contains("clap_note_name_t"),
        "#11 must generate native clap_note_name_t entries:\n{generated}"
    );
    assert!(generated.contains("CLAP_EXT_AUDIO_PORTS"), "missing clap.audio-ports ownership");
    assert!(generated.contains("CLAP_EXT_NOTE_PORTS"), "missing clap.note-ports ownership");
    assert!(generated.contains("CLAP_EXT_NOTE_NAME"), "missing clap.note-name ownership");

    let ids = fs::read_to_string(out.join("clapgen_ids.hpp")).expect("generated ID header");
    for expected in [
        "namespace kind_audio_2Dport",
        "id_main_2Din = 1u",
        "id_main_2Dout = 2u",
        "id_sidechain = 3u",
        "namespace kind_note_2Dport",
        "id_notes_2Din = 4u",
        "id_notes_2Dout = 5u",
    ] {
        assert!(ids.contains(expected), "missing immutable port ID `{expected}`:\n{ids}");
    }

    fs::remove_dir_all(root).expect("temporary directory cleanup");
}
