# 13 Macros

Object-like C macro constants.

`cgo-gen` emits supported integer, floating-point, and ordinary string literal
macros as untyped Go constants. Function-like macros, raw string literals, and
prefixed string literals are intentionally skipped.

```bash
cargo run --bin cgo-gen -- check --config examples/macros/config.yaml
cargo run --bin cgo-gen -- generate --config examples/macros/config.yaml --dump-ir
```

- `input/macro_constants.h`: input header with integer, float, and string macro constants
- `generated/`: committed generator output
