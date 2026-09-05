use super::{entry_cpp, instance_backend_cpp, processor_cpp};

#[test]
fn issue50_contains_every_generated_boundary_that_can_reach_throwing_cpp() {
    let header = instance_backend_cpp::header();

    assert_eq!(
        header.matches("catch (...)").count(),
        11,
        "every processor/destructor/thread-check-facing CLAP callback must contain exceptions:\n{header}"
    );

    for required in [
        "instance->state_ = LifecycleState::InitFailed;\n            return false;\n        } catch (...) {\n            instance->state_ = LifecycleState::InitFailed;\n            return false;",
        "try {\n            delete instance;\n        } catch (...) {\n        }",
        "try {\n            if (!instance->processor_.activate(sample_rate, min_frames_count, max_frames_count))",
        "try {\n            if (!instance->processor_.start_processing())",
        "try {\n            instance->processor_.stop_processing();\n        } catch (...) {\n        }\n        instance->state_ = LifecycleState::Active;",
        "try {\n            instance->processor_.deactivate();\n        } catch (...) {\n        }\n        instance->state_ = LifecycleState::Initialized;",
        "try {\n            instance->processor_.reset();\n        } catch (...) {\n        }",
        "try {\n            return instance->processor_.process(process);\n        } catch (...) {\n            return CLAP_PROCESS_ERROR;\n        }",
        "host_->get_extension(host_, CLAP_EXT_THREAD_CHECK));\n            return true;\n        } catch (...) {\n            return false;",
        "return thread_check_->is_main_thread(host_);\n        } catch (...) {\n            return false;",
        "return thread_check_->is_audio_thread(host_);\n        } catch (...) {\n            return false;",
    ] {
        assert!(header.contains(required), "missing containment contract `{required}`:\n{header}");
    }

    let entry = entry_cpp::source();
    for required in [
        "try {\n                return create_plugin_instance(index, host);\n            } catch (...) {\n                return nullptr;\n            }",
        "const clap_plugin_t* CLAP_ABI factory_create_plugin(",
    ] {
        assert!(entry.contains(required), "missing factory containment `{required}`:\n{entry}");
    }
}

#[test]
fn issue50_documents_borrowed_lifetimes_and_rejects_callback_pointer_retention() {
    let processor = processor_cpp::header();
    let backend = instance_backend_cpp::header();

    for required in [
        "Borrowed callback lifetime",
        "clap_process_t and its nested event lists and audio buffers are host-owned",
        "must not be retained after Processor::process() returns",
    ] {
        assert!(
            processor.contains(required),
            "missing processor lifetime note `{required}`:\n{processor}"
        );
    }

    for required in [
        "host_ is a borrowed, non-owning pointer owned by the CLAP host",
        "plugin descriptors point to immutable generated static storage",
        "host extension pointers are borrowed and remain host-owned",
    ] {
        assert!(
            backend.contains(required),
            "missing instance lifetime note `{required}`:\n{backend}"
        );
    }

    for forbidden in [
        "current_process_",
        "currentProcess",
        "cached_process",
        "process_ = process",
        "std::unique_ptr<clap_process_t",
        "std::shared_ptr<clap_process_t",
        "in_events_",
        "out_events_",
        "audio_inputs_",
        "audio_outputs_",
    ] {
        assert!(
            !backend.contains(forbidden),
            "callback-scoped pointer retention leaked through `{forbidden}`:\n{backend}"
        );
    }
}
