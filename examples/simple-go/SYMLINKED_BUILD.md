# Symlinked build workspace

Use this flow when the consuming Go build package should build from a directory outside the checked-in example tree, but the generated cgo package still expects the original relative native layout.

On Windows, symlink creation requires Developer Mode or an elevated shell.

```bash
make -C examples/simple-go link-workspace
make -C examples/simple-go build-linked
make -C examples/simple-go run-linked
```

What `link-workspace` creates:

- `build/linked-workspace/simplego/go.mod` -> `go.mod`
- `build/linked-workspace/simplego/cmd/simple-go` -> `cmd/simple-go`
- `build/linked-workspace/simplego/pkg/foo` -> `pkg/foo`
- `build/linked-workspace/simple-cpp` -> `../simple-cpp`

That extra `simple-cpp` link preserves the `${SRCDIR}/../../../simple-cpp/...` include path used by the checked-in cgo package during `go build`.
