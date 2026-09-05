set(_clapgen_issue9_verify "${CMAKE_CURRENT_LIST_DIR}/VerifyIssue9.cmake")

foreach(_clapgen_issue9_mode IN ITEMS consumer incremental cross)
  add_test(
    NAME "clapgen.cmake.issue9.${_clapgen_issue9_mode}"
    COMMAND
      "${CMAKE_COMMAND}"
      "-DCLAPGEN_SOURCE_DIR=${CMAKE_CURRENT_SOURCE_DIR}"
      "-DCLAPGEN_BINARY_DIR=${CMAKE_CURRENT_BINARY_DIR}"
      "-DCLAPGEN_CONFIG=$<CONFIG>"
      "-DMODE=${_clapgen_issue9_mode}"
      -P "${_clapgen_issue9_verify}"
  )
endforeach()

add_test(
  NAME clapgen.cmake.issue9.real-codegen
  COMMAND
    "${CMAKE_COMMAND}"
    "-DCLAPGEN_SOURCE_DIR=${CMAKE_CURRENT_SOURCE_DIR}"
    "-DCLAPGEN_BINARY_DIR=${CMAKE_CURRENT_BINARY_DIR}"
    "-DCLAPGEN_CONFIG=$<CONFIG>"
    -P "${CMAKE_CURRENT_LIST_DIR}/RealCodegen.cmake"
)
