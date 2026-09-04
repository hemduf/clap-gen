#include <clap/clap.h>

#include <atomic>
#include <cstdint>
#include <stdexcept>

#include "clapgen_entry.cpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

namespace {

enum class ThrowSite {
  None,
  Init,
  Activate,
  Deactivate,
  Start,
  Stop,
  Reset,
  Process,
  Destroy,
};

struct FaultProcessor {
  ~FaultProcessor() noexcept(false) {
    destructor_calls.fetch_add(1, std::memory_order_relaxed);
    maybe_throw(ThrowSite::Destroy);
  }

  bool init() {
    ++init_calls;
    maybe_throw(ThrowSite::Init);
    return true;
  }

  bool activate(double sample_rate, std::uint32_t min_frames, std::uint32_t max_frames) {
    ++activate_calls;
    last_sample_rate = sample_rate;
    last_min_frames = min_frames;
    last_max_frames = max_frames;
    maybe_throw(ThrowSite::Activate);
    return true;
  }

  void deactivate() {
    ++deactivate_calls;
    maybe_throw(ThrowSite::Deactivate);
  }

  bool start_processing() {
    ++start_calls;
    maybe_throw(ThrowSite::Start);
    return true;
  }

  void stop_processing() {
    ++stop_calls;
    maybe_throw(ThrowSite::Stop);
  }

  void reset() {
    ++reset_calls;
    maybe_throw(ThrowSite::Reset);
  }

  clap_process_status process(const clap_process_t* process_block) {
    ++process_calls;
    last_process = process_block;
    maybe_throw(ThrowSite::Process);
    return CLAP_PROCESS_CONTINUE;
  }

  void maybe_throw(ThrowSite site) const {
    if (throw_site == site) {
      throw std::runtime_error("clap-gen issue50 injected failure");
    }
  }

  ThrowSite throw_site = ThrowSite::None;
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

  static inline std::atomic<int> destructor_calls{0};
};

static_assert(generated::NativeProcessor<FaultProcessor>);
using Instance = detail::PluginInstance<FaultProcessor>;

clap_host_t make_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen issue50 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  return host;
}

const clap_plugin_t* make_plugin(const clap_host_t* host) {
  return detail::create_plugin_instance_for<FaultProcessor>(0u, host);
}

int init_exception_is_terminal(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr) {
    return 1;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr || instance->host() != host) {
    return 2;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Init;
  bool escaped = false;
  bool first_result = true;
  try {
    first_result = plugin->init(plugin);
  } catch (...) {
    escaped = true;
  }

  processor.throw_site = ThrowSite::None;
  const bool second_result = plugin->init(plugin);
  const int calls = processor.init_calls;
  plugin->destroy(plugin);

  if (escaped || first_result || second_result || calls != 1) {
    return 3;
  }
  return 0;
}

int activate_exception_is_retryable(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin)) {
    return 4;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 5;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Activate;
  bool escaped = false;
  bool first_result = true;
  try {
    first_result = plugin->activate(plugin, 48000.25, 17u, 4096u);
  } catch (...) {
    escaped = true;
  }

  processor.throw_site = ThrowSite::None;
  const bool retry_result = plugin->activate(plugin, 48000.25, 17u, 4096u);
  const int calls = processor.activate_calls;
  const bool arguments_ok = processor.last_sample_rate == 48000.25 &&
                            processor.last_min_frames == 17u && processor.last_max_frames == 4096u;
  plugin->deactivate(plugin);
  plugin->destroy(plugin);

  if (escaped || first_result || !retry_result || calls != 2 || !arguments_ok) {
    return 6;
  }
  return 0;
}

int start_exception_is_retryable(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin) ||
      !plugin->activate(plugin, 48000.0, 1u, 1024u)) {
    return 7;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 8;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Start;
  bool escaped = false;
  bool first_result = true;
  try {
    first_result = plugin->start_processing(plugin);
  } catch (...) {
    escaped = true;
  }

  processor.throw_site = ThrowSite::None;
  const bool retry_result = plugin->start_processing(plugin);
  const int calls = processor.start_calls;
  plugin->stop_processing(plugin);
  plugin->deactivate(plugin);
  plugin->destroy(plugin);

  if (escaped || first_result || !retry_result || calls != 2) {
    return 9;
  }
  return 0;
}

