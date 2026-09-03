#include "clapgen_instance_backend.hpp"

int main() {
  clap_host_t host{};
  return clapgen::generated::detail::create_plugin_instance(0u, &host) == nullptr ? 0 : 1;
}
