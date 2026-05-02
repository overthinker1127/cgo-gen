# 08 Overloading

C++ constructor and method overloads exposed through explicit Go names and a typed dispatcher.

`OverloadMath` has zero-argument and `int` constructors plus `Add(int, int)` and `Add(double, double)` methods. The generated Go facade exposes constructor names such as `NewOverloadMath` and `NewOverloadMathWithBase`, direct typed methods such as `AddInt32Int32` and `AddFloat64Float64`, and an `Add(args ...any)` dispatcher that selects the overload from Go argument types.

```bash
cargo run --bin cgo-gen -- check --config examples/08-overloading/config.yaml
cargo run --bin cgo-gen -- generate --config examples/08-overloading/config.yaml --dump-ir
```

- `input/overload_math.hpp`: input header with overloaded constructors and methods
- `generated/`: committed generator output
