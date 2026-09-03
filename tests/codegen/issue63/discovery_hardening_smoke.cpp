#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <limits>
#include <new>
#include <thread>
#include <vector>

#include "clapgen_entry.cpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

namespace {

std::atomic<bool> track_allocations{false};
std::atomic<std::uint64_t> allocation_count{0u};
std::atomic<std::uint64_t> host_callback_calls{0u};

clap_host_t compatible_host();

} // namespace

void* operator new(std::size_t size) {
  if (track_allocations.load(std::memory_order_relaxed)) {
    allocation_count.fetch_add(1u, std::memory_order_relaxed);
  }
  if (void* memory = std::malloc(size)) {
    return memory;
  }
  throw std::bad_alloc{};
}

void* operator new[](std::size_t size) {
  if (track_allocations.load(std::memory_order_relaxed)) {
    allocation_count.fetch_add(1u, std::memory_order_relaxed);
  }
  if (void* memory = std::malloc(size)) {
    return memory;
  }
  throw std::bad_alloc{};
}

void operator delete(void* memory) noexcept { std::free(memory); }
void operator delete[](void* memory) noexcept { std::free(memory); }
void operator delete(void* memory, std::size_t) noexcept { std::free(memory); }
void operator delete[](void* memory, std::size_t) noexcept { std::free(memory); }

namespace {

const void* CLAP_ABI host_get_extension(const clap_host_t*, const char*) {
  host_callback_calls.fetch_add(1u, std::memory_order_relaxed);
  return nullptr;
}

void CLAP_ABI host_request_restart(const clap_host_t*) {
  host_callback_calls.fetch_add(1u, std::memory_order_relaxed);
}

void CLAP_ABI host_request_process(const clap_host_t*) {
  host_callback_calls.fetch_add(1u, std::memory_order_relaxed);
}

void CLAP_ABI host_request_callback(const clap_host_t*) {
  host_callback_calls.fetch_add(1u, std::memory_order_relaxed);
}

clap_host_t compatible_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen hardening host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  host.get_extension = host_get_extension;
  host.request_restart = host_request_restart;
  host.request_process = host_request_process;
  host.request_callback = host_request_callback;
  return host;
}

} // namespace

namespace clapgen::generated::detail {

std::atomic<std::uint64_t> backend_calls{0u};
clap_plugin_t backend_plugin{};

const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  backend_calls.fetch_add(1u, std::memory_order_relaxed);
  if (descriptor_index != 0u || host == nullptr) {
    return nullptr;
  }
  return &backend_plugin;
}

} // namespace clapgen::generated::detail

int main() {
  const auto* direct_factory = &detail::generated_plugin_factory;
  const char* known_id = generated::plugin_descriptors[0]->id;
  auto host = compatible_host();

  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 1;
  }
  if (direct_factory->get_plugin_count(direct_factory) != 0u ||
      direct_factory->get_plugin_descriptor(direct_factory, 0u) != nullptr ||
      direct_factory->create_plugin(direct_factory, &host, known_id) != nullptr) {
    return 2;
  }

  if (!clap_entry.init("discovery-hardening.clap")) {
    return 3;
  }
  const auto* factory =
      static_cast<const clap_plugin_factory_t*>(clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID));
  if (factory != direct_factory) {
    return 4;
  }

  clap_plugin_factory_t unexpected{};
  if (detail::factory_get_plugin_count(nullptr) != 0u ||
      detail::factory_get_plugin_count(&unexpected) != 0u ||
      detail::factory_get_plugin_descriptor(nullptr, 0u) != nullptr ||
      detail::factory_get_plugin_descriptor(&unexpected, 0u) != nullptr ||
      detail::factory_create_plugin(nullptr, &host, known_id) != nullptr ||
      detail::factory_create_plugin(&unexpected, &host, known_id) != nullptr) {
    return 5;
  }

  const std::uint32_t count = factory->get_plugin_count(factory);
  if (count != generated::plugin_descriptor_count || count == 0u) {
    return 6;
  }
  if (factory->get_plugin_descriptor(factory, 0u) != generated::plugin_descriptors[0] ||
      factory->get_plugin_descriptor(factory, count) != nullptr ||
      factory->get_plugin_descriptor(factory, count + 1u) != nullptr ||
      factory->get_plugin_descriptor(factory, std::numeric_limits<std::uint32_t>::max()) !=
          nullptr) {
    return 7;
  }

  allocation_count.store(0u, std::memory_order_relaxed);
  host_callback_calls.store(0u, std::memory_order_relaxed);
  detail::backend_calls.store(0u, std::memory_order_relaxed);
  track_allocations.store(true, std::memory_order_relaxed);
  const void* discovered_factory = clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID);
  const std::uint32_t repeated_count = factory->get_plugin_count(factory);
  const auto* descriptor = factory->get_plugin_descriptor(factory, 0u);
  const auto* unknown = factory->create_plugin(factory, &host, "com.example.unknown");
  const auto* known = factory->create_plugin(factory, &host, known_id);
  track_allocations.store(false, std::memory_order_relaxed);
  if (discovered_factory != factory || repeated_count != count ||
      descriptor != generated::plugin_descriptors[0] || unknown != nullptr ||
      known != &detail::backend_plugin || allocation_count.load(std::memory_order_relaxed) != 0u ||
      host_callback_calls.load(std::memory_order_relaxed) != 0u) {
    return 8;
  }

  std::atomic<bool> concurrent_failure{false};
  constexpr std::size_t thread_count = 8u;
  constexpr std::size_t iterations = 2000u;
  std::vector<std::thread> threads;
  threads.reserve(thread_count);
  for (std::size_t thread_index = 0; thread_index < thread_count; ++thread_index) {
    threads.emplace_back([&] {
      for (std::size_t iteration = 0; iteration < iterations; ++iteration) {
        if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != factory ||
            factory->get_plugin_count(factory) != count ||
            factory->get_plugin_descriptor(factory, 0u) != generated::plugin_descriptors[0] ||
            factory->get_plugin_descriptor(factory,
                                           std::numeric_limits<std::uint32_t>::max()) != nullptr ||
            factory->create_plugin(factory, &host, "com.example.unknown") != nullptr ||
            factory->create_plugin(factory, &host, known_id) != &detail::backend_plugin) {
          concurrent_failure.store(true, std::memory_order_relaxed);
          return;
        }
      }
    });
  }
  for (auto& thread : threads) {
    thread.join();
  }
  if (concurrent_failure.load(std::memory_order_relaxed) ||
      host_callback_calls.load(std::memory_order_relaxed) != 0u) {
    return 9;
  }

  const std::uint64_t backend_before_deinit = detail::backend_calls.load(std::memory_order_relaxed);
  clap_entry.deinit();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 10;
  }
  if (factory->get_plugin_count(factory) != 0u ||
      factory->get_plugin_descriptor(factory, 0u) != nullptr ||
      factory->create_plugin(factory, &host, known_id) != nullptr ||
      detail::backend_calls.load(std::memory_order_relaxed) != backend_before_deinit) {
    return 11;
  }

  return 0;
}
