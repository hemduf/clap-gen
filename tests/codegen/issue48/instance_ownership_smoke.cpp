#include <clap/clap.h>

#include <cstdint>
#include <limits>
#include <stdexcept>

#include "clapgen_instance_backend.hpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

namespace {

struct LifetimeToken {
  LifetimeToken() { ++alive; }
  ~LifetimeToken() { --alive; }

  static inline int alive = 0;
};

struct InstrumentedProcessor {
  InstrumentedProcessor() {
    if (throw_on_construct) {
      throw std::runtime_error("injected processor construction failure");
    }
    instance_id = next_instance_id++;
    ++constructed;
  }

  ~InstrumentedProcessor() { ++destroyed; }

  bool init() {
    ++hook_calls;
    return true;
  }

  bool activate(double, std::uint32_t, std::uint32_t) {
    ++hook_calls;
    return true;
  }

  void deactivate() { ++hook_calls; }

  bool start_processing() {
    ++hook_calls;
    return true;
  }

  void stop_processing() { ++hook_calls; }
  void reset() { ++hook_calls; }

  clap_process_status process(const clap_process_t*) {
    ++hook_calls;
    return CLAP_PROCESS_CONTINUE;
  }

  static void reset_counters() {
    throw_on_construct = false;
    constructed = 0;
    destroyed = 0;
    hook_calls = 0;
    next_instance_id = 1u;
    LifetimeToken::alive = 0;
  }

  LifetimeToken lifetime{};
  std::uint32_t instance_id = 0u;
  int mutable_value = 0;

  static inline bool throw_on_construct = false;
  static inline int constructed = 0;
  static inline int destroyed = 0;
  static inline int hook_calls = 0;
  static inline std::uint32_t next_instance_id = 1u;
};

static_assert(generated::NativeProcessor<InstrumentedProcessor>);

clap_host_t make_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen issue48 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  return host;
}

using Instance = detail::PluginInstance<InstrumentedProcessor>;

int constructor_failure_is_clean(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();
  InstrumentedProcessor::throw_on_construct = true;

  bool threw = false;
  try {
    (void)Instance::create(generated::plugin_descriptors[0], host);
  } catch (const std::runtime_error&) {
    threw = true;
  }

  if (!threw || InstrumentedProcessor::constructed != 0 || InstrumentedProcessor::destroyed != 0 ||
      LifetimeToken::alive != 0) {
    return 1;
  }
  return 0;
}

int setup_failure_cleans_constructed_processor(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  if (Instance::create(nullptr, host) != nullptr) {
    return 2;
  }
  if (InstrumentedProcessor::constructed != 1 || InstrumentedProcessor::destroyed != 1 ||
      LifetimeToken::alive != 0) {
    return 3;
  }
  return 0;
}

int init_failure_remains_destructible(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  const clap_plugin_t* plugin = detail::create_plugin_instance_for<InstrumentedProcessor>(0u, host);
  if (plugin == nullptr || plugin->desc != generated::plugin_descriptors[0] ||
      plugin->plugin_data == nullptr) {
    return 4;
  }
  if (plugin->init == nullptr || plugin->destroy == nullptr || plugin->activate == nullptr ||
      plugin->deactivate == nullptr || plugin->start_processing == nullptr ||
      plugin->stop_processing == nullptr || plugin->reset == nullptr ||
      plugin->process == nullptr || plugin->get_extension == nullptr ||
      plugin->on_main_thread == nullptr) {
    return 5;
  }

  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr || instance->host() != host || instance->processor().instance_id == 0u) {
    return 6;
  }

  if (plugin->init(plugin)) {
    return 7;
  }
  if (plugin->activate(plugin, 48000.0, 32u, 1024u)) {
    return 8;
  }
  plugin->deactivate(plugin);
  if (plugin->start_processing(plugin)) {
    return 9;
  }
  plugin->stop_processing(plugin);
  plugin->reset(plugin);
  if (plugin->process(plugin, nullptr) != CLAP_PROCESS_ERROR) {
    return 10;
  }
  if (plugin->get_extension(plugin, "clap.test") != nullptr) {
    return 11;
  }
  plugin->on_main_thread(plugin);
  if (InstrumentedProcessor::hook_calls != 0) {
    return 12;
  }

  plugin->destroy(plugin);
  if (InstrumentedProcessor::constructed != 1 || InstrumentedProcessor::destroyed != 1 ||
      LifetimeToken::alive != 0) {
    return 13;
  }
  return 0;
}

int invalid_index_does_not_construct(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  if (detail::create_plugin_instance_for<InstrumentedProcessor>(
          std::numeric_limits<std::uint32_t>::max(), host) != nullptr) {
    return 14;
  }
  if (InstrumentedProcessor::constructed != 0 || InstrumentedProcessor::destroyed != 0 ||
      LifetimeToken::alive != 0) {
    return 15;
  }
  return 0;
}

int multiple_instances_are_isolated(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  const clap_plugin_t* first = detail::create_plugin_instance_for<InstrumentedProcessor>(0u, host);
  const clap_plugin_t* second = detail::create_plugin_instance_for<InstrumentedProcessor>(0u, host);
  if (first == nullptr || second == nullptr || first == second ||
      first->plugin_data == second->plugin_data) {
    return 16;
  }

  Instance* first_instance = Instance::from_plugin(first);
  Instance* second_instance = Instance::from_plugin(second);
  if (first_instance == nullptr || second_instance == nullptr ||
      first_instance->processor().instance_id == second_instance->processor().instance_id) {
    return 17;
  }

  first_instance->processor().mutable_value = 41;
  second_instance->processor().mutable_value = 7;
  if (first_instance->processor().mutable_value != 41 ||
      second_instance->processor().mutable_value != 7) {
    return 18;
  }

  first->destroy(first);
  if (InstrumentedProcessor::destroyed != 1 || LifetimeToken::alive != 1 ||
      second_instance->processor().mutable_value != 7) {
    return 19;
  }
  second->destroy(second);
  if (InstrumentedProcessor::constructed != 2 || InstrumentedProcessor::destroyed != 2 ||
      LifetimeToken::alive != 0) {
    return 20;
  }
  return 0;
}

int first_failure(const clap_host_t* host) {
  int result = constructor_failure_is_clean(host);
  if (result != 0) {
    return result;
  }
  result = setup_failure_cleans_constructed_processor(host);
  if (result != 0) {
    return result;
  }
  result = init_failure_remains_destructible(host);
  if (result != 0) {
    return result;
  }
  result = invalid_index_does_not_construct(host);
  if (result != 0) {
    return result;
  }
  return multiple_instances_are_isolated(host);
}

} // namespace

int main() {
  const auto host = make_host();
  const int result = first_failure(&host);
  if (result != 0) {
    return result;
  }

  if (Instance::from_plugin(nullptr) != nullptr) {
    return 21;
  }
  return 0;
}
