# 09 Go Module

Minimal example for writing Go module metadata with `--go-module`.

This example shows `go.mod` and `build_flags.go` generation only. It is not a
standalone buildable Go package by itself because the native implementation is
kept outside `generated/`.

Native headers and implementations such as `input/score.h` and `input/score.c`
still need to be made available by the consuming package, for example by
copying/installing headers and linking a native `.a` or `.so`.

```bash
cargo run --bin cgo-gen -- check --config examples/09-go-module/config.yaml
cargo run --bin cgo-gen -- generate --config examples/09-go-module/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/09-go-module/generated
```

- `input/score.h`: input header
- `input/score.c`: matching implementation for reference
- `generated/`: committed generator output, including `go.mod` and `build_flags.go`
