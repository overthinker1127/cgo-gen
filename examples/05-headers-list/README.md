# 05 Headers List

Explicit header-list config that wraps only selected headers.

```bash
cargo run --bin cgo-gen -- check --config examples/05-headers-list/config.yaml
cargo run --bin cgo-gen -- generate --config examples/05-headers-list/config.yaml --dump-ir
```

- `input/selected_widget.hpp`: selected input header
- `input/selected_counter.hpp`: selected input header
- `input/shared_dependency.hpp`: included by a selected header, but not wrapped unless listed in `input.headers`
- `generated/`: committed generator output
