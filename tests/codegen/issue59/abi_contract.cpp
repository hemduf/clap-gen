#include <type_traits>

#include "clapgen_entry.cpp"

namespace detail = clapgen::generated::detail;

static_assert(
    std::is_same_v<decltype(&detail::entry_init), decltype(clap_plugin_entry_t::init)>);
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
