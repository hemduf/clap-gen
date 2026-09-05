# Generated runtime validation

The #8 core runtime qualification builds a real generated CLAP module, runs the repository CTest
suite, and validates the module with a SHA256-pinned clap-validator release.

## Build and test locally

```sh
cmake -S . -B build/validation -DBUILD_TESTING=ON -DCLAPGEN_FETCH_CLAP=ON -DCMAKE_BUILD_TYPE=Debug
cmake --build build/validation --target clapgen_issue55_validation_plugin
ctest --test-dir build/validation -C Debug --output-on-failure
python3 tools/run_clap_validator.py --search-root build/validation --plugin-name clapgen_issue55_validation.clap
```

Repeat with `-DCMAKE_BUILD_TYPE=Release` for the Release configuration. Multi-config generators may
place the plugin below a `Debug` or `Release` subdirectory; the runner searches recursively.

The runner downloads clap-validator 0.4.1 from the official `free-audio/clap-validator` GitHub
release and verifies the platform archive's pinned SHA256 before extraction. To reproduce with an
already installed validator, bypass downloading it:

```sh
python3 tools/run_clap_validator.py \
  --search-root build/validation \
  --plugin-name clapgen_issue55_validation.clap \
  --validator /path/to/clap-validator
```

On macOS, `clapgen_issue55_validation.clap` is a proper bundle and the validator receives the bundle
root, matching the CLAP entry-point path contract. Linux and Windows use a `.clap` shared library.
