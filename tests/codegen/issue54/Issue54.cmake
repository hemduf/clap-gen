add_library(
  clapgen_issue54_native_abi_contract
  OBJECT
  "${CMAKE_CURRENT_SOURCE_DIR}/tests/codegen/issue54/native_abi_contract.cpp"
)
add_dependencies(clapgen_issue54_native_abi_contract clapgen_issue59_generated)
target_compile_features(clapgen_issue54_native_abi_contract PRIVATE cxx_std_20)
target_include_directories(
  clapgen_issue54_native_abi_contract
  PRIVATE
    "${CLAPGEN_ISSUE59_GENERATED_DIR}"
    "${clap_SOURCE_DIR}/include"
)
clapgen_enable_warnings(clapgen_issue54_native_abi_contract)
if(MSVC)
  target_compile_options(clapgen_issue54_native_abi_contract PRIVATE /WX)
else()
  target_compile_options(clapgen_issue54_native_abi_contract PRIVATE -Werror)
endif()
