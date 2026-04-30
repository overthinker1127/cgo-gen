# 07 Enums

C/C++ enum declarations and enum values in function and method signatures.

`cgo-gen` emits named enums as Go `int64` enum types, emits standalone anonymous enums as untyped Go constants, and converts enum parameters/returns through the C ABI as `int64_t`.

```bash
cargo run --bin cgo-gen -- check --config examples/07-enums/config.yaml
cargo run --bin cgo-gen -- generate --config examples/07-enums/config.yaml --dump-ir
```

- `input/device_controller.hpp`: input header with named, typedef, and anonymous enums
- `generated/`: committed generator output
