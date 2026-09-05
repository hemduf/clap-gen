add_executable(
  clapgen_issue53_realtime_thread_smoke
  "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue53/realtime_thread_smoke.cpp"
)
add_dependencies(clapgen_issue53_realtime_thread_smoke clapgen_issue59_generated)
target_compile_features(clapgen_issue53_realtime_thread_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue53_realtime_thread_smoke
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
clapgen_enable_warnings(clapgen_issue53_realtime_thread_smoke)
add_test(
  NAME clapgen.codegen.issue53.realtime_thread_smoke
  COMMAND clapgen_issue53_realtime_thread_smoke
)
