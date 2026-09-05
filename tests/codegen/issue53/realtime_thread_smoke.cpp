#include <clap/clap.h>
#include <clap/ext/thread-check.h>

#include <atomic>
#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <new>

#include "clapgen_instance_backend.hpp"

namespace {
std::atomic<std::size_t> allocation_count{0};
}

void* operator new(std::size_t size) {
  allocation_count.fetch_add(1u, std::memory_order_relaxed);
  if (void* pointer = std::malloc(size)) {
    return pointer;
  }
  throw std::bad_alloc{};
}

void* operator new[](std::size_t size) {
  allocation_count.fetch_add(1u, std::memory_order_relaxed);
  if (void* pointer = std::malloc(size)) {
    return pointer;
  }
  throw std::bad_alloc{};
}

void operator delete(void* pointer) noexcept { std::free(pointer); }
void operator delete[](void* pointer) noexcept { std::free(pointer); }
void operator delete(void* pointer, std::size_t) noexcept { std::free(pointer); }
void operator delete[](void* pointer, std::size_t) noexcept { std::free(pointer); }

namespace detail = clapgen::generated::detail;

namespace {

enum class ThreadRole { Other, Main, Audio };

thread_local ThreadRole current_role = ThreadRole::Other;
std::atomic<int> extension_queries{0};
std::atomic<int> main_thread_checks{0};
std::atomic<int> audio_thread_checks{0};

bool CLAP_ABI is_main_thread(const clap_host_t*) {
  main_thread_checks.fetch_add(1, std::memory_order_relaxed);
  return current_role == ThreadRole::Main;
}

bool CLAP_ABI is_audio_thread(const clap_host_t*) {
  audio_thread_checks.fetch_add(1, std::memory_order_relaxed);
  return current_role == ThreadRole::Audio;
}

const clap_host_thread_check_t thread_check{
    .is_main_thread = is_main_thread,
    .is_audio_thread = is_audio_thread,
};

const void* CLAP_ABI host_get_extension(const clap_host_t*, const char* extension_id) {
  extension_queries.fetch_add(1, std::memory_order_relaxed);
  if (extension_id != nullptr && std::strcmp(extension_id, CLAP_EXT_THREAD_CHECK) == 0) {
    return &thread_check;
  }
  return nullptr;
}

clap_host_t make_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen issue53 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  host.get_extension = host_get_extension;
  return host;
}

struct RealtimeProcessor {
  ~RealtimeProcessor() { destroyed.fetch_add(1, std::memory_order_relaxed); }

  bool init() {
    ++init_calls;
    return true;
  }
  bool activate(double, std::uint32_t, std::uint32_t) {
    ++activate_calls;
    return true;
  }
  void deactivate() { ++deactivate_calls; }
  bool start_processing() {
    ++start_calls;
    return true;
  }
  void stop_processing() { ++stop_calls; }
  void reset() { ++reset_calls; }
  clap_process_status process(const clap_process_t*) {
    ++process_calls;
    return CLAP_PROCESS_CONTINUE;
  }

  int init_calls = 0;
  int activate_calls = 0;
  int deactivate_calls = 0;
  int start_calls = 0;
  int stop_calls = 0;
  int reset_calls = 0;
  int process_calls = 0;

  static inline std::atomic<int> destroyed{0};
};

using Instance = detail::PluginInstance<RealtimeProcessor>;

bool allocation_count_unchanged(std::size_t before) {
  return allocation_count.load(std::memory_order_relaxed) == before;
}

