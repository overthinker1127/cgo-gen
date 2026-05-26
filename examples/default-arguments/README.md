# 12 Default Arguments

C++ trailing default arguments exposed through dispatcher-only Go overload APIs.

`Clamp` and `DefaultCounter` declare default values in C++. The generated Go facade does not copy those literal defaults into Go; instead its `args ...any` dispatchers call the shorter C ABI variants directly so the native defaults are applied by C++.

```bash
cargo run --bin cgo-gen -- check --config examples/default-arguments/config.yaml
cargo run --bin cgo-gen -- generate --config examples/default-arguments/config.yaml --dump-ir
```

- `input/default_arguments.hpp`: input header with free function, constructor, and method default arguments
- `generated/`: committed generator output
