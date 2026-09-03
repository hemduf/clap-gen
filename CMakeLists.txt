cmake_minimum_required(VERSION 3.25)

project(
  ClapGen
  VERSION 0.1.0
  DESCRIPTION "Metadata-driven CLAP plugin code generator"
  LANGUAGES CXX
)

option(CLAPGEN_FETCH_CLAP "Fetch the pinned CLAP SDK for targets that need its headers" OFF)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(ClapGenWarnings)

if(CLAPGEN_FETCH_CLAP)
  include(dependencies/Clap)
endif()
add_library(clapgen_runtime STATIC runtime/src/runtime.cpp)
add_library(ClapGen::Runtime ALIAS clapgen_runtime)

target_compile_features(clapgen_runtime PUBLIC cxx_std_20)
target_include_directories(
  clapgen_runtime
  PUBLIC
    "$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/runtime/include>"
    "$<INSTALL_INTERFACE:include>"
)
set_target_properties(
  clapgen_runtime
  PROPERTIES
    CXX_EXTENSIONS OFF
    POSITION_INDEPENDENT_CODE ON
)
clapgen_enable_warnings(clapgen_runtime)

include(CTest)
if(BUILD_TESTING)
  add_executable(clapgen_runtime_smoke tests/runtime/runtime_smoke.cpp)
  target_link_libraries(clapgen_runtime_smoke PRIVATE ClapGen::Runtime)
  clapgen_enable_warnings(clapgen_runtime_smoke)
  add_test(NAME clapgen.runtime.smoke COMMAND clapgen_runtime_smoke)

  if(CLAPGEN_FETCH_CLAP)
    find_program(CLAPGEN_CARGO_EXECUTABLE NAMES cargo REQUIRED)
    set(CLAPGEN_ISSUE59_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated/issue59")
    set(CLAPGEN_ISSUE59_ENTRY "${CLAPGEN_ISSUE59_GENERATED_DIR}/clapgen_entry.cpp")
    set(
      CLAPGEN_ISSUE59_BACKEND_HEADER
      "${CLAPGEN_ISSUE59_GENERATED_DIR}/clapgen_instance_backend.hpp"
    )
    set(
      CLAPGEN_ISSUE59_BACKEND_SOURCE
      "${CLAPGEN_ISSUE59_GENERATED_DIR}/clapgen_instance_backend.cpp"
    )
    set(
      CLAPGEN_ISSUE59_DESCRIPTORS
      "${CLAPGEN_ISSUE59_GENERATED_DIR}/clapgen_descriptors.hpp"
    )

    file(
      GLOB_RECURSE CLAPGEN_CODEGEN_RUST_SOURCES
      CONFIGURE_DEPENDS
      "${CMAKE_CURRENT_SOURCE_DIR}/crates/clapgen-cli/src/*.rs"
    )
    add_custom_command(
      OUTPUT
        "${CLAPGEN_ISSUE59_ENTRY}"
        "${CLAPGEN_ISSUE59_BACKEND_HEADER}"
        "${CLAPGEN_ISSUE59_BACKEND_SOURCE}"
        "${CLAPGEN_ISSUE59_DESCRIPTORS}"
      COMMAND
        "${CLAPGEN_CARGO_EXECUTABLE}" run --quiet -p clapgen-cli -- generate
        --metadata "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue59/plugin.kdl"
        --out "${CLAPGEN_ISSUE59_GENERATED_DIR}"
      DEPENDS
        ${CLAPGEN_CODEGEN_RUST_SOURCES}
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock"
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue59/plugin.kdl"
      WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
      VERBATIM
    )
    add_custom_target(
      clapgen_issue59_generated
      DEPENDS
        "${CLAPGEN_ISSUE59_ENTRY}"
        "${CLAPGEN_ISSUE59_BACKEND_HEADER}"
        "${CLAPGEN_ISSUE59_BACKEND_SOURCE}"
        "${CLAPGEN_ISSUE59_DESCRIPTORS}"
    )

    add_library(clapgen_issue59_entry_abi OBJECT tests/codegen/issue59/abi_contract.cpp)
    add_dependencies(clapgen_issue59_entry_abi clapgen_issue59_generated)
    target_compile_features(clapgen_issue59_entry_abi PRIVATE cxx_std_20)
    target_include_directories(
      clapgen_issue59_entry_abi
      PRIVATE
        "${CLAPGEN_ISSUE59_GENERATED_DIR}"
        "${clap_SOURCE_DIR}/include"
    )
    clapgen_enable_warnings(clapgen_issue59_entry_abi)

    add_executable(
      clapgen_issue59_backend_smoke
      tests/codegen/issue59/backend_smoke.cpp
      "${CLAPGEN_ISSUE59_BACKEND_SOURCE}"
    )
    add_dependencies(clapgen_issue59_backend_smoke clapgen_issue59_generated)
    target_compile_features(clapgen_issue59_backend_smoke PRIVATE cxx_std_20)
    target_include_directories(
      clapgen_issue59_backend_smoke
      PRIVATE
        "${CLAPGEN_ISSUE59_GENERATED_DIR}"
        "${clap_SOURCE_DIR}/include"
    )
    clapgen_enable_warnings(clapgen_issue59_backend_smoke)
    add_test(NAME clapgen.codegen.issue59.backend_smoke COMMAND clapgen_issue59_backend_smoke)

    add_executable(clapgen_issue59_backend_spy_smoke tests/codegen/issue59/backend_spy_smoke.cpp)
    add_dependencies(clapgen_issue59_backend_spy_smoke clapgen_issue59_generated)
    target_compile_features(clapgen_issue59_backend_spy_smoke PRIVATE cxx_std_20)
    target_include_directories(
      clapgen_issue59_backend_spy_smoke
      PRIVATE
        "${CLAPGEN_ISSUE59_GENERATED_DIR}"
        "${clap_SOURCE_DIR}/include"
    )
    clapgen_enable_warnings(clapgen_issue59_backend_spy_smoke)
    add_test(
      NAME clapgen.codegen.issue59.backend_spy_smoke
      COMMAND clapgen_issue59_backend_spy_smoke
    )

    add_executable(
      clapgen_issue60_factory_smoke
      tests/codegen/issue60/factory_smoke.cpp
      "${CLAPGEN_ISSUE59_BACKEND_SOURCE}"
    )
    add_dependencies(clapgen_issue60_factory_smoke clapgen_issue59_generated)
    target_compile_features(clapgen_issue60_factory_smoke PRIVATE cxx_std_20)
    target_include_directories(
      clapgen_issue60_factory_smoke
      PRIVATE
        "${CLAPGEN_ISSUE59_GENERATED_DIR}"
        "${clap_SOURCE_DIR}/include"
    )
    clapgen_enable_warnings(clapgen_issue60_factory_smoke)
    add_test(NAME clapgen.codegen.issue60.factory_smoke COMMAND clapgen_issue60_factory_smoke)

    add_executable(
      clapgen_issue61_create_plugin_spy_smoke
      tests/codegen/issue61/create_plugin_spy_smoke.cpp
    )
    add_dependencies(clapgen_issue61_create_plugin_spy_smoke clapgen_issue59_generated)
    target_compile_features(clapgen_issue61_create_plugin_spy_smoke PRIVATE cxx_std_20)
    target_include_directories(
      clapgen_issue61_create_plugin_spy_smoke
      PRIVATE
        "${CLAPGEN_ISSUE59_GENERATED_DIR}"
        "${clap_SOURCE_DIR}/include"
    )
    clapgen_enable_warnings(clapgen_issue61_create_plugin_spy_smoke)
    add_test(
      NAME clapgen.codegen.issue61.create_plugin_spy_smoke
      COMMAND clapgen_issue61_create_plugin_spy_smoke
    )
  endif()
endif()
