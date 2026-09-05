include_guard(GLOBAL)
include(CMakeParseArguments)

function(_clapgen_resolve_generator out_var explicit_generator)
  if(explicit_generator)
    set(${out_var} "${explicit_generator}" PARENT_SCOPE)
    return()
  endif()

  if(CMAKE_CROSSCOMPILING)
    if(DEFINED CLAPGEN_HOST_EXECUTABLE AND NOT CLAPGEN_HOST_EXECUTABLE STREQUAL "")
      set(${out_var} "${CLAPGEN_HOST_EXECUTABLE}" PARENT_SCOPE)
      return()
    endif()

    if(TARGET ClapGen::clapgen)
      get_target_property(_clapgen_imported ClapGen::clapgen IMPORTED)
      if(_clapgen_imported)
        set(${out_var} "$<TARGET_FILE:ClapGen::clapgen>" PARENT_SCOPE)
        return()
      endif()
    endif()

    find_program(_clapgen_host_program NAMES clapgen)
    if(_clapgen_host_program)
      set(${out_var} "${_clapgen_host_program}" PARENT_SCOPE)
      return()
    endif()

    message(FATAL_ERROR
      "Cross-compiling requires a host-runnable clapgen. Set CLAPGEN_HOST_EXECUTABLE, "
      "pass GENERATOR to clapgen_add_plugin(), or provide an imported ClapGen::clapgen target."
    )
  endif()

  if(TARGET ClapGen::clapgen)
    set(${out_var} "$<TARGET_FILE:ClapGen::clapgen>" PARENT_SCOPE)
    return()
  endif()

  if(DEFINED CLAPGEN_EXECUTABLE AND NOT CLAPGEN_EXECUTABLE STREQUAL "")
    set(${out_var} "${CLAPGEN_EXECUTABLE}" PARENT_SCOPE)
    return()
  endif()

  find_program(_clapgen_program NAMES clapgen)
  if(NOT _clapgen_program)
    message(FATAL_ERROR
      "Unable to locate the clapgen host executable. Set CLAPGEN_EXECUTABLE or pass "
      "GENERATOR to clapgen_add_plugin()."
    )
  endif()
  set(${out_var} "${_clapgen_program}" PARENT_SCOPE)
endfunction()

