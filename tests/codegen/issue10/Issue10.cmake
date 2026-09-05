set(CLAPGEN_ISSUE10_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated/issue10")
set(CLAPGEN_ISSUE10_EXTENSIONS "${CLAPGEN_ISSUE10_GENERATED_DIR}/clapgen_extensions.hpp")
set(
  CLAPGEN_ISSUE10_BACKEND_HEADER
  "${CLAPGEN_ISSUE10_GENERATED_DIR}/clapgen_instance_backend.hpp"
)
set(
  CLAPGEN_ISSUE10_DESCRIPTORS
  "${CLAPGEN_ISSUE10_GENERATED_DIR}/clapgen_descriptors.hpp"
)
set(CLAPGEN_ISSUE10_PROCESSOR "${CLAPGEN_ISSUE10_GENERATED_DIR}/clapgen_processor.hpp")

add_custom_command(
  OUTPUT
    "${CLAPGEN_ISSUE10_EXTENSIONS}"
    "${CLAPGEN_ISSUE10_BACKEND_HEADER}"
    "${CLAPGEN_ISSUE10_DESCRIPTORS}"
    "${CLAPGEN_ISSUE10_PROCESSOR}"
  COMMAND
    "${CLAPGEN_CARGO_EXECUTABLE}" run --quiet -p clapgen-cli -- generate
    --metadata "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue10/plugin.kdl"
    --out "${CLAPGEN_ISSUE10_GENERATED_DIR}"
  DEPENDS
    ${CLAPGEN_CODEGEN_RUST_SOURCES}
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
    "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue10/plugin.kdl"
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue10/plugin.ids.kdl"
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
  VERBATIM
)
add_custom_target(
  clapgen_issue10_generated
  DEPENDS
    "${CLAPGEN_ISSUE10_EXTENSIONS}"
    "${CLAPGEN_ISSUE10_BACKEND_HEADER}"
    "${CLAPGEN_ISSUE10_DESCRIPTORS}"
    "${CLAPGEN_ISSUE10_PROCESSOR}"
)

add_executable(
  clapgen_issue10_params_state_smoke
  tests/codegen/issue10/params_state_smoke.cpp
)
add_dependencies(clapgen_issue10_params_state_smoke clapgen_issue10_generated)
target_compile_features(clapgen_issue10_params_state_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue10_params_state_smoke
  PRIVATE
    "${CLAPGEN_ISSUE10_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
clapgen_enable_warnings(clapgen_issue10_params_state_smoke)
add_test(
  NAME clapgen.codegen.issue10.params_state_smoke
  COMMAND clapgen_issue10_params_state_smoke
)
