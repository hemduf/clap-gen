#include <clap/clap.h>

#include <cstdint>
#include <cstring>

#include "bundle_processors.hpp"
#include "clapgen_entry.cpp"

namespace generated = clapgen::generated;
namespace detail = clapgen::generated::detail;
namespace issue52 = clapgen::issue52;

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
  host.name = "clap-gen issue52 host";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  host.get_extension = host_get_extension;
  host.request_restart = host_request_restart;
  host.request_process = host_request_process;
  host.request_callback = host_request_callback;
  return host;
}

bool descriptor_id_is(const clap_plugin_factory_t* factory, std::uint32_t index,
                      const char* expected) {
  const clap_plugin_descriptor_t* descriptor = factory->get_plugin_descriptor(factory, index);
  return descriptor != nullptr && descriptor->id != nullptr &&
         std::strcmp(descriptor->id, expected) == 0;
}

bool activate_and_start(const clap_plugin_t* plugin) {
  return plugin->activate(plugin, 48'000.0, 16u, 1024u) && plugin->start_processing(plugin);
}

void stop_deactivate_destroy(const clap_plugin_t* plugin) {
  plugin->stop_processing(plugin);
  plugin->deactivate(plugin);
  plugin->destroy(plugin);
}

} // namespace

int main() {
  issue52::reset_counters();
  host_callback_calls = 0u;

  if (!clap_entry.init("issue52-multi-plugin-bundle.clap")) {
    return 1;
  }
  const auto* factory = static_cast<const clap_plugin_factory_t*>(
      clap_entry.get_factory(CLAP_PLUGIN_FACTORY_ID));
  if (factory == nullptr) {
    return 2;
  }

  if (factory->get_plugin_count(factory) != 3u ||
      !descriptor_id_is(factory, 0u, "com.example.alpha") ||
      !descriptor_id_is(factory, 1u, "com.example.fail") ||
      !descriptor_id_is(factory, 2u, "com.example.zeta")) {
    return 3;
  }
  if (factory->get_plugin_descriptor(factory, 0u) !=
          factory->get_plugin_descriptor(factory, 0u) ||
      factory->get_plugin_descriptor(factory, 3u) != nullptr) {
    return 4;
  }

  auto host = compatible_host();
  if (factory->create_plugin(factory, &host, "com.example.unknown") != nullptr) {
    return 5;
  }

  char alpha_id[] = "com.example.alpha";
  char failing_id[] = "com.example.fail";
  char zeta_id[] = "com.example.zeta";
  if (alpha_id == generated::plugin_descriptors[0]->id ||
      failing_id == generated::plugin_descriptors[1]->id ||
      zeta_id == generated::plugin_descriptors[2]->id) {
    return 6;
  }

  const clap_plugin_t* alpha_first = factory->create_plugin(factory, &host, alpha_id);
  const clap_plugin_t* alpha_second = factory->create_plugin(factory, &host, alpha_id);
  const clap_plugin_t* failing = factory->create_plugin(factory, &host, failing_id);
  const clap_plugin_t* zeta_first = factory->create_plugin(factory, &host, zeta_id);
  if (alpha_first == nullptr || alpha_second == nullptr || failing == nullptr ||
      zeta_first == nullptr) {
    return 7;
  }
  if (alpha_first == alpha_second || alpha_first->plugin_data == alpha_second->plugin_data) {
    return 8;
  }
  if (alpha_first->desc != generated::plugin_descriptors[0] ||
      alpha_second->desc != generated::plugin_descriptors[0] ||
      failing->desc != generated::plugin_descriptors[1] ||
      zeta_first->desc != generated::plugin_descriptors[2]) {
    return 9;
  }

  if (issue52::counter(issue52::alpha_lifetime.constructed) != 2 ||
      issue52::counter(issue52::failing_lifetime.constructed) != 1 ||
      issue52::counter(issue52::zeta_lifetime.constructed) != 1) {
    return 10;
  }

  if (!alpha_first->init(alpha_first) || !alpha_second->init(alpha_second) ||
      failing->init(failing) || !zeta_first->init(zeta_first)) {
    return 11;
  }
  if (failing->init(failing) || issue52::counter(issue52::failing_init_calls) != 1) {
    return 12;
  }

  failing->destroy(failing);
  if (issue52::counter(issue52::failing_lifetime.destroyed) != 1) {
    return 13;
  }

  const clap_plugin_t* zeta_after_failure = factory->create_plugin(factory, &host, zeta_id);
  if (zeta_after_failure == nullptr || zeta_after_failure->desc != generated::plugin_descriptors[2] ||
      !zeta_after_failure->init(zeta_after_failure)) {
    return 14;
  }
  if (factory->get_plugin_count(factory) != 3u ||
      !descriptor_id_is(factory, 0u, "com.example.alpha") ||
      !descriptor_id_is(factory, 1u, "com.example.fail") ||
      !descriptor_id_is(factory, 2u, "com.example.zeta")) {
    return 15;
  }

  if (!activate_and_start(alpha_first) || !activate_and_start(alpha_second) ||
      !activate_and_start(zeta_first) || !activate_and_start(zeta_after_failure)) {
    return 16;
  }

  clap_process_t process{};
  if (alpha_first->process(alpha_first, &process) != CLAP_PROCESS_CONTINUE ||
      alpha_second->process(alpha_second, &process) != CLAP_PROCESS_CONTINUE ||
      zeta_first->process(zeta_first, &process) != CLAP_PROCESS_CONTINUE_IF_NOT_QUIET ||
      zeta_after_failure->process(zeta_after_failure, &process) !=
          CLAP_PROCESS_CONTINUE_IF_NOT_QUIET) {
    return 17;
  }

  alpha_first->reset(alpha_first);
  if (alpha_first->process(alpha_first, &process) != CLAP_PROCESS_SLEEP ||
      alpha_second->process(alpha_second, &process) != CLAP_PROCESS_CONTINUE) {
    return 18;
  }

  stop_deactivate_destroy(alpha_first);
  stop_deactivate_destroy(zeta_first);
  stop_deactivate_destroy(alpha_second);
  stop_deactivate_destroy(zeta_after_failure);

  if (issue52::counter(issue52::alpha_lifetime.constructed) != 2 ||
      issue52::counter(issue52::alpha_lifetime.destroyed) != 2 ||
      issue52::counter(issue52::failing_lifetime.constructed) != 1 ||
      issue52::counter(issue52::failing_lifetime.destroyed) != 1 ||
      issue52::counter(issue52::zeta_lifetime.constructed) != 2 ||
      issue52::counter(issue52::zeta_lifetime.destroyed) != 2) {
    return 19;
  }
  if (host_callback_calls != 0u) {
    return 20;
  }

  clap_entry.deinit();
  return 0;
}
