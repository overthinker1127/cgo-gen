# Symlinked build workspace

Use this flow when the consuming Go build package should build from a directory outside the checked-in example tree, but the generated cgo package still expects the original relative native layout.

On Windows, symlink creation requires Developer Mode or an elevated shell.

```bash
make -C examples/simple-go-struct link-workspace
make -C examples/simple-go-struct build-linked
make -C examples/simple-go-struct run-linked
```

What `link-workspace` creates:

- `build/linked-workspace/simplegostruct/go.mod` -> `go.mod`
- `build/linked-workspace/simplegostruct/cmd/simple-go-struct` -> `cmd/simple-go-struct`
- `build/linked-workspace/simplegostruct/pkg/demo` -> `pkg/demo`
- `build/linked-workspace/simplegostruct/cpp` -> `cpp`

That `cpp` link preserves the `${SRCDIR}/../../cpp/...` includes compiled by the checked-in cgo package during `go build`.
