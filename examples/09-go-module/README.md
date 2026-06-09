# 09 Go Module

Minimal example for generation with `--go-module`.

`input.dirs` is exported through `build_flags.go` so the generated Go module
can compile against the input headers. Native implementations such as
`input/score.c` still need to be built or linked by the consuming package.

```bash
cargo run --bin cgo-gen -- check --config examples/09-go-module/config.yaml
cargo run --bin cgo-gen -- generate --config examples/09-go-module/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/09-go-module/generated
```

- `input/score.h`: input header
- `input/score.c`: matching implementation for reference
- `generated/`: committed generator output, including `go.mod` and `build_flags.go`
