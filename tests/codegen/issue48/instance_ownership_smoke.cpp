#include <clap/clap.h>

#include <atomic>
#include <cstdint>
#include <limits>
#include <stdexcept>

#include "clapgen_instance_backend.hpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

namespace {

struct LifetimeToken {
  LifetimeToken() { alive.fetch_add(1, std::memory_order_relaxed); }
  ~LifetimeToken() { alive.fetch_sub(1, std::memory_order_relaxed); }

  static inline std::atomic<int> alive{0};
};

struct InstrumentedProcessor {
  InstrumentedProcessor() {
    if (throw_on_construct.load(std::memory_order_relaxed)) {
      throw std::runtime_error("injected processor construction failure");
    }
    instance_id = next_instance_id.fetch_add(1u, std::memory_order_relaxed);
    constructed.fetch_add(1, std::memory_order_relaxed);
  }

  ~InstrumentedProcessor() { destroyed.fetch_add(1, std::memory_order_relaxed); }

  bool init() {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
    return true;
  }

  bool activate(double, std::uint32_t, std::uint32_t) {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
    return true;
  }

  void deactivate() {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
  }

  bool start_processing() {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
    return true;
  }

  void stop_processing() {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
  }

  void reset() {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
  }

  clap_process_status process(const clap_process_t*) {
    ++mutable_value;
    hook_calls.fetch_add(1, std::memory_order_relaxed);
    return CLAP_PROCESS_CONTINUE;
  }

  static void reset_counters() {
    throw_on_construct.store(false, std::memory_order_relaxed);
    constructed.store(0, std::memory_order_relaxed);
    destroyed.store(0, std::memory_order_relaxed);
    hook_calls.store(0, std::memory_order_relaxed);
    next_instance_id.store(1u, std::memory_order_relaxed);
    LifetimeToken::alive.store(0, std::memory_order_relaxed);
  }

  LifetimeToken lifetime{};
  std::uint32_t instance_id = 0u;
  int mutable_value = 0;

  static inline std::atomic<bool> throw_on_construct{false};
  static inline std::atomic<int> constructed{0};
  static inline std::atomic<int> destroyed{0};
  static inline std::atomic<int> hook_calls{0};
  static inline std::atomic<std::uint32_t> next_instance_id{1u};
};

static_assert(generated::NativeProcessor<InstrumentedProcessor>);

int counter(const std::atomic<int>& value) { return value.load(std::memory_order_relaxed); }

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
  InstrumentedProcessor::throw_on_construct.store(true, std::memory_order_relaxed);

  bool threw = false;
  try {
    (void)Instance::create(generated::plugin_descriptors[0], host);
  } catch (const std::runtime_error&) {
    threw = true;
  }

  if (!threw || counter(InstrumentedProcessor::constructed) != 0 ||
      counter(InstrumentedProcessor::destroyed) != 0 || counter(LifetimeToken::alive) != 0) {
    return 1;
  }
  return 0;
}

int setup_failure_cleans_constructed_processor(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  if (Instance::create(nullptr, host) != nullptr) {
    return 2;
  }
  if (counter(InstrumentedProcessor::constructed) != 1 ||
      counter(InstrumentedProcessor::destroyed) != 1 || counter(LifetimeToken::alive) != 0) {
    return 3;
  }
  return 0;
}

int created_instance_is_owned_and_destructible(const clap_host_t* host) {
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
  if (plugin->get_extension(plugin, "clap.test") != nullptr) {
    return 7;
  }
  plugin->on_main_thread(plugin);
  if (counter(InstrumentedProcessor::hook_calls) != 0 || instance->processor().mutable_value != 0) {
    return 8;
  }

  plugin->destroy(plugin);
  if (counter(InstrumentedProcessor::constructed) != 1 ||
      counter(InstrumentedProcessor::destroyed) != 1 || counter(LifetimeToken::alive) != 0) {
    return 9;
  }
  return 0;
}

int invalid_index_does_not_construct(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  if (detail::create_plugin_instance_for<InstrumentedProcessor>(
          std::numeric_limits<std::uint32_t>::max(), host) != nullptr) {
    return 10;
  }
  if (counter(InstrumentedProcessor::constructed) != 0 ||
      counter(InstrumentedProcessor::destroyed) != 0 || counter(LifetimeToken::alive) != 0) {
    return 11;
  }
  return 0;
}

int multiple_instances_are_isolated(const clap_host_t* host) {
  InstrumentedProcessor::reset_counters();

  const clap_plugin_t* first = detail::create_plugin_instance_for<InstrumentedProcessor>(0u, host);
  const clap_plugin_t* second = detail::create_plugin_instance_for<InstrumentedProcessor>(0u, host);
  if (first == nullptr || second == nullptr || first == second ||
      first->plugin_data == second->plugin_data) {
    return 12;
  }

  Instance* first_instance = Instance::from_plugin(first);
  Instance* second_instance = Instance::from_plugin(second);
  if (first_instance == nullptr || second_instance == nullptr ||
      first_instance->processor().instance_id == second_instance->processor().instance_id) {
    return 13;
  }

  first_instance->processor().mutable_value = 41;
  second_instance->processor().mutable_value = 7;
  if (first_instance->processor().mutable_value != 41 ||
      second_instance->processor().mutable_value != 7) {
    return 14;
  }

  first->destroy(first);
  if (counter(InstrumentedProcessor::destroyed) != 1 || counter(LifetimeToken::alive) != 1 ||
      second_instance->processor().mutable_value != 7) {
    return 15;
  }
  second->destroy(second);
  if (counter(InstrumentedProcessor::constructed) != 2 ||
      counter(InstrumentedProcessor::destroyed) != 2 || counter(LifetimeToken::alive) != 0) {
    return 16;
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
  result = created_instance_is_owned_and_destructible(host);
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
    return 17;
  }
  return 0;
}
