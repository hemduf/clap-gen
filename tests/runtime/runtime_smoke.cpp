#include <clapgen/runtime.hpp>

#include <string_view>

#ifndef CLAPGEN_EXPECTED_VERSION
#error "CLAPGEN_EXPECTED_VERSION must be provided by the build system"
#endif

int main() {
  using namespace std::literals;
  return clapgen::runtime_version() == std::string_view{CLAPGEN_EXPECTED_VERSION} ? 0 : 1;
}
