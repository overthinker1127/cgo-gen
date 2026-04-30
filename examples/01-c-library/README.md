# 01 C Library

Small C-style header with free functions.

```bash
cargo run --bin cgo-gen -- check --config examples/01-c-library/config.yaml
cargo run --bin cgo-gen -- generate --config examples/01-c-library/config.yaml --dump-ir
```

- `input/calculator.h`: input header
- `input/calculator.c`: matching implementation for reference
- `generated/`: committed generator output
