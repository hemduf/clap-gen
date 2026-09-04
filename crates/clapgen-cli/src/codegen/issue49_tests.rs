use super::instance_backend_cpp;

#[test]
fn issue49_generates_explicit_lifecycle_state_machine_and_native_dispatch() {
    let header = instance_backend_cpp::header();

    for required in [
        "enum class LifecycleState",
        "Created",
        "Initialized",
        "Active",
        "Processing",
        ".init = init_plugin,",
        ".activate = activate_plugin,",
        ".deactivate = deactivate_plugin,",
        ".start_processing = start_processing_plugin,",
        ".stop_processing = stop_processing_plugin,",
        ".reset = reset_plugin,",
        ".process = process_plugin,",
        "processor_.init()",
        "processor_.activate(sample_rate, min_frames_count, max_frames_count)",
        "processor_.start_processing()",
        "processor_.process(process)",
        "processor_.stop_processing()",
        "processor_.deactivate()",
        "processor_.reset()",
        "LifecycleState state_ = LifecycleState::Created;",
    ] {
        assert!(header.contains(required), "missing `{required}`:\n{header}");
    }

    for forbidden in [
        "unavailable_init",
        "unavailable_activate",
        "unavailable_start_processing",
        "unavailable_stop_processing",
        "unavailable_reset",
        "unavailable_process",
        "ProcessBlock",
        "ProcessStatus",
        "ActivateContext",
    ] {
        assert!(!header.contains(forbidden), "unexpected `{forbidden}`:\n{header}");
    }
}

#[test]
fn issue49_lifecycle_guards_invalid_ordering_and_preserves_native_values() {
    let header = instance_backend_cpp::header();

    for required in [
        "state_ != LifecycleState::Created",
        "state_ != LifecycleState::Initialized",
        "state_ != LifecycleState::Active",
        "state_ != LifecycleState::Processing",
        "state_ = LifecycleState::Initialized;",
        "state_ = LifecycleState::Active;",
        "state_ = LifecycleState::Processing;",
        "return CLAP_PROCESS_ERROR;",
        "return instance->processor_.process(process);",
    ] {
        assert!(header.contains(required), "missing `{required}`:\n{header}");
    }
}
