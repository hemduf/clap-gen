if(NOT DEFINED CLAPGEN_SOURCE_DIR OR NOT DEFINED MODE)
  message(FATAL_ERROR "CLAPGEN_SOURCE_DIR and MODE are required")
endif()

file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenPackage.cmake" package_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenFunctions.cmake" functions_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenConfig.cmake.in" config_source)
file(READ "${CLAPGEN_SOURCE_DIR}/cmake/ClapGenWarnings.cmake" bootstrap_source)

if(MODE STREQUAL "consumer")
  foreach(required IN ITEMS
      "EXPORT_NAME Runtime"
      "ClapGenTargets.cmake"
      "ClapGenConfigVersion.cmake"
      "ClapGenFunctions.cmake")
    string(FIND "${package_source}" "${required}" found)
    if(found EQUAL -1)
      message(FATAL_ERROR "installed package contract is missing `${required}`")
    endif()
  endforeach()
  foreach(required IN ITEMS "ClapGenTargets.cmake" "ClapGenFunctions.cmake" "check_required_components")
    string(FIND "${config_source}" "${required}" found)
    if(found EQUAL -1)
      message(FATAL_ERROR "package config is missing `${required}`")
    endif()
  endforeach()
  string(FIND "${bootstrap_source}" "include(ClapGenPackage)" found)
  if(found EQUAL -1)
    message(FATAL_ERROR "root CMake bootstrap does not load ClapGenPackage")
  endif()
elseif(MODE STREQUAL "incremental")
  foreach(required IN ITEMS
      "OUTPUT \"\${_clapgen_manifest}\""
      "BYPRODUCTS"
      "DEPENDS \"\${_clapgen_metadata}\""
      "DEPFILE \"\${_clapgen_depfile}\""
      "VERBATIM"
      "\${CMAKE_CURRENT_BINARY_DIR}/clapgen/\${target}")
    string(FIND "${functions_source}" "${required}" found)
    if(found EQUAL -1)
      message(FATAL_ERROR "incremental generation contract is missing `${required}`")
    endif()
  endforeach()
elseif(MODE STREQUAL "cross")
  foreach(required IN ITEMS
      "CMAKE_CROSSCOMPILING"
      "CLAPGEN_HOST_EXECUTABLE"
      "get_target_property(_clapgen_imported ClapGen::clapgen IMPORTED)"
      "Cross-compiling requires a host-runnable clapgen")
    string(FIND "${functions_source}" "${required}" found)
    if(found EQUAL -1)
      message(FATAL_ERROR "cross-compilation contract is missing `${required}`")
    endif()
  endforeach()
else()
  message(FATAL_ERROR "unknown MODE `${MODE}`")
endif()
