#include <cstdint>
#include <initializer_list>
#include <limits>
#include <type_traits>

#include "clapgen_entry.cpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

static_assert(std::is_same_v<decltype(&detail::factory_get_plugin_count),
                             decltype(clap_plugin_factory_t::get_plugin_count)>);
static_assert(std::is_same_v<decltype(&detail::factory_get_plugin_descriptor),
                             decltype(clap_plugin_factory_t::get_plugin_descriptor)>);

int main() {
  const clap_plugin_factory_t* factory = &detail::generated_plugin_factory;

  if (factory->get_plugin_count != detail::factory_get_plugin_count) {
    return 1;
  }
  if (factory->get_plugin_descriptor != detail::factory_get_plugin_descriptor) {
    return 2;
  }
  if (!clap_entry.init("issue60-factory-smoke.clap")) {
    return 3;
  }

  const auto count = factory->get_plugin_count(factory);
  if (count != generated::plugin_descriptor_count || count == 0u) {
    return 4;
  }

  if (factory->get_plugin_descriptor(factory, 0u) != generated::plugin_descriptors[0]) {
    return 5;
  }
  if (factory->get_plugin_descriptor(factory, count - 1u) !=
      generated::plugin_descriptors[count - 1u]) {
    return 6;
  }

  for (const std::uint32_t invalid :
       {count, count + 1u, std::numeric_limits<std::uint32_t>::max()}) {
    if (factory->get_plugin_descriptor(factory, invalid) != nullptr) {
      return 7;
    }
  }

  clap_plugin_factory_t unexpected{};
  if (detail::factory_get_plugin_count(nullptr) != 0u ||
      detail::factory_get_plugin_count(&unexpected) != 0u) {
    return 8;
  }
  if (detail::factory_get_plugin_descriptor(nullptr, 0u) != nullptr ||
      detail::factory_get_plugin_descriptor(&unexpected, 0u) != nullptr) {
    return 9;
  }

  clap_entry.deinit();
  return 0;
}
