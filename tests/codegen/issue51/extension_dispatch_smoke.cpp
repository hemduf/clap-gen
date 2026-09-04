#include <clap/clap.h>
#include <clap/ext/latency.h>
#include <clap/ext/tail.h>

#include <cstdint>
#include <type_traits>

#include "clapgen_instance_backend.hpp"

namespace detail = clapgen::generated::detail;

namespace {

std::uint32_t CLAP_ABI latency_get(const clap_plugin_t*) { return 17u; }
std::uint32_t CLAP_ABI tail_get(const clap_plugin_t*) { return 23u; }

static_assert(std::is_same_v<decltype(&latency_get), decltype(clap_plugin_latency_t::get)>);
static_assert(std::is_same_v<decltype(&tail_get), decltype(clap_plugin_tail_t::get)>);

constinit const clap_plugin_latency_t latency_table{
    .get = latency_get,
};
constinit const clap_plugin_tail_t tail_table{
    .get = tail_get,
};

constinit const detail::PluginExtensionBinding test_bindings[] = {
    {CLAP_EXT_TAIL, &tail_table},
    {CLAP_EXT_LATENCY, &latency_table},
};

struct Processor {
  bool init() {
    ++calls;
    return true;
  }
  bool activate(double, std::uint32_t, std::uint32_t) {
    ++calls;
    return true;
  }
  void deactivate() { ++calls; }
  bool start_processing() {
    ++calls;
    return true;
  }
  void stop_processing() { ++calls; }
  void reset() { ++calls; }
  clap_process_status process(const clap_process_t*) {
    ++calls;
    return CLAP_PROCESS_CONTINUE;
  }

  int calls = 0;
};

clap_host_t make_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen issue51 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  return host;
}

int lookup_native_tables_by_string_content() {
  if (detail::lookup_plugin_extension(test_bindings, 2u, nullptr) != nullptr) {
    return 1;
  }
  if (detail::lookup_plugin_extension(nullptr, 1u, CLAP_EXT_LATENCY) != nullptr) {
    return 2;
  }
  if (detail::lookup_plugin_extension(test_bindings, 2u, "com.example.unknown") != nullptr) {
    return 3;
  }

  char copied_latency[] = "clap.latency";
  char copied_tail[] = "clap.tail";
  const void* latency = detail::lookup_plugin_extension(test_bindings, 2u, copied_latency);
  const void* tail = detail::lookup_plugin_extension(test_bindings, 2u, copied_tail);
  if (latency != &latency_table || tail != &tail_table) {
    return 4;
  }
  if (detail::lookup_plugin_extension(test_bindings, 2u, copied_latency) != latency) {
    return 5;
  }
  return 0;
}

int production_plugin_exposes_no_unowned_extension() {
  const auto host = make_host();
  const clap_plugin_t* plugin = detail::create_plugin_instance_for<Processor>(0u, &host);
  if (plugin == nullptr || plugin->get_extension == nullptr) {
    return 6;
  }

  char copied_latency[] = "clap.latency";
  char copied_tail[] = "clap.tail";
  if (plugin->get_extension(plugin, copied_latency) != nullptr ||
      plugin->get_extension(plugin, copied_tail) != nullptr ||
      plugin->get_extension(plugin, nullptr) != nullptr) {
    plugin->destroy(plugin);
    return 7;
  }

  plugin->destroy(plugin);
  return 0;
}

} // namespace

int main() {
  if (const int result = lookup_native_tables_by_string_content(); result != 0) {
    return result;
  }
  return production_plugin_exposes_no_unowned_extension();
}
