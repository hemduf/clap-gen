#pragma once

#include <clap/clap.h>

#include <atomic>
#include <cstdint>

namespace clapgen::issue52 {

struct LifetimeCounters {
  std::atomic<int> constructed{0};
  std::atomic<int> destroyed{0};
};

inline LifetimeCounters alpha_lifetime{};
inline LifetimeCounters failing_lifetime{};
inline LifetimeCounters zeta_lifetime{};
inline std::atomic<int> failing_init_calls{0};

inline void reset_counters() {
  alpha_lifetime.constructed.store(0, std::memory_order_relaxed);
  alpha_lifetime.destroyed.store(0, std::memory_order_relaxed);
  failing_lifetime.constructed.store(0, std::memory_order_relaxed);
  failing_lifetime.destroyed.store(0, std::memory_order_relaxed);
  zeta_lifetime.constructed.store(0, std::memory_order_relaxed);
  zeta_lifetime.destroyed.store(0, std::memory_order_relaxed);
  failing_init_calls.store(0, std::memory_order_relaxed);
}

inline int counter(const std::atomic<int>& value) {
  return value.load(std::memory_order_relaxed);
}

struct AlphaProcessor {
  AlphaProcessor() { alpha_lifetime.constructed.fetch_add(1, std::memory_order_relaxed); }
  ~AlphaProcessor() { alpha_lifetime.destroyed.fetch_add(1, std::memory_order_relaxed); }

  bool init() { return true; }
  bool activate(double, std::uint32_t, std::uint32_t) { return true; }
  void deactivate() {}
  bool start_processing() { return true; }
  void stop_processing() {}
  void reset() { ++generation_; }

  clap_process_status process(const clap_process_t*) {
    return generation_ == 0 ? CLAP_PROCESS_CONTINUE : CLAP_PROCESS_SLEEP;
  }

private:
  int generation_ = 0;
};

struct FailingProcessor {
  FailingProcessor() { failing_lifetime.constructed.fetch_add(1, std::memory_order_relaxed); }
  ~FailingProcessor() { failing_lifetime.destroyed.fetch_add(1, std::memory_order_relaxed); }

  bool init() {
    failing_init_calls.fetch_add(1, std::memory_order_relaxed);
    return false;
  }
  bool activate(double, std::uint32_t, std::uint32_t) { return true; }
  void deactivate() {}
  bool start_processing() { return true; }
  void stop_processing() {}
  void reset() {}
  clap_process_status process(const clap_process_t*) { return CLAP_PROCESS_ERROR; }
};

struct ZetaProcessor {
  ZetaProcessor() { zeta_lifetime.constructed.fetch_add(1, std::memory_order_relaxed); }
  ~ZetaProcessor() { zeta_lifetime.destroyed.fetch_add(1, std::memory_order_relaxed); }

  bool init() { return true; }
  bool activate(double, std::uint32_t, std::uint32_t) { return true; }
  void deactivate() {}
  bool start_processing() { return true; }
  void stop_processing() {}
  void reset() {}

  clap_process_status process(const clap_process_t*) {
    return CLAP_PROCESS_CONTINUE_IF_NOT_QUIET;
  }
};

} // namespace clapgen::issue52
