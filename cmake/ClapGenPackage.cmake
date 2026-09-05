include_guard(GLOBAL)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

function(_clapgen_configure_package)
  if(NOT TARGET clapgen_runtime)
    message(FATAL_ERROR "ClapGen package setup requires the clapgen_runtime target")
  endif()

  set_target_properties(clapgen_runtime PROPERTIES EXPORT_NAME Runtime)

  install(
    TARGETS clapgen_runtime
    EXPORT ClapGenTargets
    ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
    INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  )
  install(
    DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/runtime/include/"
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  )

  set(_clapgen_package_dir "${CMAKE_INSTALL_LIBDIR}/cmake/ClapGen")
  configure_package_config_file(
    "${CMAKE_CURRENT_FUNCTION_LIST_DIR}/ClapGenConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/ClapGenConfig.cmake"
    INSTALL_DESTINATION "${_clapgen_package_dir}"
  )
  write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/ClapGenConfigVersion.cmake"
    VERSION "${PROJECT_VERSION}"
    COMPATIBILITY SameMajorVersion
  )

  install(
    EXPORT ClapGenTargets
    FILE ClapGenTargets.cmake
    NAMESPACE ClapGen::
    DESTINATION "${_clapgen_package_dir}"
  )
  install(
    FILES
      "${CMAKE_CURRENT_BINARY_DIR}/ClapGenConfig.cmake"
      "${CMAKE_CURRENT_BINARY_DIR}/ClapGenConfigVersion.cmake"
      "${CMAKE_CURRENT_FUNCTION_LIST_DIR}/ClapGenFunctions.cmake"
    DESTINATION "${_clapgen_package_dir}"
  )

  if(BUILD_TESTING)
    include("${CMAKE_CURRENT_SOURCE_DIR}/tests/cmake/issue9/Issue9.cmake")
  endif()
endfunction()

cmake_language(DEFER CALL _clapgen_configure_package)
