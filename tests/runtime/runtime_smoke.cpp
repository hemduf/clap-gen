#include <clapgen/runtime.hpp>

#include <string_view>

int main() {
  using namespace std::literals;
  return clapgen::runtime_version() == "0.1.0"sv ? 0 : 1;
}
