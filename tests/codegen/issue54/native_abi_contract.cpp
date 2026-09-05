#include <clap/clap.h>

#include <cstdint>
#include <type_traits>

#include "clapgen_entry.cpp"
#include "clapgen_instance_backend.hpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;

namespace {

struct AbiProcessor {
  bool init() { return true; }
  bool activate(double, std::uint32_t, std::uint32_t) { return true; }
  void deactivate() {}
  bool start_processing() { return true; }
  void stop_processing() {}
  void reset() {}
  clap_process_status process(const clap_process_t*) { return CLAP_PROCESS_CONTINUE; }
};

static_assert(generated::NativeProcessor<AbiProcessor>);

static_assert(std::is_same_v<decltype(&detail::entry_init), decltype(clap_plugin_entry_t::init)>);
static_assert(
    std::is_same_v<decltype(&detail::entry_deinit), decltype(clap_plugin_entry_t::deinit)>);
static_assert(std::is_same_v<decltype(&detail::entry_get_factory),
                             decltype(clap_plugin_entry_t::get_factory)>);
static_assert(std::is_same_v<decltype(&detail::factory_get_plugin_count),
                             decltype(clap_plugin_factory_t::get_plugin_count)>);
static_assert(std::is_same_v<decltype(&detail::factory_get_plugin_descriptor),
                             decltype(clap_plugin_factory_t::get_plugin_descriptor)>);
static_assert(std::is_same_v<decltype(&detail::factory_create_plugin),
                             decltype(clap_plugin_factory_t::create_plugin)>);

using NativeCreateFunction = const clap_plugin_t* (*)(std::uint32_t, const clap_host_t*);
static_assert(std::is_same_v<decltype(&detail::create_plugin_instance_for<AbiProcessor>),
                             NativeCreateFunction>);

[[maybe_unused]] constexpr auto instantiate_native_backend =
    &detail::create_plugin_instance_for<AbiProcessor>;

} // namespace
