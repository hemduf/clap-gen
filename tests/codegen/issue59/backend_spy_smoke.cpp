#include <cstdint>

#include "clapgen_instance_backend.hpp"

namespace {
std::uint32_t captured_index = 0;
const clap_host_t* captured_host = nullptr;
const clap_plugin_t sentinel_plugin{};
} // namespace

namespace clapgen::generated::detail {
const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  captured_index = descriptor_index;
  captured_host = host;
  return &sentinel_plugin;
}
} // namespace clapgen::generated::detail

int main() {
  clap_host_t host{};
  const clap_plugin_t* result = clapgen::generated::detail::create_plugin_instance(7u, &host);
  return result == &sentinel_plugin && captured_index == 7u && captured_host == &host ? 0 : 1;
}
