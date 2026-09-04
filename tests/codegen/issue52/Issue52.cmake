set(CLAPGEN_ISSUE52_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated/issue52")
set(CLAPGEN_ISSUE52_ENTRY "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_entry.cpp")
set(
  CLAPGEN_ISSUE52_BACKEND_HEADER
  "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_instance_backend.hpp"
)
set(
  CLAPGEN_ISSUE52_BACKEND_SOURCE
  "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_instance_backend.cpp"
)
set(
  CLAPGEN_ISSUE52_DESCRIPTORS
  "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_descriptors.hpp"
)
set(CLAPGEN_ISSUE52_EXTENSIONS "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_extensions.hpp")
set(CLAPGEN_ISSUE52_PROCESSOR "${CLAPGEN_ISSUE52_GENERATED_DIR}/clapgen_processor.hpp")

add_custom_command(
  OUTPUT
    "${CLAPGEN_ISSUE52_ENTRY}"
    "${CLAPGEN_ISSUE52_BACKEND_HEADER}"
    "${CLAPGEN_ISSUE52_BACKEND_SOURCE}"
    "${CLAPGEN_ISSUE52_DESCRIPTORS}"
    "${CLAPGEN_ISSUE52_EXTENSIONS}"
    "${CLAPGEN_ISSUE52_PROCESSOR}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${CLAPGEN_ISSUE52_GENERATED_DIR}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different "${CLAPGEN_ISSUE59_ENTRY}"
    "${CLAPGEN_ISSUE52_ENTRY}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different "${CLAPGEN_ISSUE59_BACKEND_HEADER}"
    "${CLAPGEN_ISSUE52_BACKEND_HEADER}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different "${CLAPGEN_ISSUE59_BACKEND_SOURCE}"
    "${CLAPGEN_ISSUE52_BACKEND_SOURCE}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue52/clapgen_descriptors.hpp"
    "${CLAPGEN_ISSUE52_DESCRIPTORS}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different "${CLAPGEN_ISSUE59_EXTENSIONS}"
    "${CLAPGEN_ISSUE52_EXTENSIONS}"
  COMMAND
    "${CMAKE_COMMAND}" -E copy_if_different "${CLAPGEN_ISSUE59_PROCESSOR}"
    "${CLAPGEN_ISSUE52_PROCESSOR}"
  DEPENDS
    clapgen_issue59_generated
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue52/clapgen_descriptors.hpp"
  VERBATIM
)
add_custom_target(
  clapgen_issue52_generated
  DEPENDS
    "${CLAPGEN_ISSUE52_ENTRY}"
    "${CLAPGEN_ISSUE52_BACKEND_HEADER}"
    "${CLAPGEN_ISSUE52_BACKEND_SOURCE}"
    "${CLAPGEN_ISSUE52_DESCRIPTORS}"
    "${CLAPGEN_ISSUE52_EXTENSIONS}"
    "${CLAPGEN_ISSUE52_PROCESSOR}"
)

add_executable(
  clapgen_issue52_multi_plugin_bundle_smoke
  tests/codegen/issue52/multi_plugin_bundle_smoke.cpp
  "${CLAPGEN_ISSUE52_BACKEND_SOURCE}"
)
add_dependencies(clapgen_issue52_multi_plugin_bundle_smoke clapgen_issue52_generated)
target_compile_features(clapgen_issue52_multi_plugin_bundle_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue52_multi_plugin_bundle_smoke
  PRIVATE
    "${CLAPGEN_ISSUE52_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
if(
  UNIX
  AND CMAKE_BUILD_TYPE STREQUAL "Debug"
  AND CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang|AppleClang"
)
  target_compile_options(
    clapgen_issue52_multi_plugin_bundle_smoke
    PRIVATE -fsanitize=address,undefined -fno-omit-frame-pointer
  )
  target_link_options(
    clapgen_issue52_multi_plugin_bundle_smoke
    PRIVATE -fsanitize=address,undefined
  )
endif()
clapgen_enable_warnings(clapgen_issue52_multi_plugin_bundle_smoke)
add_test(
  NAME clapgen.codegen.issue52.multi_plugin_bundle_smoke
  COMMAND clapgen_issue52_multi_plugin_bundle_smoke
)
