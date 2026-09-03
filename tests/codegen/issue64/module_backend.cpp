#include <cstdint>

#include "clapgen_descriptors.hpp"
#include "clapgen_instance_backend.hpp"

namespace clapgen::generated::detail {

clap_plugin_t test_module_plugin{};

const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  if (host == nullptr || descriptor_index >= plugin_descriptor_count) {
    return nullptr;
  }

  test_module_plugin.desc = plugin_descriptors[descriptor_index];
  test_module_plugin.plugin_data = const_cast<clap_host_t*>(host);
  return &test_module_plugin;
}

} // namespace clapgen::generated::detail
