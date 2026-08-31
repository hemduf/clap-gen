#pragma once

#include <string_view>

namespace clapgen {

[[nodiscard]] std::string_view runtime_version() noexcept;

} // namespace clapgen
