# 04 Go Module

Minimal example for generation with `--go-module`.

```bash
cargo run --bin cgo-gen -- check --config examples/04-go-module/config.yaml
cargo run --bin cgo-gen -- generate --config examples/04-go-module/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/04-go-module/generated
```

- `input/score.h`: input header
- `input/score.c`: matching implementation for reference
- `generated/`: committed generator output, including `go.mod` and `build_flags.go`
