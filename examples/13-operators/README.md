# 13 Operators

C++ operator declarations are generated as named wrapper calls.

Member operators use `Oper...` method names in the Go facade and C ABI. Free
operators use the same `Oper...` name as package-level Go functions.

```bash
cargo run --bin cgo-gen -- check --config examples/13-operators/config.yaml
cargo run --bin cgo-gen -- generate --config examples/13-operators/config.yaml --dump-ir
```

- `Vector2::operator+` -> `Vector2.OperPlus`
- `Vector2::operator==` -> `Vector2.OperEqual`
- `Vector2::operator bool` -> `Vector2.OperBool`
- `operator-` -> `OperMinus`

`input/vector2.hpp` keeps the operators inline so the generated wrapper can be
compiled directly from the example without a separate native library.
