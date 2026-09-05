if(NOT DEFINED CLAPGEN_SOURCE_DIR OR NOT DEFINED CLAPGEN_BINARY_DIR)
  message(FATAL_ERROR "CLAPGEN_SOURCE_DIR and CLAPGEN_BINARY_DIR are required")
endif()

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

function(run_and_require_regeneration expected_text)
  execute_process(
    COMMAND ${ARGN}
    RESULT_VARIABLE result
    OUTPUT_VARIABLE output
    ERROR_VARIABLE error
  )
  if(NOT result EQUAL 0)
    string(REPLACE ";" " " command_text "${ARGN}")
    message(FATAL_ERROR
      "command failed (${result}): ${command_text}\nstdout:\n${output}\nstderr:\n${error}"
    )
  endif()
  string(FIND "${output}\n${error}" "${expected_text}" found)
  if(found EQUAL -1)
    message(FATAL_ERROR
      "dependency edit did not trigger `${expected_text}`\nstdout:\n${output}\nstderr:\n${error}"
    )
  endif()
endfunction()

find_program(CARGO_EXECUTABLE NAMES cargo REQUIRED)
run_checked(
  "${CARGO_EXECUTABLE}"
  build
  --quiet
  --locked
  --manifest-path "${CLAPGEN_SOURCE_DIR}/Cargo.toml"
  -p clapgen-cli
)

if(WIN32)
  set(clapgen_executable "${CLAPGEN_SOURCE_DIR}/target/debug/clapgen.exe")
else()
  set(clapgen_executable "${CLAPGEN_SOURCE_DIR}/target/debug/clapgen")
endif()
if(NOT EXISTS "${clapgen_executable}")
  message(FATAL_ERROR "cargo did not produce the expected host clapgen: ${clapgen_executable}")
endif()

set(test_root "${CLAPGEN_BINARY_DIR}/issue9/real codegen")
set(project_source "${test_root}/source with spaces")
set(project_build "${test_root}/build with spaces")
file(REMOVE_RECURSE "${test_root}")
file(MAKE_DIRECTORY "${project_source}/shared" "${project_source}/assets")
configure_file(
  "${CLAPGEN_SOURCE_DIR}/tests/codegen/issue59/plugin.kdl"
  "${project_source}/first plugin.kdl"
  COPYONLY
)
configure_file(
  "${CLAPGEN_SOURCE_DIR}/tests/codegen/issue59/plugin.kdl"
  "${project_source}/second plugin.kdl"
  COPYONLY
)
file(WRITE "${project_source}/shared/common.kdl" "clapgen schema=\"1.0.0\"\n")
file(WRITE "${project_source}/assets/view file.html" "<main>v1</main>\n")

file(READ "${project_source}/first plugin.kdl" first_metadata)
string(
  REPLACE
  "clapgen schema=\"1.0.0\"\n"
  "clapgen schema=\"1.0.0\"\nimport \"shared/common.kdl\"\n"
  first_metadata
  "${first_metadata}"
)
file(WRITE "${project_source}/first plugin.kdl" "${first_metadata}")

file(READ "${project_source}/second plugin.kdl" second_metadata)
string(
  REPLACE
  "gui {}"
  "gui {\n    resource \"assets/view file.html\" mime=\"text/html\"\n}"
  second_metadata
  "${second_metadata}"
)
file(WRITE "${project_source}/second plugin.kdl" "${second_metadata}")

file(WRITE "${project_source}/CMakeLists.txt" "cmake_minimum_required(VERSION 3.25)\nproject(Issue9RealCodegen LANGUAGES CXX)\ninclude(\"${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake\")\nclapgen_add_plugin(first METADATA \"\${CMAKE_CURRENT_SOURCE_DIR}/first plugin.kdl\" GENERATOR \"${clapgen_executable}\")\nclapgen_add_plugin(second METADATA \"\${CMAKE_CURRENT_SOURCE_DIR}/second plugin.kdl\" GENERATOR \"${clapgen_executable}\")\nget_target_property(first_out first CLAPGEN_OUTPUT_DIR)\nget_target_property(second_out second CLAPGEN_OUTPUT_DIR)\nif(first_out STREQUAL second_out)\n  message(FATAL_ERROR \"multiple targets share a generation directory\")\nendif()\n")

run_checked("${CMAKE_COMMAND}" -S "${project_source}" -B "${project_build}")
set(build_config_args)
if(DEFINED CLAPGEN_CONFIG AND NOT CLAPGEN_CONFIG STREQUAL "")
  list(APPEND build_config_args --config "${CLAPGEN_CONFIG}")
