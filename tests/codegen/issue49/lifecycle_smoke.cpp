#include <clap/clap.h>

#include <cstdint>

#include "clapgen_instance_backend.hpp"

namespace detail = clapgen::generated::detail;

namespace {

struct LifecycleProcessor {
  bool init() {
    ++init_calls;
    return init_result;
  }

  bool activate(double sample_rate, std::uint32_t min_frames, std::uint32_t max_frames) {
    ++activate_calls;
    last_sample_rate = sample_rate;
    last_min_frames = min_frames;
    last_max_frames = max_frames;
    return activate_result;
  }

  void deactivate() { ++deactivate_calls; }

  bool start_processing() {
    ++start_calls;
    return start_result;
  }

  void stop_processing() { ++stop_calls; }
  void reset() { ++reset_calls; }

  clap_process_status process(const clap_process_t* process) {
    ++process_calls;
    last_process = process;
    return process_result;
  }

  bool init_result = true;
  bool activate_result = true;
  bool start_result = true;
  clap_process_status process_result = CLAP_PROCESS_TAIL;
  int init_calls = 0;
  int activate_calls = 0;
  int deactivate_calls = 0;
  int start_calls = 0;
  int stop_calls = 0;
  int reset_calls = 0;
  int process_calls = 0;
  double last_sample_rate = 0.0;
  std::uint32_t last_min_frames = 0u;
  std::uint32_t last_max_frames = 0u;
  const clap_process_t* last_process = nullptr;
};

using Instance = detail::PluginInstance<LifecycleProcessor>;

clap_host_t make_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen issue49 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  return host;
}

int legal_sequence(const clap_host_t* host) {
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<LifecycleProcessor>(0u, host);
  if (plugin == nullptr) {
    return 1;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 2;
  }
  const LifecycleProcessor& processor = instance->processor();

  if (!plugin->init(plugin) || processor.init_calls != 1) {
    return 3;
  }
  if (!plugin->activate(plugin, 48000.25, 17u, 4096u) || processor.activate_calls != 1 ||
      processor.last_sample_rate != 48000.25 || processor.last_min_frames != 17u ||
      processor.last_max_frames != 4096u) {
    return 4;
  }

  plugin->reset(plugin);
  if (processor.reset_calls != 1) {
    return 5;
  }
  if (!plugin->start_processing(plugin) || processor.start_calls != 1) {
    return 6;
  }

  clap_process_t process{};
  if (plugin->process(plugin, &process) != CLAP_PROCESS_TAIL || processor.process_calls != 1 ||
      processor.last_process != &process) {
    return 7;
  }
  plugin->reset(plugin);
  if (processor.reset_calls != 2) {
    return 8;
  }

  plugin->stop_processing(plugin);
  if (processor.stop_calls != 1) {
    return 9;
  }
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 1) {
    return 10;
  }

  if (!plugin->activate(plugin, 96000.0, 1u, 2048u) || processor.activate_calls != 2) {
    return 11;
  }
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 2) {
    return 12;
  }
  plugin->destroy(plugin);
  return 0;
}

int invalid_order_is_fail_closed(const clap_host_t* host) {
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<LifecycleProcessor>(0u, host);
  if (plugin == nullptr) {
    return 13;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 14;
  }
  const LifecycleProcessor& processor = instance->processor();
  clap_process_t process{};

  if (plugin->activate(plugin, 48000.0, 32u, 1024u) || plugin->start_processing(plugin) ||
      plugin->process(plugin, &process) != CLAP_PROCESS_ERROR) {
    return 15;
  }
  plugin->reset(plugin);
  plugin->deactivate(plugin);
  plugin->stop_processing(plugin);
  if (processor.activate_calls != 0 || processor.start_calls != 0 || processor.process_calls != 0 ||
      processor.reset_calls != 0 || processor.deactivate_calls != 0 || processor.stop_calls != 0) {
    return 16;
  }

  const bool first_init = plugin->init(plugin);
  const bool second_init = plugin->init(plugin);
  if (!first_init || second_init || processor.init_calls != 1) {
    return 17;
  }
  if (plugin->start_processing(plugin) || plugin->process(plugin, &process) != CLAP_PROCESS_ERROR) {
    return 18;
  }
  plugin->reset(plugin);
  if (processor.start_calls != 0 || processor.process_calls != 0 || processor.reset_calls != 0) {
    return 19;
  }

  const bool first_activate = plugin->activate(plugin, 44100.0, 8u, 512u);
  const bool second_activate = plugin->activate(plugin, 44100.0, 8u, 512u);
  if (!first_activate || second_activate || processor.activate_calls != 1) {
    return 20;
  }
  const bool first_start = plugin->start_processing(plugin);
  const bool second_start = plugin->start_processing(plugin);
  if (!first_start || second_start || processor.start_calls != 1) {
    return 21;
  }
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 0) {
    return 22;
  }

  plugin->stop_processing(plugin);
  plugin->stop_processing(plugin);
  if (processor.stop_calls != 1) {
    return 23;
  }
  plugin->deactivate(plugin);
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 1) {
    return 24;
  }
  if (plugin->process(plugin, &process) != CLAP_PROCESS_ERROR) {
    return 25;
  }
  plugin->reset(plugin);
  if (processor.process_calls != 0 || processor.reset_calls != 0) {
    return 26;
  }

  plugin->destroy(plugin);
  return 0;
}

int failure_states_remain_retryable_or_destructible(const clap_host_t* host) {
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<LifecycleProcessor>(0u, host);
  if (plugin == nullptr) {
    return 27;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 28;
  }
  LifecycleProcessor& processor = instance->processor();
  processor.activate_result = false;
  processor.start_result = false;

  if (!plugin->init(plugin)) {
    return 29;
  }
  if (plugin->activate(plugin, 48000.0, 4u, 256u) || processor.activate_calls != 1) {
    return 30;
  }
  processor.activate_result = true;
  if (!plugin->activate(plugin, 48000.0, 4u, 256u) || processor.activate_calls != 2) {
    return 31;
  }
  if (plugin->start_processing(plugin) || processor.start_calls != 1) {
    return 32;
  }
  processor.start_result = true;
  if (!plugin->start_processing(plugin) || processor.start_calls != 2) {
    return 33;
  }
  plugin->stop_processing(plugin);
  plugin->deactivate(plugin);
  plugin->destroy(plugin);
  return 0;
}

} // namespace

int main() {
  const auto host = make_host();
  if (const int result = legal_sequence(&host); result != 0) {
    return result;
  }
  if (const int result = invalid_order_is_fail_closed(&host); result != 0) {
    return result;
  }
  return failure_states_remain_retryable_or_destructible(&host);
}
