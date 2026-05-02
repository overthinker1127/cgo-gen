# 10 Strings

C and C++ string values exposed through the Go facade.

`StringTool` combines borrowed C string returns, C string parameters, `std::string` parameters and returns, and a `std::string_view` parameter. The generated Go facade converts these to `string`, returning `(string, error)` when native code can produce a nil string pointer.

```bash
cargo run --bin cgo-gen -- check --config examples/10-strings/config.yaml
cargo run --bin cgo-gen -- generate --config examples/10-strings/config.yaml --dump-ir
```

- `input/string_tool.hpp`: input header with C and C++ string APIs
- `generated/`: committed generator output