endif()
set(first_build_command
  "${CMAKE_COMMAND}" --build "${project_build}" ${build_config_args} --target first_clapgen_codegen
)
set(second_build_command
  "${CMAKE_COMMAND}" --build "${project_build}" ${build_config_args} --target second_clapgen_codegen
)
run_checked(${first_build_command})
run_checked(${second_build_command})

set(first_manifest "${project_build}/clapgen/first/clapgen.manifest.kdl")
set(second_manifest "${project_build}/clapgen/second/clapgen.manifest.kdl")
set(first_depfile "${project_build}/clapgen/first/clapgen.d")
set(second_depfile "${project_build}/clapgen/second/clapgen.d")
if(NOT EXISTS "${first_manifest}" OR NOT EXISTS "${second_manifest}")
  message(FATAL_ERROR "real generation did not produce both target manifests")
endif()
file(READ "${first_depfile}" first_depfile_content)
file(READ "${second_depfile}" second_depfile_content)
string(FIND "${first_depfile_content}" "common.kdl" import_dep)
if(import_dep EQUAL -1)
  message(FATAL_ERROR "generated depfile does not track imported KDL metadata")
endif()
string(FIND "${second_depfile_content}" "view" resource_dep)
if(resource_dep EQUAL -1)
  message(FATAL_ERROR "generated depfile does not track declared resources")
endif()

file(TIMESTAMP "${first_manifest}" first_before UTC)
execute_process(COMMAND "${CMAKE_COMMAND}" -E sleep 1)
run_checked(${first_build_command})
file(TIMESTAMP "${first_manifest}" first_noop UTC)
if(NOT first_noop STREQUAL first_before)
  message(FATAL_ERROR "no-op rebuild changed the generated manifest timestamp")
endif()

execute_process(COMMAND "${CMAKE_COMMAND}" -E sleep 1)
file(APPEND "${project_source}/shared/common.kdl" "// imported dependency changed\n")
run_and_require_regeneration("Generating CLAP sources for first" ${first_build_command})
file(TOUCH_NOCREATE "${first_manifest}")

execute_process(COMMAND "${CMAKE_COMMAND}" -E sleep 1)
file(APPEND "${project_source}/assets/view file.html" "<!-- resource dependency changed -->\n")
run_and_require_regeneration("Generating CLAP sources for second" ${second_build_command})
file(TOUCH_NOCREATE "${second_manifest}")

file(TIMESTAMP "${first_manifest}" first_before_metadata UTC)
file(TIMESTAMP "${second_manifest}" second_before_metadata UTC)
execute_process(COMMAND "${CMAKE_COMMAND}" -E sleep 1)
file(READ "${project_source}/first plugin.kdl" first_metadata)
string(REPLACE "version=\"1.0.0\"" "version=\"1.0.1\"" first_metadata "${first_metadata}")
file(WRITE "${project_source}/first plugin.kdl" "${first_metadata}")
run_checked(${first_build_command})
file(TIMESTAMP "${first_manifest}" first_after UTC)
file(TIMESTAMP "${second_manifest}" second_after UTC)
if(first_after STREQUAL first_before_metadata)
  message(FATAL_ERROR "metadata edit did not regenerate the affected target")
endif()
if(NOT second_after STREQUAL second_before_metadata)
  message(FATAL_ERROR "metadata edit regenerated an unrelated target")
endif()

find_program(NINJA_EXECUTABLE NAMES ninja ninja-build)
if(NINJA_EXECUTABLE)
  set(ninja_build "${test_root}/ninja build")
  run_checked("${CMAKE_COMMAND}" -G Ninja -S "${project_source}" -B "${ninja_build}")
  run_checked("${CMAKE_COMMAND}" --build "${ninja_build}" --target first_clapgen_codegen)
endif()

if(CMAKE_HOST_APPLE)
  set(xcode_build "${test_root}/xcode build")
  run_checked("${CMAKE_COMMAND}" -G Xcode -S "${project_source}" -B "${xcode_build}")
  set(xcode_config Debug)
  if(DEFINED CLAPGEN_CONFIG AND NOT CLAPGEN_CONFIG STREQUAL "")
    set(xcode_config "${CLAPGEN_CONFIG}")
  endif()
  run_checked(
    "${CMAKE_COMMAND}" --build "${xcode_build}" --config "${xcode_config}"
    --target first_clapgen_codegen
  )
endif()
