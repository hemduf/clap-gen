#include "clapgen_instance_backend.hpp"

#include <array>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>

namespace {

struct TestProcessor {
    bool init() { return true; }
    bool activate(double, std::uint32_t, std::uint32_t) { return true; }
    void deactivate() {}
    bool start_processing() { return true; }
    void stop_processing() {}
    void reset() {}
    clap_process_status process(const clap_process_t*) {
        ++process_calls;
        return CLAP_PROCESS_CONTINUE;
    }

    void on_parameter_event(const clap_event_header_t* event) {
        if (event_count < event_types.size()) {
            event_types[event_count] = event->type;
            event_times[event_count] = event->time;
            ++event_count;
        }
    }

    void on_state_loaded() { ++state_loads; }

    std::array<std::uint16_t, 8> event_types{};
    std::array<std::uint32_t, 8> event_times{};
    std::size_t event_count = 0;
    std::uint32_t process_calls = 0;
    std::uint32_t state_loads = 0;
};

struct InputEvents {
    InputEvents() {
        iface.ctx = this;
        iface.size = size_callback;
        iface.get = get_callback;
    }

    void push(const clap_event_header_t* event) {
        assert(count < events.size());
        events[count++] = event;
    }

    static std::uint32_t CLAP_ABI size_callback(const clap_input_events_t* list) {
        return static_cast<const InputEvents*>(list->ctx)->count;
    }

    static const clap_event_header_t* CLAP_ABI get_callback(
        const clap_input_events_t* list,
        std::uint32_t index) {
        const auto* self = static_cast<const InputEvents*>(list->ctx);
        return index < self->count ? self->events[index] : nullptr;
    }

    clap_input_events_t iface{};
    std::array<const clap_event_header_t*, 8> events{};
    std::uint32_t count = 0;
};

struct FixedOutputStream {
    FixedOutputStream() {
        iface.ctx = this;
        iface.write = write_callback;
    }

    static std::int64_t CLAP_ABI write_callback(
        const clap_ostream_t* stream,
        const void* data,
        std::uint64_t size) {
        auto* self = static_cast<FixedOutputStream*>(stream->ctx);
        if (data == nullptr || size > self->bytes.size() - self->size) {
            return -1;
        }
        std::memcpy(self->bytes.data() + self->size, data, static_cast<std::size_t>(size));
        self->size += static_cast<std::size_t>(size);
        return static_cast<std::int64_t>(size);
    }

    clap_ostream_t iface{};
    std::array<std::byte, 4096> bytes{};
    std::size_t size = 0;
};

struct FixedInputStream {
    FixedInputStream(const std::byte* source, std::size_t source_size)
        : data(source), size(source_size) {
        iface.ctx = this;
        iface.read = read_callback;
    }

    static std::int64_t CLAP_ABI read_callback(
        const clap_istream_t* stream,
        void* destination,
        std::uint64_t requested) {
        auto* self = static_cast<FixedInputStream*>(stream->ctx);
        if (destination == nullptr) {
            return -1;
        }
        const auto available = self->size - self->offset;
        const auto count = std::min<std::size_t>(available, static_cast<std::size_t>(requested));
        if (count == 0u) {
            return 0;
        }
        std::memcpy(destination, self->data + self->offset, count);
        self->offset += count;
        return static_cast<std::int64_t>(count);
    }

    clap_istream_t iface{};
    const std::byte* data = nullptr;
    std::size_t size = 0;
    std::size_t offset = 0;
};

struct HostState {
    std::uint32_t value_rescans = 0u;
};

void CLAP_ABI host_params_rescan(const clap_host_t* host, clap_param_rescan_flags flags) {
    auto* state = static_cast<HostState*>(host->host_data);
    if ((flags & CLAP_PARAM_RESCAN_VALUES) != 0u) {
        ++state->value_rescans;
    }
}

void CLAP_ABI host_params_clear(const clap_host_t*, clap_id, clap_param_clear_flags) {}
void CLAP_ABI host_params_request_flush(const clap_host_t*) {}

const clap_host_params_t host_params{
    .rescan = host_params_rescan,
    .clear = host_params_clear,
    .request_flush = host_params_request_flush,
};

const void* CLAP_ABI get_host_extension(const clap_host_t*, const char* extension_id) {
    if (extension_id != nullptr && std::strcmp(extension_id, CLAP_EXT_PARAMS) == 0) {
        return &host_params;
    }
    return nullptr;
}

void CLAP_ABI no_request(const clap_host_t*) {}

HostState host_state{};
const clap_host_t host{
    .clap_version = CLAP_VERSION,
    .host_data = &host_state,
    .name = "issue10-host",
    .vendor = "clap-gen",
    .url = "https://example.invalid",
    .version = "1",
    .get_extension = get_host_extension,
    .request_restart = no_request,
    .request_process = no_request,
    .request_callback = no_request,
};

} // namespace

