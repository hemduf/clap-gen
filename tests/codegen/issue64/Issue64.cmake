add_library(
  clapgen_issue64_test_module
  MODULE
    "${CLAPGEN_ISSUE59_ENTRY}"
    "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue64/module_backend.cpp"
)
add_dependencies(clapgen_issue64_test_module clapgen_issue59_generated)
target_compile_features(clapgen_issue64_test_module PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue64_test_module
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
set_target_properties(
  clapgen_issue64_test_module
  PROPERTIES
    PREFIX ""
    OUTPUT_NAME "clapgen_issue64_plugin"
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN YES
)
clapgen_enable_warnings(clapgen_issue64_test_module)

add_executable(
  clapgen_issue64_dso_loader_smoke
  "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue64/dso_loader_smoke.cpp"
)
add_dependencies(clapgen_issue64_dso_loader_smoke clapgen_issue64_test_module)
target_compile_features(clapgen_issue64_dso_loader_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue64_dso_loader_smoke
  PRIVATE
    "${clap_SOURCE_DIR}/include"
)
target_link_libraries(clapgen_issue64_dso_loader_smoke PRIVATE ${CMAKE_DL_LIBS})
clapgen_enable_warnings(clapgen_issue64_dso_loader_smoke)

add_test(
  NAME clapgen.codegen.issue64.dso_loader_smoke
  COMMAND clapgen_issue64_dso_loader_smoke "$<TARGET_FILE:clapgen_issue64_test_module>"
)
