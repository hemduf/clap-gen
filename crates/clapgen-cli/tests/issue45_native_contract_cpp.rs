use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!("clapgen-{name}-{}-{nonce}", std::process::id()))
}

fn run(command: &mut Command, context: &str) {
    let output =
        command.output().unwrap_or_else(|error| panic!("failed to run {context}: {error}"));
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_native_processor_contract_and_ids_compile_as_cxx20() {
    let root = temporary_directory("issue45-native-contract-cpp");
    let source = root.join("source");
    let generated = root.join("generated");
    let include = root.join("include/clap");
    let build = root.join("cmake-build");
    fs::create_dir_all(&source).expect("source directory");
    fs::create_dir_all(&include).expect("stub CLAP include directory");

    let manifest = source.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.native-contract\" name=\"Native Contract\" vendor=\"Example\" version=\"1.0.0\"\nprocessor class=\"CompileProcessor\"\nparameters { param \"Gain\" id=\"gain\" min=0 max=1 default=0.5 }\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("manifest should be writable");
    fs::write(
        source.join("plugin.ids.kdl"),
        "ids version=1 next=2 {\n    entry kind=\"parameter\" key=\"gain\" value=1 tombstone=#false\n}\n",
    )
    .expect("ID registry should be writable");
    fs::write(
        include.join("clap.h"),
        "#pragma once\n#include <stdint.h>\ntypedef uint32_t clap_id;\ntypedef int32_t clap_process_status;\ntypedef struct clap_process { uint32_t frames_count; } clap_process_t;\n",
    )
    .expect("CLAP stub should be writable");

    let generation = Command::new(env!("CARGO_BIN_EXE_clapgen"))
        .args(["generate", "--metadata"])
        .arg(&manifest)
        .arg("--out")
        .arg(&generated)
        .output()
        .expect("clapgen generate should run");
    assert!(
        generation.status.success(),
        "generation failed: {}",
        String::from_utf8_lossy(&generation.stderr)
    );

    fs::write(
        root.join("smoke.cpp"),
        "#include \"clapgen_ids.hpp\"\n#include \"clapgen_processor.hpp\"\n#include <concepts>\n#include <cstdint>\n\nstruct CompileProcessor {\n    bool init();\n    bool activate(double, std::uint32_t, std::uint32_t);\n    void deactivate();\n    bool start_processing();\n    void stop_processing();\n    void reset();\n    clap_process_status process(const clap_process_t*);\n};\n\nstatic_assert(clapgen::generated::NativeProcessor<CompileProcessor>);\nstatic_assert(std::same_as<decltype(clapgen::generated::ids::kind_parameter::id_gain), const clap_id>);\nstatic_assert(clapgen::generated::ids::kind_parameter::id_gain == 1u);\n\nint main() { return 0; }\n",
    )
    .expect("smoke source");
    fs::write(
        root.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.20)\nproject(clapgen_issue45_smoke LANGUAGES CXX)\nadd_executable(issue45_smoke smoke.cpp)\ntarget_include_directories(issue45_smoke PRIVATE generated include)\ntarget_compile_features(issue45_smoke PRIVATE cxx_std_20)\n",
    )
    .expect("CMake project");

    run(
        Command::new("cmake").arg("-S").arg(&root).arg("-B").arg(&build),
        "CMake configure for generated native contract",
    );
    run(
        Command::new("cmake").arg("--build").arg(&build).arg("--config").arg("Release"),
        "CMake build for generated native contract",
    );

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
