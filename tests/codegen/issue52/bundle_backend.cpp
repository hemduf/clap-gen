#include <cstdint>
#include <string_view>

#include "bundle_processors.hpp"
#include "clapgen_instance_backend.hpp"

namespace issue52 = clapgen::issue52;

namespace clapgen::generated::detail {

static_assert(std::string_view{plugin_descriptors[0]->id} == "com.example.alpha");
static_assert(std::string_view{plugin_descriptors[1]->id} == "com.example.fail");
static_assert(std::string_view{plugin_descriptors[2]->id} == "com.example.zeta");

const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  switch (descriptor_index) {
  case 0u:
    return create_plugin_instance_for<issue52::AlphaProcessor>(descriptor_index, host);
  case 1u:
    return create_plugin_instance_for<issue52::FailingProcessor>(descriptor_index, host);
  case 2u:
    return create_plugin_instance_for<issue52::ZetaProcessor>(descriptor_index, host);
  default:
    return nullptr;
  }
}

} // namespace clapgen::generated::detail