function(clapgen_add_plugin target)
  set(options)
  set(one_value_args METADATA OUTPUT_DIR GENERATOR CLAP_INCLUDE_DIR)
  set(multi_value_args SOURCES DEPENDS)
  cmake_parse_arguments(CLAPGEN "${options}" "${one_value_args}" "${multi_value_args}" ${ARGN})

  if(CLAPGEN_UNPARSED_ARGUMENTS)
    message(FATAL_ERROR "clapgen_add_plugin(${target}): unknown arguments: ${CLAPGEN_UNPARSED_ARGUMENTS}")
  endif()
  if(NOT target)
    message(FATAL_ERROR "clapgen_add_plugin requires a target name")
  endif()
  if(TARGET "${target}")
    message(FATAL_ERROR "clapgen_add_plugin(${target}): target already exists")
  endif()
  if(NOT CLAPGEN_METADATA)
    message(FATAL_ERROR "clapgen_add_plugin(${target}) requires METADATA <plugin.kdl>")
  endif()

  get_filename_component(_clapgen_metadata "${CLAPGEN_METADATA}" ABSOLUTE BASE_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
  if(NOT EXISTS "${_clapgen_metadata}")
    message(FATAL_ERROR "clapgen_add_plugin(${target}): metadata does not exist: ${_clapgen_metadata}")
  endif()

  if(CLAPGEN_OUTPUT_DIR)
    get_filename_component(_clapgen_output_dir "${CLAPGEN_OUTPUT_DIR}" ABSOLUTE BASE_DIR "${CMAKE_CURRENT_BINARY_DIR}")
  else()
    set(_clapgen_output_dir "${CMAKE_CURRENT_BINARY_DIR}/clapgen/${target}")
  endif()

  _clapgen_resolve_generator(_clapgen_generator "${CLAPGEN_GENERATOR}")

  set(_clapgen_depfile "${_clapgen_output_dir}/clapgen.d")
  set(_clapgen_manifest "${_clapgen_output_dir}/clapgen.manifest.kdl")
  set(_clapgen_sources_manifest "${_clapgen_output_dir}/clapgen.sources.kdl")
  set(_clapgen_descriptors "${_clapgen_output_dir}/clapgen_descriptors.hpp")
  set(_clapgen_entry "${_clapgen_output_dir}/clapgen_entry.cpp")
  set(_clapgen_extensions "${_clapgen_output_dir}/clapgen_extensions.hpp")
  set(_clapgen_ids "${_clapgen_output_dir}/clapgen_ids.hpp")
  set(_clapgen_backend_source "${_clapgen_output_dir}/clapgen_instance_backend.cpp")
  set(_clapgen_backend_header "${_clapgen_output_dir}/clapgen_instance_backend.hpp")
  set(_clapgen_metadata_source "${_clapgen_output_dir}/clapgen_metadata.cpp")
  set(_clapgen_metadata_header "${_clapgen_output_dir}/clapgen_metadata.hpp")
  set(_clapgen_processor "${_clapgen_output_dir}/clapgen_processor.hpp")
  set(_clapgen_resources "${_clapgen_output_dir}/clapgen_resources.hpp")

  set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${_clapgen_metadata}")

  add_custom_command(
    OUTPUT "${_clapgen_manifest}"
    BYPRODUCTS
      "${_clapgen_depfile}"
      "${_clapgen_sources_manifest}"
      "${_clapgen_descriptors}"
      "${_clapgen_entry}"
      "${_clapgen_extensions}"
      "${_clapgen_ids}"
      "${_clapgen_backend_source}"
      "${_clapgen_backend_header}"
      "${_clapgen_metadata_source}"
      "${_clapgen_metadata_header}"
      "${_clapgen_processor}"
      "${_clapgen_resources}"
    COMMAND "${_clapgen_generator}" generate --metadata "${_clapgen_metadata}" --out "${_clapgen_output_dir}"
    DEPENDS "${_clapgen_metadata}" ${CLAPGEN_DEPENDS}
    DEPFILE "${_clapgen_depfile}"
    COMMENT "Generating CLAP sources for ${target}"
    VERBATIM
  )

  set(_clapgen_codegen_target "${target}_clapgen_codegen")
  add_custom_target("${_clapgen_codegen_target}" DEPENDS "${_clapgen_manifest}")

  set_source_files_properties(
    "${_clapgen_entry}"
    "${_clapgen_backend_source}"
    "${_clapgen_metadata_source}"
    PROPERTIES GENERATED TRUE
  )

  add_library(
    "${target}"
    MODULE
    "${_clapgen_entry}"
    "${_clapgen_backend_source}"
    "${_clapgen_metadata_source}"
    ${CLAPGEN_SOURCES}
  )
  add_dependencies("${target}" "${_clapgen_codegen_target}")
  target_compile_features("${target}" PRIVATE cxx_std_20)
  target_include_directories("${target}" PRIVATE "${_clapgen_output_dir}")

  if(CLAPGEN_CLAP_INCLUDE_DIR)
    target_include_directories("${target}" PRIVATE "${CLAPGEN_CLAP_INCLUDE_DIR}")
  elseif(DEFINED CLAP_INCLUDE_DIR AND NOT CLAP_INCLUDE_DIR STREQUAL "")
    target_include_directories("${target}" PRIVATE "${CLAP_INCLUDE_DIR}")
  endif()

  if(TARGET ClapGen::Runtime)
    target_link_libraries("${target}" PRIVATE ClapGen::Runtime)
  endif()

  set_target_properties(
    "${target}"
    PROPERTIES
      PREFIX ""
      SUFFIX ".clap"
      CLAPGEN_METADATA "${_clapgen_metadata}"
      CLAPGEN_OUTPUT_DIR "${_clapgen_output_dir}"
      CLAPGEN_GENERATED_MANIFEST "${_clapgen_manifest}"
  )
endfunction()