int main() {
    using Instance = clapgen::generated::detail::PluginInstance<TestProcessor>;
    const clap_plugin_t* plugin = Instance::create(
        clapgen::generated::plugin_descriptors[0],
        &host);
    assert(plugin != nullptr);
    assert(plugin->init(plugin));

    const auto* params = static_cast<const clap_plugin_params_t*>(
        plugin->get_extension(plugin, CLAP_EXT_PARAMS));
    const auto* state = static_cast<const clap_plugin_state_t*>(
        plugin->get_extension(plugin, CLAP_EXT_STATE));
    assert(params != nullptr);
    assert(state != nullptr);
    assert(params->count(plugin) == 2u);

    clap_param_info_t gain_info{};
    clap_param_info_t mode_info{};
    assert(params->get_info(plugin, 0u, &gain_info));
    assert(params->get_info(plugin, 1u, &mode_info));
    assert(gain_info.id == 1u);
    assert(mode_info.id == 2u);
    assert(std::strcmp(gain_info.name, "Gain") == 0);
    assert((gain_info.flags & CLAP_PARAM_IS_AUTOMATABLE) != 0u);
    assert((gain_info.flags & CLAP_PARAM_IS_MODULATABLE) != 0u);
    assert((mode_info.flags & CLAP_PARAM_IS_STEPPED) != 0u);
    assert((mode_info.flags & CLAP_PARAM_IS_ENUM) != 0u);

    double value = 0.0;
    assert(params->get_value(plugin, 1u, &value));
    assert(value == 1.0);

    char text[64]{};
    assert(params->value_to_text(plugin, 1u, 1.25, text, sizeof(text)));
    double parsed = 0.0;
    assert(params->text_to_value(plugin, 1u, text, &parsed));
    assert(std::abs(parsed - 1.25) < 1.0e-12);

    assert(plugin->activate(plugin, 48000.0, 1u, 64u));
    assert(plugin->start_processing(plugin));

    const clap_event_param_value_t stale_automation{
        .header = clap_event_header_t{
            .size = sizeof(clap_event_param_value_t),
            .time = 3u,
            .space_id = CLAP_CORE_EVENT_SPACE_ID,
            .type = CLAP_EVENT_PARAM_VALUE,
            .flags = 0u,
        },
        .param_id = 999u,
        .cookie = nullptr,
        .note_id = -1,
        .port_index = -1,
        .channel = -1,
        .key = -1,
        .value = 0.75,
    };
    const clap_event_param_value_t automation{
        .header = clap_event_header_t{
            .size = sizeof(clap_event_param_value_t),
            .time = 17u,
            .space_id = CLAP_CORE_EVENT_SPACE_ID,
            .type = CLAP_EVENT_PARAM_VALUE,
            .flags = 0u,
        },
        .param_id = 1u,
        .cookie = nullptr,
        .note_id = -1,
        .port_index = -1,
        .channel = -1,
        .key = -1,
        .value = 1.5,
    };
    const clap_event_param_mod_t modulation{
        .header = clap_event_header_t{
            .size = sizeof(clap_event_param_mod_t),
            .time = 31u,
            .space_id = CLAP_CORE_EVENT_SPACE_ID,
            .type = CLAP_EVENT_PARAM_MOD,
            .flags = 0u,
        },
        .param_id = 1u,
        .cookie = nullptr,
        .note_id = -1,
        .port_index = -1,
        .channel = -1,
        .key = -1,
        .amount = 0.25,
    };
    InputEvents process_events;
    process_events.push(&stale_automation.header);
    process_events.push(&automation.header);
    process_events.push(&modulation.header);
    const clap_process_t process{
        .steady_time = 0,
        .frames_count = 64u,
        .transport = nullptr,
        .audio_inputs = nullptr,
        .audio_outputs = nullptr,
        .audio_inputs_count = 0u,
        .audio_outputs_count = 0u,
        .in_events = &process_events.iface,
        .out_events = nullptr,
    };
    assert(plugin->process(plugin, &process) == CLAP_PROCESS_CONTINUE);

    auto* instance = Instance::from_plugin(plugin);
    assert(instance != nullptr);
    assert(instance->processor().process_calls == 1u);
    assert(instance->processor().event_count == 2u);
    assert(instance->processor().event_types[0] == CLAP_EVENT_PARAM_VALUE);
    assert(instance->processor().event_times[0] == 17u);
    assert(instance->processor().event_types[1] == CLAP_EVENT_PARAM_MOD);
    assert(instance->processor().event_times[1] == 31u);

    plugin->stop_processing(plugin);
    plugin->deactivate(plugin);
    assert(params->get_value(plugin, 1u, &value));
    assert(value == 1.5); // modulation must not overwrite the base value

    FixedOutputStream saved;
    assert(state->save(plugin, &saved.iface));
    assert(saved.size > 0u);
    assert(std::to_integer<unsigned char>(saved.bytes[0]) == static_cast<unsigned char>('C'));
    assert(std::to_integer<unsigned char>(saved.bytes[1]) == static_cast<unsigned char>('G'));
    assert(std::to_integer<unsigned char>(saved.bytes[2]) == static_cast<unsigned char>('P'));
    assert(std::to_integer<unsigned char>(saved.bytes[3]) == static_cast<unsigned char>('1'));

    const clap_event_param_value_t changed{
        .header = clap_event_header_t{
            .size = sizeof(clap_event_param_value_t),
            .time = 0u,
            .space_id = CLAP_CORE_EVENT_SPACE_ID,
            .type = CLAP_EVENT_PARAM_VALUE,
            .flags = 0u,
        },
        .param_id = 1u,
        .cookie = nullptr,
        .note_id = -1,
        .port_index = -1,
        .channel = -1,
        .key = -1,
        .value = 0.5,
    };
    InputEvents flush_events;
    flush_events.push(&changed.header);
    params->flush(plugin, &flush_events.iface, nullptr);
    assert(params->get_value(plugin, 1u, &value));
    assert(value == 0.5);

    FixedInputStream truncated(saved.bytes.data(), saved.size - 1u);
    assert(!state->load(plugin, &truncated.iface));
    assert(params->get_value(plugin, 1u, &value));
    assert(value == 0.5); // failed load is transactional
    assert(host_state.value_rescans == 0u);

    FixedInputStream restored(saved.bytes.data(), saved.size);
    assert(state->load(plugin, &restored.iface));
    assert(params->get_value(plugin, 1u, &value));
    assert(value == 1.5);
    assert(instance->processor().state_loads == 1u);
    assert(host_state.value_rescans == 1u);

    plugin->destroy(plugin);
    return 0;
}
