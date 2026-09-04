add_executable(
  clapgen_issue50_abi_exception_smoke
  "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue50/abi_exception_smoke.cpp"
)
add_dependencies(clapgen_issue50_abi_exception_smoke clapgen_issue59_generated)
target_compile_features(clapgen_issue50_abi_exception_smoke PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue50_abi_exception_smoke
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
if(
  UNIX
  AND CMAKE_BUILD_TYPE STREQUAL "Debug"
  AND CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang|AppleClang"
)
  target_compile_options(
    clapgen_issue50_abi_exception_smoke
    PRIVATE -fsanitize=address,undefined -fno-omit-frame-pointer
  )
  target_link_options(
    clapgen_issue50_abi_exception_smoke
    PRIVATE -fsanitize=address,undefined
  )
endif()
clapgen_enable_warnings(clapgen_issue50_abi_exception_smoke)
add_test(
  NAME clapgen.codegen.issue50.abi_exception_smoke
  COMMAND clapgen_issue50_abi_exception_smoke
)

# #51 depends on #50; keep the focused runtime test harnesses chained in dependency order.
include("${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue51/Issue51.cmake")
