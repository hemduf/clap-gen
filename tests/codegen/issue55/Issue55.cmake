add_library(
  clapgen_issue55_validation_plugin
  MODULE
    "${CLAPGEN_ISSUE59_ENTRY}"
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue55/validator_backend.cpp"
)
add_dependencies(clapgen_issue55_validation_plugin clapgen_issue59_generated)
target_compile_features(clapgen_issue55_validation_plugin PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue55_validation_plugin
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
set_target_properties(
  clapgen_issue55_validation_plugin
  PROPERTIES
    PREFIX ""
    OUTPUT_NAME "clapgen_issue55_validation"
    CXX_EXTENSIONS OFF
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN YES
    WINDOWS_EXPORT_ALL_SYMBOLS OFF
)
clapgen_enable_warnings(clapgen_issue55_validation_plugin)

if(APPLE)
  set(CLAPGEN_ISSUE55_BUNDLE_DIR "${CMAKE_CURRENT_BINARY_DIR}/issue55/clapgen_issue55_validation.clap")
  file(MAKE_DIRECTORY "${CLAPGEN_ISSUE55_BUNDLE_DIR}/Contents/MacOS")
  configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue55/Info.plist"
    "${CLAPGEN_ISSUE55_BUNDLE_DIR}/Contents/Info.plist"
    COPYONLY
  )
  set_target_properties(
    clapgen_issue55_validation_plugin
    PROPERTIES
      LIBRARY_OUTPUT_DIRECTORY "${CLAPGEN_ISSUE55_BUNDLE_DIR}/Contents/MacOS"
      SUFFIX ""
  )
else()
  set_target_properties(clapgen_issue55_validation_plugin PROPERTIES SUFFIX ".clap")
endif()
