#include <clap/clap.h>

#include <cstdint>
#include <cstring>
#include <iostream>
#include <limits>
#include <string>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace {

class DynamicLibrary {
public:
  explicit DynamicLibrary(const char* path) {
#ifdef _WIN32
    handle_ = LoadLibraryA(path);
#else
    handle_ = dlopen(path, RTLD_NOW | RTLD_LOCAL);
#endif
  }

  ~DynamicLibrary() {
#ifdef _WIN32
    if (handle_ != nullptr) {
      FreeLibrary(handle_);
    }
#else
    if (handle_ != nullptr) {
      dlclose(handle_);
    }
#endif
  }

  DynamicLibrary(const DynamicLibrary&) = delete;
  DynamicLibrary& operator=(const DynamicLibrary&) = delete;

  [[nodiscard]] bool valid() const { return handle_ != nullptr; }

  [[nodiscard]] void* symbol(const char* name) const {
#ifdef _WIN32
    const FARPROC raw = GetProcAddress(handle_, name);
    if (raw == nullptr) {
      return nullptr;
    }
    static_assert(sizeof(raw) == sizeof(void*));
    void* result = nullptr;
    std::memcpy(&result, &raw, sizeof(result));
    return result;
#else
    return dlsym(handle_, name);
#endif
  }

private:
#ifdef _WIN32
  HMODULE handle_ = nullptr;
#else
  void* handle_ = nullptr;
#endif
};

std::uint32_t host_callback_calls = 0u;

const void* CLAP_ABI host_get_extension(const clap_host_t*, const char*) {
  ++host_callback_calls;
  return nullptr;
}

void CLAP_ABI host_request_restart(const clap_host_t*) { ++host_callback_calls; }

void CLAP_ABI host_request_process(const clap_host_t*) { ++host_callback_calls; }

void CLAP_ABI host_request_callback(const clap_host_t*) { ++host_callback_calls; }

clap_host_t compatible_host() {
  clap_host_t host{};
  host.clap_version = CLAP_VERSION;
  host.name = "clap-gen DSO loader";
  host.vendor = "clap-gen";
  host.url = "https://example.invalid";
  host.version = "1.0";
  host.get_extension = host_get_extension;
  host.request_restart = host_request_restart;
  host.request_process = host_request_process;
  host.request_callback = host_request_callback;
  return host;
}

int fail(int code, const char* message) {
  std::cerr << "#64 DSO smoke failure " << code << ": " << message << '\n';
  return code;
}

} // namespace

int main(int argc, char** argv) {
  if (argc != 2) {
    return fail(1, "expected module path argument");
  }

  DynamicLibrary module(argv[1]);
  if (!module.valid()) {
    return fail(2, "failed to load generated module");
  }

  const auto* entry = static_cast<const clap_plugin_entry_t*>(module.symbol("clap_entry"));
  if (entry == nullptr) {
    return fail(3, "exact exported data symbol `clap_entry` was not found");
  }
  if (entry->clap_version.major != CLAP_VERSION.major ||
      entry->clap_version.minor != CLAP_VERSION.minor ||
      entry->clap_version.revision != CLAP_VERSION.revision) {
    return fail(4, "resolved entry exposes unexpected CLAP version");
  }

  if (entry->get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr ||
      entry->get_factory(nullptr) != nullptr ||
      entry->get_factory("clap.unknown-factory") != nullptr) {
    return fail(5, "pre-init factory discovery was not rejected");
  }

  if (!entry->init(argv[1]) || !entry->init(argv[1])) {
    return fail(6, "repeated entry init failed");
  }

  const auto* factory =
      static_cast<const clap_plugin_factory_t*>(entry->get_factory(CLAP_PLUGIN_FACTORY_ID));
  std::string copied_factory_id(CLAP_PLUGIN_FACTORY_ID);
  if (copied_factory_id.c_str() == CLAP_PLUGIN_FACTORY_ID) {
    return fail(7, "factory-ID copy unexpectedly reused SDK storage");
  }
  const auto* copied_factory =
      static_cast<const clap_plugin_factory_t*>(entry->get_factory(copied_factory_id.c_str()));
  if (factory == nullptr || copied_factory != factory) {
    return fail(8, "factory ID was not matched by content across the DSO boundary");
  }
  if (entry->get_factory(nullptr) != nullptr ||
      entry->get_factory("clap.unknown-factory") != nullptr) {
    return fail(9, "invalid factory IDs were not rejected after init");
  }

  const std::uint32_t count = factory->get_plugin_count(factory);
  if (count != 1u) {
    return fail(10, "fixture should expose exactly one generated descriptor");
  }

  const clap_plugin_descriptor_t* descriptor = factory->get_plugin_descriptor(factory, 0u);
  if (descriptor == nullptr || factory->get_plugin_descriptor(factory, 0u) != descriptor) {
    return fail(11, "descriptor pointer identity was not stable");
  }
  if (factory->get_plugin_descriptor(factory, count) != nullptr ||
      factory->get_plugin_descriptor(factory, count + 1u) != nullptr ||
      factory->get_plugin_descriptor(factory, std::numeric_limits<std::uint32_t>::max()) !=
          nullptr) {
    return fail(12, "out-of-range descriptor lookup did not fail safely");
  }

  auto host = compatible_host();
  auto incompatible_host = host;
  incompatible_host.clap_version = clap_version_t{0u, 99u, 99u};

  host_callback_calls = 0u;
  if (factory->create_plugin(factory, nullptr, descriptor->id) != nullptr ||
      factory->create_plugin(factory, &host, nullptr) != nullptr ||
      factory->create_plugin(factory, &incompatible_host, descriptor->id) != nullptr ||
      factory->create_plugin(factory, &host, "com.example.unknown") != nullptr) {
    return fail(13, "invalid create_plugin request did not fail safely");
  }
  if (host_callback_calls != 0u) {
    return fail(14, "create_plugin called back into the host");
  }

  std::string copied_plugin_id(descriptor->id);
  if (copied_plugin_id.c_str() == descriptor->id) {
    return fail(15, "plugin-ID copy unexpectedly reused descriptor storage");
  }

  const clap_plugin_t* plugin = factory->create_plugin(factory, &host, copied_plugin_id.c_str());
  if (plugin == nullptr) {
    return fail(16, "known copied plugin ID did not route into test backend");
  }
  if (plugin->desc != descriptor) {
    return fail(17, "backend did not receive the expected descriptor index");
  }
  if (plugin->plugin_data != &host) {
    return fail(18, "backend did not receive the original borrowed host pointer");
  }
  if (factory->create_plugin(factory, &host, descriptor->id) != plugin) {
    return fail(19, "test backend did not return a stable sentinel plugin pointer");
  }
  if (host_callback_calls != 0u) {
    return fail(20, "known-ID routing called back into the host");
  }

  entry->deinit();
  if (entry->get_factory(CLAP_PLUGIN_FACTORY_ID) != factory ||
      factory->get_plugin_count(factory) != count) {
    return fail(21, "factory disappeared before the final matched deinit");
  }

  entry->deinit();
  if (entry->get_factory(CLAP_PLUGIN_FACTORY_ID) != nullptr) {
    return fail(22, "factory remained discoverable after final deinit");
  }
  if (factory->get_plugin_count(factory) != 0u ||
      factory->get_plugin_descriptor(factory, 0u) != nullptr ||
      factory->create_plugin(factory, &host, descriptor->id) != nullptr) {
    return fail(23, "stale factory pointer remained active after final deinit");
  }

  return 0;
}