int process_exception_maps_to_clap_error(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin) ||
      !plugin->activate(plugin, 48000.0, 1u, 1024u) || !plugin->start_processing(plugin)) {
    return 10;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 11;
  }

  FaultProcessor& processor = instance->processor();
  clap_process_t process{};
  processor.throw_site = ThrowSite::Process;
  bool escaped = false;
  clap_process_status status = CLAP_PROCESS_CONTINUE;
  try {
    status = plugin->process(plugin, &process);
  } catch (...) {
    escaped = true;
  }

  processor.throw_site = ThrowSite::None;
  const bool pointer_ok = processor.last_process == &process;
  const int calls = processor.process_calls;
  plugin->stop_processing(plugin);
  plugin->deactivate(plugin);
  plugin->destroy(plugin);

  if (escaped || status != CLAP_PROCESS_ERROR || calls != 1 || !pointer_ok) {
    return 12;
  }
  return 0;
}

int reset_exception_stays_active(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin) ||
      !plugin->activate(plugin, 48000.0, 1u, 1024u)) {
    return 13;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 14;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Reset;
  bool escaped = false;
  try {
    plugin->reset(plugin);
  } catch (...) {
    escaped = true;
  }
  processor.throw_site = ThrowSite::None;
  const int calls = processor.reset_calls;
  plugin->deactivate(plugin);
  plugin->destroy(plugin);

  if (escaped || calls != 1) {
    return 15;
  }
  return 0;
}

int stop_exception_keeps_teardown_viable(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin) ||
      !plugin->activate(plugin, 48000.0, 1u, 1024u) || !plugin->start_processing(plugin)) {
    return 16;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 17;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Stop;
  bool escaped = false;
  try {
    plugin->stop_processing(plugin);
  } catch (...) {
    escaped = true;
  }
  processor.throw_site = ThrowSite::None;
  plugin->stop_processing(plugin);
  const int calls = processor.stop_calls;
  plugin->deactivate(plugin);
  plugin->destroy(plugin);

  if (escaped || calls != 1) {
    return 18;
  }
  return 0;
}

int deactivate_exception_keeps_destroy_viable(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr || !plugin->init(plugin) ||
      !plugin->activate(plugin, 48000.0, 1u, 1024u)) {
    return 19;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 20;
  }

  FaultProcessor& processor = instance->processor();
  processor.throw_site = ThrowSite::Deactivate;
  bool escaped = false;
  try {
    plugin->deactivate(plugin);
  } catch (...) {
    escaped = true;
  }
  processor.throw_site = ThrowSite::None;
  plugin->deactivate(plugin);
  const int calls = processor.deactivate_calls;
  plugin->destroy(plugin);

  if (escaped || calls != 1) {
    return 21;
  }
  return 0;
}

int destroy_exception_does_not_escape(const clap_host_t* host) {
  const clap_plugin_t* plugin = make_plugin(host);
  if (plugin == nullptr) {
    return 22;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 23;
  }

  instance->processor().throw_site = ThrowSite::Destroy;
  const int before = FaultProcessor::destructor_calls.load(std::memory_order_relaxed);
  bool escaped = false;
  try {
    plugin->destroy(plugin);
  } catch (...) {
    escaped = true;
  }
  const int after = FaultProcessor::destructor_calls.load(std::memory_order_relaxed);

  if (escaped || after != before + 1) {
    return 24;
  }
  return 0;
}

int factory_backend_exception_maps_to_null(const clap_host_t* host) {
  if (!clap_entry.init("/tmp/clapgen-issue50")) {
    return 25;
  }
  const auto* factory = static_cast<const clap_plugin_factory_t*>(
      clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID));
  if (factory == nullptr) {
    clap_entry.deinit();
    return 26;
  }

  bool escaped = false;
  const clap_plugin_t* plugin = reinterpret_cast<const clap_plugin_t*>(1);
  try {
    plugin = factory->create_plugin(factory, host, generated::plugin_descriptors[0]->id);
  } catch (...) {
    escaped = true;
  }
  clap_entry.deinit();

  if (escaped || plugin != nullptr) {
    return 27;
  }
  return 0;
}

int first_failure(const clap_host_t* host) {
  using Check = int (*)(const clap_host_t*);
  constexpr Check checks[] = {
      init_exception_is_terminal,
      activate_exception_is_retryable,
      start_exception_is_retryable,
      process_exception_maps_to_clap_error,
      reset_exception_stays_active,
      stop_exception_keeps_teardown_viable,
      deactivate_exception_keeps_destroy_viable,
      destroy_exception_does_not_escape,
      factory_backend_exception_maps_to_null,
  };
  for (const auto check : checks) {
    if (const int result = check(host); result != 0) {
      return result;
    }
  }
  return 0;
}

} // namespace

namespace clapgen::generated::detail {

const clap_plugin_t* create_plugin_instance(std::uint32_t, const clap_host_t*) {
  throw std::runtime_error("clap-gen issue50 injected factory failure");
}

} // namespace clapgen::generated::detail

int main() {
  const auto host = make_host();
  return first_failure(&host);
}
