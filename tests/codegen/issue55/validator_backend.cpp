#include <clap/clap.h>

#include <cstdint>

#include "clapgen_instance_backend.hpp"

namespace clapgen::generated::detail {

namespace {

struct ValidatorProcessor {
  bool init() {
    initialized_ = true;
    return true;
  }

  bool activate(double sample_rate, std::uint32_t min_frames_count,
                std::uint32_t max_frames_count) {
    if (!initialized_ || sample_rate <= 0.0 || min_frames_count > max_frames_count) {
      return false;
    }
    active_ = true;
    return true;
  }

  void deactivate() { active_ = false; }

  bool start_processing() {
    if (!active_) {
      return false;
    }
    processing_ = true;
    return true;
  }

  void stop_processing() { processing_ = false; }

  void reset() { ++reset_count_; }

  clap_process_status process(const clap_process_t* process_data) {
    if (!processing_ || process_data == nullptr) {
      return CLAP_PROCESS_ERROR;
    }
    ++process_count_;
    return CLAP_PROCESS_CONTINUE;
  }

private:
  bool initialized_ = false;
  bool active_ = false;
  bool processing_ = false;
  std::uint32_t reset_count_ = 0u;
  std::uint64_t process_count_ = 0u;
};

static_assert(clapgen::generated::NativeProcessor<ValidatorProcessor>);

} // namespace

const clap_plugin_t* create_plugin_instance(std::uint32_t descriptor_index,
                                            const clap_host_t* host) {
  return create_plugin_instance_for<ValidatorProcessor>(descriptor_index, host);
}

} // namespace clapgen::generated::detail
