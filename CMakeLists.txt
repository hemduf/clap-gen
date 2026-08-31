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
endif()
