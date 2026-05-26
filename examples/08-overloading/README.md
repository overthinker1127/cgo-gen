# 08 Overloading

C++ constructor overloads exposed through explicit Go names, with safe method overloads exposed through a dispatcher-only Go API.

`OverloadMath` has zero-argument and `int` constructors plus `Add(int, int)` and `Add(double, double)` methods. The generated Go facade exposes constructor names such as `NewOverloadMath` and `NewOverloadMathWithBase`, and an `Add(args ...any)` dispatcher that selects the overload from Go argument types while calling the C ABI wrapper directly.

```bash
cargo run --bin cgo-gen -- check --config examples/08-overloading/config.yaml
cargo run --bin cgo-gen -- generate --config examples/08-overloading/config.yaml --dump-ir
```

- `input/overload_math.hpp`: input header with overloaded constructors and methods
- `generated/`: committed generator output
