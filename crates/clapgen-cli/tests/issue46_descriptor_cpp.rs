use std::env;
use std::fs;
use std::path::PathBuf;
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
fn generated_descriptor_compiles_and_has_one_stable_address_across_translation_units() {
    let root = temporary_directory("issue46-descriptor-cpp");
    let source = root.join("source");
    let generated = root.join("generated");
    let include = root.join("include/clap");
    let build = root.join("cmake-build");
    fs::create_dir_all(&source).expect("source directory");
    fs::create_dir_all(&include).expect("stub CLAP include directory");

    let manifest = source.join("plugin.kdl");
    fs::write(
        &manifest,
        "clapgen schema=\"1.0.0\"\nplugin id=\"com.example.descriptor-smoke\" name=\"Descriptor Smoke\" vendor=\"Example\" version=\"1.2.3\" {\n    feature \"audio-effect\"\n    feature \"stereo\"\n}\nprocessor class=\"DescriptorProcessor\"\nparameters {}\naudio-ports {}\nnote-ports {}\nstate {}\ngui {}\npresets {}\nfactories {}\nextensions {}\n",
    )
    .expect("manifest should be writable");
    fs::write(
        include.join("clap.h"),
        "#pragma once\n#include <stdint.h>\ntypedef struct clap_version { uint32_t major; uint32_t minor; uint32_t revision; } clap_version_t;\ninline constexpr clap_version_t CLAP_VERSION{1u, 2u, 10u};\ntypedef struct clap_plugin_descriptor {\n    clap_version_t clap_version;\n    const char* id;\n    const char* name;\n    const char* vendor;\n    const char* url;\n    const char* manual_url;\n    const char* support_url;\n    const char* version;\n    const char* description;\n    const char* const* features;\n} clap_plugin_descriptor_t;\n",
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
        root.join("descriptor_a.cpp"),
        "#include \"clapgen_descriptors.hpp\"\nextern \"C\" const clap_plugin_descriptor_t* descriptor_from_a() { return &clapgen::generated::plugin_descriptor_0; }\nextern \"C\" const char* const* features_from_a() { return clapgen::generated::plugin_descriptor_0.features; }\n",
    )
    .expect("translation unit A");
    fs::write(
        root.join("descriptor_b.cpp"),
        "#include \"clapgen_descriptors.hpp\"\nextern \"C\" const clap_plugin_descriptor_t* descriptor_from_b() { return &clapgen::generated::plugin_descriptor_0; }\nextern \"C\" const char* const* features_from_b() { return clapgen::generated::plugin_descriptor_0.features; }\n",
    )
    .expect("translation unit B");
    fs::write(
        root.join("main.cpp"),
        "#include \"clapgen_descriptors.hpp\"\n#include <cstring>\n#include <type_traits>\n\nextern \"C\" const clap_plugin_descriptor_t* descriptor_from_a();\nextern \"C\" const clap_plugin_descriptor_t* descriptor_from_b();\nextern \"C\" const char* const* features_from_a();\nextern \"C\" const char* const* features_from_b();\n\nusing GeneratedDescriptor = decltype(clapgen::generated::plugin_descriptor_0);\nstatic_assert(std::is_const_v<GeneratedDescriptor>);\nstatic_assert(std::is_same_v<std::remove_cv_t<GeneratedDescriptor>, clap_plugin_descriptor_t>);\nstatic_assert(clapgen::generated::plugin_descriptor_count == 1u);\n\nint main() {\n    const auto* descriptor = &clapgen::generated::plugin_descriptor_0;\n    if (descriptor_from_a() != descriptor || descriptor_from_b() != descriptor) return 1;\n    if (features_from_a() != descriptor->features || features_from_b() != descriptor->features) return 2;\n    if (clapgen::generated::plugin_descriptors[0] != descriptor) return 3;\n    if (std::strcmp(descriptor->id, \"com.example.descriptor-smoke\") != 0) return 4;\n    if (std::strcmp(descriptor->name, \"Descriptor Smoke\") != 0) return 5;\n    if (!descriptor->features || !descriptor->features[0] || !descriptor->features[1]) return 6;\n    if (descriptor->features[2] != nullptr) return 7;\n    if (std::strcmp(descriptor->features[0], \"audio-effect\") != 0) return 8;\n    if (std::strcmp(descriptor->features[1], \"stereo\") != 0) return 9;\n    return 0;\n}\n",
    )
    .expect("main translation unit");
    fs::write(
        root.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.20)\nproject(clapgen_issue46_descriptor_smoke LANGUAGES CXX)\nadd_executable(issue46_descriptor_smoke main.cpp descriptor_a.cpp descriptor_b.cpp)\ntarget_include_directories(issue46_descriptor_smoke PRIVATE generated include)\ntarget_compile_features(issue46_descriptor_smoke PRIVATE cxx_std_20)\n",
    )
    .expect("CMake project");

    run(
        Command::new("cmake").arg("-S").arg(&root).arg("-B").arg(&build),
        "CMake configure for generated descriptor",
    );
    run(
        Command::new("cmake").arg("--build").arg(&build).arg("--config").arg("Release"),
        "CMake build for generated descriptor",
    );

    let executable = if cfg!(windows) {
        build.join("Release/issue46_descriptor_smoke.exe")
    } else {
        build.join("issue46_descriptor_smoke")
    };
    run(Command::new(executable), "generated descriptor smoke executable");

    fs::remove_dir_all(root).expect("temporary directory should be removable");
}
