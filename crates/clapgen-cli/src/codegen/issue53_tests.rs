use super::instance_backend_cpp;

#[test]
fn issue53_debug_thread_checks_cover_native_clap_callback_annotations() {
    let backend = instance_backend_cpp::header();

    for required in [
        "#include <clap/ext/thread-check.h>",
        "host_->get_extension(host_, CLAP_EXT_THREAD_CHECK)",
        "debug_on_main_thread()",
        "debug_on_audio_thread()",
        "static bool CLAP_ABI init_plugin(",
        "static bool CLAP_ABI activate_plugin(",
        "static void CLAP_ABI deactivate_plugin(",
        "static bool CLAP_ABI start_processing_plugin(",
        "static void CLAP_ABI stop_processing_plugin(",
        "static void CLAP_ABI reset_plugin(",
        "static clap_process_status CLAP_ABI process_plugin(",
    ] {
        assert!(backend.contains(required), "missing `{required}`:\n{backend}");
    }

    assert!(
        backend.matches("debug_on_main_thread()").count() >= 5,
        "main-thread lifecycle callbacks must be guarded in debug builds:\n{backend}"
    );
    assert!(
        backend.matches("debug_on_audio_thread()").count() >= 4,
        "audio-thread callbacks must be guarded in debug builds:\n{backend}"
    );
}

#[test]
fn issue53_realtime_callback_region_has_no_generated_allocation_lock_io_or_unbounded_loop() {
    let backend = instance_backend_cpp::header();
    let start = backend
        .find("static bool CLAP_ABI start_processing_plugin(")
        .expect("start-processing callback");
    let end = backend
        .find("static const void* CLAP_ABI get_extension_plugin(")
        .expect("extension callback");
    let realtime = &backend[start..end];

    for forbidden in [
        "new ",
        "delete ",
        "malloc(",
        "calloc(",
        "realloc(",
        "free(",
        "std::mutex",
        "std::lock_guard",
        "std::unique_lock",
        "condition_variable",
        "std::cout",
        "std::cerr",
        "fprintf(",
        "printf(",
        "sleep(",
        "wait(",
        "for (",
        "while (",
    ] {
        assert!(
            !realtime.contains(forbidden),
            "realtime generated callback region contains forbidden `{forbidden}`:\n{realtime}"
        );
    }
}

#[test]
fn issue53_thread_checks_are_debug_only_and_release_path_stays_minimal() {
    let backend = instance_backend_cpp::header();

    for required in [
        "#ifndef NDEBUG\n#include <clap/ext/thread-check.h>\n#endif",
        "#ifndef NDEBUG\n    bool cache_debug_thread_check() noexcept",
        "#ifndef NDEBUG\n    const clap_host_thread_check_t* thread_check_ = nullptr;\n#endif",
        "#ifndef NDEBUG\n        if (!instance->debug_on_audio_thread())",
        "#ifndef NDEBUG\n        if (!instance->debug_on_main_thread())",
    ] {
        assert!(backend.contains(required), "missing debug-only contract `{required}`:\n{backend}");
    }
}

#[test]
fn issue53_smoke_is_registered_in_cmake_and_cross_platform_ci() {
    let cmake = include_str!("../../../../CMakeLists.txt");
    assert!(
        cmake.contains("tests/codegen/issue53/Issue53.cmake"),
        "issue53 CMake harness must be registered:\n{cmake}"
    );

    let ci = include_str!("../../../../.github/workflows/ci.yml");
    for required in ["tests/codegen/issue53/realtime_thread_smoke.cpp", "tests/codegen/issue53"] {
        assert!(ci.contains(required), "CI is missing `{required}`:\n{ci}");
    }
}
