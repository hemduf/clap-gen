#include <cstdint>
#include <limits>
#include <type_traits>

#include "clapgen_entry.cpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

static_assert(std::is_same_v<decltype(&detail::factory_create_plugin),
                             decltype(clap_plugin_factory_t::create_plugin)>);

namespace {

std::uint32_t host_callback_calls = 0u;

const void* CLAP_ABI host_get_extension(const clap_host_t*, const char*) {
  ++host_callback_calls;
  return nullptr;
}

void CLAP_ABI host_request_restart(const clap_host_t*) { ++host_callback_calls; }

void CLAP_ABI host_request_process(const clap_host_t*) { ++host_callback_calls; }

void CLAP_ABI host_request_callback(const clap_host_t*) { ++host_callback_calls; }

clap_host_t compatible_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen test host";
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

std::uint32_t backend_calls = 0u;
std::uint32_t backend_descriptor_index = std::numeric_limits<std::uint32_t>::max();
const clap_host_t* backend_host = nullptr;
clap_plugin_t backend_plugin{};

const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  ++backend_calls;
  backend_descriptor_index = descriptor_index;
  backend_host = host;
  return &backend_plugin;
}

void reset_backend_spy() {
  backend_calls = 0u;
  backend_descriptor_index = std::numeric_limits<std::uint32_t>::max();
  backend_host = nullptr;
}

} // namespace clapgen::generated::detail

int main() {
  const clap_plugin_factory_t* factory = &detail::generated_plugin_factory;
  if (factory->create_plugin != detail::factory_create_plugin) {
    return 1;
  }
  if (!clap_entry.init("issue61-create-plugin-smoke.clap")) {
    return 2;
  }

  auto host = compatible_host();
  const char* known_id = generated::plugin_descriptors[0]->id;

  detail::reset_backend_spy();
  if (detail::factory_create_plugin(nullptr, &host, known_id) != nullptr ||
      detail::backend_calls != 0u) {
    return 3;
  }

  clap_plugin_factory_t unexpected{};
  detail::reset_backend_spy();
  if (detail::factory_create_plugin(&unexpected, &host, known_id) != nullptr ||
      detail::backend_calls != 0u) {
    return 4;
  }

  detail::reset_backend_spy();
  if (factory->create_plugin(factory, nullptr, known_id) != nullptr ||
      detail::backend_calls != 0u) {
    return 5;
  }

  detail::reset_backend_spy();
  if (factory->create_plugin(factory, &host, nullptr) != nullptr || detail::backend_calls != 0u) {
    return 6;
  }

  auto incompatible = host;
  incompatible.clap_version = clap_version_t{0u, 99u, 99u};
  detail::reset_backend_spy();
  host_callback_calls = 0u;
  if (factory->create_plugin(factory, &incompatible, known_id) != nullptr ||
      detail::backend_calls != 0u || host_callback_calls != 0u) {
    return 7;
  }

  detail::reset_backend_spy();
  host_callback_calls = 0u;
  if (factory->create_plugin(factory, &host, "com.example.unknown") != nullptr ||
      detail::backend_calls != 0u || host_callback_calls != 0u) {
    return 8;
  }

  char copied_id[] = "com.example.entry";
  if (copied_id == known_id) {
    return 9;
  }

  detail::reset_backend_spy();
  host_callback_calls = 0u;
  const clap_plugin_t* plugin = factory->create_plugin(factory, &host, copied_id);
  if (plugin != &detail::backend_plugin || detail::backend_calls != 1u ||
      detail::backend_descriptor_index != 0u || detail::backend_host != &host ||
      host_callback_calls != 0u) {
    return 10;
  }

  clap_entry.deinit();
  return 0;
}
