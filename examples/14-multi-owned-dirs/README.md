# 14 Multi Owned Dirs

Two owned include directories where `A/A.h` includes `../B/B.h` and stores `B`
by value.

```bash
cargo run --bin cgo-gen -- check --config examples/14-multi-owned-dirs/config.yaml
cargo run --bin cgo-gen -- generate --config examples/14-multi-owned-dirs/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/14-multi-owned-dirs/generated
```

- `A/A.h`: owns `A` and includes `../B/B.h`
- `B/B.h`: owns `B`
- `generated/`: generated wrappers
