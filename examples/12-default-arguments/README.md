# 12 Default Arguments

C++ trailing default arguments exposed as shorter generated wrapper overloads.

`Clamp` and `DefaultCounter` declare default values in C++. The generated Go facade does not copy those literal defaults into Go; instead it emits shorter overload variants that call C++ with fewer arguments so the native defaults are applied by C++.

```bash
cargo run --bin cgo-gen -- check --config examples/12-default-arguments/config.yaml
cargo run --bin cgo-gen -- generate --config examples/12-default-arguments/config.yaml --dump-ir
```

- `input/default_arguments.hpp`: input header with free function, constructor, and method default arguments
- `generated/`: committed generator output
