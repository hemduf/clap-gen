add_executable(
  clapgen_issue51_extension_dispatch_smoke
  "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue51/extension_dispatch_smoke.cpp"
)
add_dependencies(clapgen_issue51_extension_dispatch_smoke clapgen_issue59_generated)
target_compile_features(clapgen_issue51_extension_dispatch_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue51_extension_dispatch_smoke
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
clapgen_enable_warnings(clapgen_issue51_extension_dispatch_smoke)
add_test(
  NAME clapgen.codegen.issue51.extension_dispatch_smoke
  COMMAND clapgen_issue51_extension_dispatch_smoke
)
