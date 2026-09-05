if(NOT DEFINED CLAPGEN_SOURCE_DIR OR NOT DEFINED CLAPGEN_BINARY_DIR OR NOT DEFINED MODE)
  message(FATAL_ERROR "CLAPGEN_SOURCE_DIR, CLAPGEN_BINARY_DIR and MODE are required")
endif()

file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenPackage.cmake" package_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake" functions_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenConfig.cmake.in" config_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenWarnings.cmake" bootstrap_source)
include("${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake")

set(test_root "${CLAPGEN_BINARY_DIR}/issue9/${MODE}")
file(REMOVE_RECURSE "${test_root}")
file(MAKE_DIRECTORY "${test_root}")

function(require_contains source value context)
  string(FIND "${source}" "${value}" found)
  if(found EQUAL -1)
    message(FATAL_ERROR "${context} is missing `${value}`")
  endif()
endfunction()

function(run_checked)
  execute_process(
    COMMAND ${ARGV}
    RESULT_VARIABLE result
    OUTPUT_VARIABLE output
    ERROR_VARIABLE error
  )
  if(NOT result EQUAL 0)
    string(REPLACE ";" " " command_text "${ARGV}")
    message(FATAL_ERROR
      "command failed (${result}): ${command_text}\nstdout:\n${output}\nstderr:\n${error}"
    )
  endif()
endfunction()

if(MODE STREQUAL "consumer")
  foreach(required IN ITEMS
      "EXPORT_NAME Runtime"
      "ClapGenTargets.cmake"
      "ClapGenConfigVersion.cmake"
      "ClapGenFunctions.cmake")
    require_contains("${package_source}" "${required}" "installed package contract")
  endforeach()
  foreach(required IN ITEMS "ClapGenTargets.cmake" "ClapGenFunctions.cmake" "check_required_components")
    require_contains("${config_source}" "${required}" "package config")
  endforeach()
  require_contains("${bootstrap_source}" "include(ClapGenPackage)" "root CMake bootstrap")

  set(prefix "${test_root}/prefix with spaces")
  run_checked("${CMAKE_COMMAND}" --install "${CLAPGEN_BINARY_DIR}" --prefix "${prefix}")

  set(consumer_source "${test_root}/consumer source")
  set(consumer_build "${test_root}/consumer build")
  file(MAKE_DIRECTORY "${consumer_source}")
  file(WRITE "${consumer_source}/CMakeLists.txt" [=[
cmake_minimum_required(VERSION 3.25)
project(ClapGenConsumer LANGUAGES CXX)
find_package(ClapGen CONFIG REQUIRED)
if(NOT TARGET ClapGen::Runtime)
  message(FATAL_ERROR "installed package did not provide ClapGen::Runtime")
endif()
if(NOT COMMAND clapgen_add_plugin)
  message(FATAL_ERROR "installed package did not provide clapgen_add_plugin()")
endif()
]=])
  run_checked(
    "${CMAKE_COMMAND}"
    -S "${consumer_source}"
    -B "${consumer_build}"
    "-DCMAKE_PREFIX_PATH=${prefix}"
  )
elseif(MODE STREQUAL "incremental")
  foreach(required IN ITEMS
      "OUTPUT \"\${_clapgen_manifest}\""
      "BYPRODUCTS"
      "DEPENDS \"\${_clapgen_metadata}\""
      "DEPFILE \"\${_clapgen_depfile}\""
      "VERBATIM"
      "\${CMAKE_CURRENT_BINARY_DIR}/clapgen/\${target}")
    require_contains("${functions_source}" "${required}" "incremental generation contract")
  endforeach()

  set(project_source "${test_root}/project with spaces")
  set(project_build "${test_root}/build with spaces")
  file(MAKE_DIRECTORY "${project_source}")
  file(WRITE "${project_source}/first plugin.kdl" "clapgen schema=\"1.0.0\"\n")
  file(WRITE "${project_source}/second plugin.kdl" "clapgen schema=\"1.0.0\"\n")
  file(WRITE "${project_source}/CMakeLists.txt" "cmake_minimum_required(VERSION 3.25)\nproject(Issue9Incremental LANGUAGES CXX)\ninclude(\"${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake\")\nclapgen_add_plugin(first METADATA \"\${CMAKE_CURRENT_SOURCE_DIR}/first plugin.kdl\" GENERATOR \"${CMAKE_COMMAND}\")\nclapgen_add_plugin(second METADATA \"\${CMAKE_CURRENT_SOURCE_DIR}/second plugin.kdl\" GENERATOR \"${CMAKE_COMMAND}\")\nget_target_property(first_out first CLAPGEN_OUTPUT_DIR)\nget_target_property(second_out second CLAPGEN_OUTPUT_DIR)\nif(first_out STREQUAL second_out)\n  message(FATAL_ERROR \"multiple targets share a generation directory\")\nendif()\n")
  run_checked("${CMAKE_COMMAND}" -S "${project_source}" -B "${project_build}")
elseif(MODE STREQUAL "cross")
  foreach(required IN ITEMS
      "CMAKE_CROSSCOMPILING"
      "CLAPGEN_HOST_EXECUTABLE"
      "get_target_property(_clapgen_imported ClapGen::clapgen IMPORTED)"
      "Cross-compiling requires a host-runnable clapgen")
    require_contains("${functions_source}" "${required}" "cross-compilation contract")
  endforeach()

  set(project_source "${test_root}/cross source")
  set(project_build "${test_root}/cross build")
  file(MAKE_DIRECTORY "${project_source}")
  file(WRITE "${project_source}/plugin.kdl" "clapgen schema=\"1.0.0\"\n")
  file(WRITE "${project_source}/CMakeLists.txt" "cmake_minimum_required(VERSION 3.25)\nproject(Issue9Cross LANGUAGES CXX)\nset(CLAPGEN_HOST_EXECUTABLE \"${CMAKE_COMMAND}\")\ninclude(\"${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake\")\nclapgen_add_plugin(cross_plugin METADATA \"\${CMAKE_CURRENT_SOURCE_DIR}/plugin.kdl\")\n")
  file(WRITE "${project_source}/toolchain.cmake" "set(CMAKE_SYSTEM_NAME Generic)\n")
  run_checked(
    "${CMAKE_COMMAND}"
    -S "${project_source}"
    -B "${project_build}"
    "-DCMAKE_TOOLCHAIN_FILE=${project_source}/toolchain.cmake"
  )
else()
  message(FATAL_ERROR "unknown MODE `${MODE}`")
endif()
