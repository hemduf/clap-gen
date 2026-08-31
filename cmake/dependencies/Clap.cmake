include_guard(GLOBAL)
include(FetchContent)

set(
  CLAPGEN_CLAP_GIT_REPOSITORY
  "https://github.com/free-audio/clap.git"
  CACHE STRING
  "CLAP SDK repository"
)
set(
  CLAPGEN_CLAP_GIT_TAG
  "a47f6badb49d948fd009998f28309cdab78979c9"
  CACHE STRING
  "Pinned CLAP SDK commit"
)

FetchContent_Declare(
  clap
  GIT_REPOSITORY "${CLAPGEN_CLAP_GIT_REPOSITORY}"
  GIT_TAG "${CLAPGEN_CLAP_GIT_TAG}"
  GIT_SHALLOW FALSE
)
FetchContent_MakeAvailable(clap)
