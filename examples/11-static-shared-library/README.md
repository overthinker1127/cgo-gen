# Static and Shared Library Example

This example wraps declarations from `input/library_math.hpp` while linking the generated Go package against prebuilt native libraries:

- `lib/libnative_static_math.a`: constructor, class methods, and `static_offset`
- `lib/libnative_shared_multiplier.so`: `shared_multiplier`

This example is Linux-only because it builds and links a `.so` and uses `LD_LIBRARY_PATH`.

Build the native libraries before checking or testing the generated package:

```bash
./examples/11-static-shared-library/build-libs.sh
cargo run --bin cgo-gen -- generate --config examples/11-static-shared-library/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/11-static-shared-library/generated
cd examples/11-static-shared-library/generated && LD_LIBRARY_PATH=../lib go test ./...
```

Compiled `.a` and `.so` files are intentionally ignored. Recreate them with `build-libs.sh`.
