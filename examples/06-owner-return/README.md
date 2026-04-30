# 06 Owner Return

C++ facade method that returns a `new`-allocated model pointer.

`SessionFactory::CreateSession` returns ownership to the caller, so the config lists it under `input.owner`. The generated Go facade wraps that return as an owned value whose `Close` method releases the native object.

```bash
cargo run --bin cgo-gen -- check --config examples/06-owner-return/config.yaml
cargo run --bin cgo-gen -- generate --config examples/06-owner-return/config.yaml --dump-ir
```

- `input/session_factory.hpp`: input header with an owned factory return and a borrowed return
- `generated/`: committed generator output
