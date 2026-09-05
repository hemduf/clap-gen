if(NOT DEFINED INPUT OR INPUT STREQUAL "")
  message(FATAL_ERROR "ClapGenRewriteDepfile.cmake requires INPUT")
endif()
if(NOT DEFINED OUTPUT OR OUTPUT STREQUAL "")
  message(FATAL_ERROR "ClapGenRewriteDepfile.cmake requires OUTPUT")
endif()
if(NOT DEFINED PREFIX OR PREFIX STREQUAL "")
  set(PREFIX ".")
endif()

file(READ "${INPUT}" _clapgen_depfile)

# clapgen's public depfile is portable and relative to its output directory.
# CMake policy CMP0116 interprets relative depfile entries from the current
# binary directory, so prefix each makefile token without disturbing escaped
# spaces inside paths. The public generator output itself remains unchanged.
set(_clapgen_prefix "${PREFIX}")
string(REPLACE "\\" "/" _clapgen_prefix "${_clapgen_prefix}")
string(REPLACE " " "\\ " _clapgen_prefix "${_clapgen_prefix}")
string(REPLACE "#" "\\#" _clapgen_prefix "${_clapgen_prefix}")
string(REPLACE "$" "$$" _clapgen_prefix "${_clapgen_prefix}")

set(_clapgen_escaped_space "__CLAPGEN_DEPFILE_ESCAPED_SPACE__")
string(REPLACE "\\ " "${_clapgen_escaped_space}" _clapgen_depfile "${_clapgen_depfile}")
string(REPLACE " " " ${_clapgen_prefix}/" _clapgen_depfile "${_clapgen_depfile}")
string(REPLACE "${_clapgen_escaped_space}" "\\ " _clapgen_depfile "${_clapgen_depfile}")
string(REPLACE
  "clapgen.manifest.kdl:"
  "${_clapgen_prefix}/clapgen.manifest.kdl:"
  _clapgen_depfile
  "${_clapgen_depfile}"
)

file(WRITE "${OUTPUT}" "${_clapgen_depfile}")
