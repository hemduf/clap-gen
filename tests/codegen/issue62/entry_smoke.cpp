#include <cstdint>
#include <thread>
#include <type_traits>

#include "clapgen_entry.cpp"

namespace detail = clapgen::generated::detail;

static_assert(std::is_same_v<decltype(&detail::entry_init), decltype(clap_plugin_entry_t::init)>);
static_assert(
    std::is_same_v<decltype(&detail::entry_deinit), decltype(clap_plugin_entry_t::deinit)>);
static_assert(std::is_same_v<decltype(&detail::entry_get_factory),
                             decltype(clap_plugin_entry_t::get_factory)>);

int main() {
  if (clap_entry.clap_version.major != CLAP_VERSION.major ||
      clap_entry.clap_version.minor != CLAP_VERSION.minor ||
      clap_entry.clap_version.revision != CLAP_VERSION.revision) {
    return 1;
  }
  if (clap_entry.init != detail::entry_init || clap_entry.deinit != detail::entry_deinit ||
      clap_entry.get_factory != detail::entry_get_factory) {
    return 2;
  }

  const void* expected_factory = &detail::generated_plugin_factory;
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 3;
  }

  clap_entry.deinit();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 4;
  }

  if (!clap_entry.init("entry-smoke.clap")) {
    return 5;
  }

  char copied_factory_id[] = "clap.plugin-factory";
  if (static_cast<const void*>(copied_factory_id) ==
      static_cast<const void*>(CLAP_PLUGIN_FACTORY_ID)) {
    return 6;
  }
  if (clap_entry.get_factory(copied_factory_id) != expected_factory) {
    return 7;
  }
  if (clap_entry.get_factory(nullptr) != nullptr ||
      clap_entry.get_factory("clap.unknown-factory") != nullptr) {
    return 8;
  }

  if (!clap_entry.init("entry-smoke-second.clap")) {
    return 9;
  }
  clap_entry.deinit();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != expected_factory) {
    return 10;
  }
  clap_entry.deinit();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 11;
  }

  clap_entry.deinit();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 12;
  }

  bool thread_init_ok = false;
  std::thread init_thread(
      [&thread_init_ok] { thread_init_ok = clap_entry.init("entry-thread.clap"); });
  init_thread.join();
  if (!thread_init_ok || clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != expected_factory) {
    return 13;
  }

  std::thread deinit_thread([] { clap_entry.deinit(); });
  deinit_thread.join();
  if (clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return 14;
  }

  return 0;
}
