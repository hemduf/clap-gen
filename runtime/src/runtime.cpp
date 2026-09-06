#include <clapgen/runtime.hpp>

#ifndef CLAPGEN_VERSION
#error "CLAPGEN_VERSION must be provided by the build system"
#endif

namespace clapgen {

std::string_view runtime_version() noexcept { return CLAPGEN_VERSION; }

} // namespace clapgen