int legal_realtime_sequence_has_no_generated_allocations(const clap_host_t* host) {
  current_role = ThreadRole::Main;
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<RealtimeProcessor>(0u, host);
  if (plugin == nullptr || !plugin->init(plugin)) {
    return 1;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 2;
  }
  const RealtimeProcessor& processor = instance->processor();
  if (!plugin->activate(plugin, 48000.0, 1u, 1024u)) {
    return 3;
  }

  current_role = ThreadRole::Audio;

  std::size_t before = allocation_count.load(std::memory_order_relaxed);
  plugin->reset(plugin);
  if (!allocation_count_unchanged(before) || processor.reset_calls != 1) {
    return 4;
  }

  before = allocation_count.load(std::memory_order_relaxed);
  if (!plugin->start_processing(plugin) || !allocation_count_unchanged(before) ||
      processor.start_calls != 1) {
    return 5;
  }

  clap_process_t process{};
  before = allocation_count.load(std::memory_order_relaxed);
  if (plugin->process(plugin, &process) != CLAP_PROCESS_CONTINUE ||
      !allocation_count_unchanged(before) || processor.process_calls != 1) {
    return 6;
  }

  before = allocation_count.load(std::memory_order_relaxed);
  plugin->reset(plugin);
  if (!allocation_count_unchanged(before) || processor.reset_calls != 2) {
    return 7;
  }

  before = allocation_count.load(std::memory_order_relaxed);
  plugin->stop_processing(plugin);
  if (!allocation_count_unchanged(before) || processor.stop_calls != 1) {
    return 8;
  }

  current_role = ThreadRole::Main;
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 1) {
    return 9;
  }
  plugin->destroy(plugin);
  return 0;
}

int debug_thread_checks_fail_closed(const clap_host_t* host) {
#ifndef NDEBUG
  current_role = ThreadRole::Main;
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<RealtimeProcessor>(0u, host);
  if (plugin == nullptr || !plugin->init(plugin)) {
    return 10;
  }
  Instance* instance = Instance::from_plugin(plugin);
  if (instance == nullptr) {
    return 11;
  }
  const RealtimeProcessor& processor = instance->processor();

  current_role = ThreadRole::Audio;
  if (plugin->activate(plugin, 44100.0, 1u, 512u) || processor.activate_calls != 0) {
    return 12;
  }

  current_role = ThreadRole::Main;
  if (!plugin->activate(plugin, 44100.0, 1u, 512u) || processor.activate_calls != 1) {
    return 13;
  }
  if (plugin->start_processing(plugin) || processor.start_calls != 0) {
    return 14;
  }

  current_role = ThreadRole::Audio;
  if (!plugin->start_processing(plugin) || processor.start_calls != 1) {
    return 15;
  }

  clap_process_t process{};
  current_role = ThreadRole::Main;
  plugin->reset(plugin);
  if (processor.reset_calls != 0) {
    return 16;
  }
  if (plugin->process(plugin, &process) != CLAP_PROCESS_ERROR || processor.process_calls != 0) {
    return 17;
  }
  plugin->stop_processing(plugin);
  if (processor.stop_calls != 0) {
    return 18;
  }

  current_role = ThreadRole::Audio;
  plugin->stop_processing(plugin);
  if (processor.stop_calls != 1) {
    return 19;
  }
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 0) {
    return 20;
  }

  current_role = ThreadRole::Main;
  plugin->deactivate(plugin);
  if (processor.deactivate_calls != 1) {
    return 21;
  }

  const int destroyed_before = RealtimeProcessor::destroyed.load(std::memory_order_relaxed);
  current_role = ThreadRole::Audio;
  plugin->destroy(plugin);
  if (RealtimeProcessor::destroyed.load(std::memory_order_relaxed) != destroyed_before) {
    return 22;
  }
  current_role = ThreadRole::Main;
  plugin->destroy(plugin);
  if (RealtimeProcessor::destroyed.load(std::memory_order_relaxed) != destroyed_before + 1) {
    return 23;
  }

  if (extension_queries.load(std::memory_order_relaxed) < 2 ||
      main_thread_checks.load(std::memory_order_relaxed) == 0 ||
      audio_thread_checks.load(std::memory_order_relaxed) == 0) {
    return 24;
  }
#else
  if (extension_queries.load(std::memory_order_relaxed) != 0 ||
      main_thread_checks.load(std::memory_order_relaxed) != 0 ||
      audio_thread_checks.load(std::memory_order_relaxed) != 0) {
    return 25;
  }
#endif
  return 0;
}

} // namespace

int main() {
  const clap_host_t host = make_host();
  if (const int result = legal_realtime_sequence_has_no_generated_allocations(&host); result != 0) {
    return result;
  }
  return debug_thread_checks_fail_closed(&host);
}
