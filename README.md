# cgo-gen

[한국어](./README.ko.md)

`cgo-gen` is a Rust CLI that parses a conservative subset of C/C++ headers and generates:

- C ABI wrapper headers and sources
- optional normalized IR dumps
- Go `cgo` facade files beside the generated native wrapper

It is designed for controlled C/C++ header surfaces, not for arbitrary modern C++ codebases.

## Quick Start

If you just want to see the current workflow end to end, use the checked-in example:

```bash
cargo run --bin cgo-gen -- check --config examples/simple-go/config.yaml
cargo run --bin cgo-gen -- generate --config examples/simple-go/config.yaml --dump-ir
make -C examples/simple-go run
```

That path exercises the actual supported flow in this repository:

1. load a YAML config
2. parse headers with `libclang`
3. normalize declarations into IR
4. generate wrapper files into `output.dir`
5. build or consume the generated Go package

## Requirements

- Rust toolchain
- `libclang` available at runtime
- a Clang-compatible compile environment for non-trivial headers
- Go toolchain only if you plan to build generated Go packages

### Clang And libclang

`cgo-gen` uses `libclang` to preprocess, parse, and type-check C/C++ headers.

- if `libclang` is installed in a non-standard location, set `LIBCLANG_PATH`

Typical install paths:

- Windows
  - `winget install LLVM.LLVM`
  - if needed, set `LIBCLANG_PATH` to the LLVM `bin` directory, for example `D:\programs\LLVM\bin`
  - for Mingw64, `pacman -S mingw64/mingw-w64-x86_64-clang`
- macOS
  - Homebrew: `brew install llvm`
  - MacPorts: `port install clang`
- Debian/Ubuntu
  - `apt install libclang-dev`
  - install `clang` as well if you need the full Clang CLI locally
- Arch
  - `pacman -S clang`
- Fedora
  - `dnf install clang-devel`
- OpenBSD
  - `pkg_add llvm`
  - if needed, set `LIBCLANG_PATH=/usr/local/lib`

If your package manager does not provide a recent enough Clang/libclang, build from source. For this project you only need the Clang pieces, not the full LLVM optional stack.

## Install

Run from the repository:

```bash
cargo run --bin cgo-gen -- --help
```

Or install locally:

```bash
cargo install --path .
cgo-gen --help
```

## Core Commands

`cgo-gen` currently exposes three subcommands:

- `generate --config <path> [--dump-ir] [--go-module <module-path>]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

Typical flow:

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

Use `ir` when you want to inspect the normalized model without writing wrapper files:

```bash
cgo-gen ir --config path/to/config.yaml --format yaml
```

## Minimal Config

The supported config surface is intentionally small:

```yaml
version: 1

input:
  dir: path/to/include
  clang_args:
    - -Ipath/to/include
    - -std=c++17
  owner:
    - WidgetFactory::Create
  ldflags:
    - -Lpath/to/lib
    - -lfoo

output:
  dir: gen
```

Key behaviors:

- relative paths are resolved from the config file location
- unknown keys are rejected at load time
- `input.dir` is scanned recursively
- generated `.go`, `.h`, `.cpp`, and optional `.ir.yaml` files are written together under `output.dir`
- when `--go-module <module-path>` is set, `generate` also writes `go.mod` and `build_flags.go`

## Generated Output

For each supported entry header, `generate` can emit:

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `<name>_wrapper.ir.yaml` when `--dump-ir` is enabled

When `--go-module` is set, it also writes:

- `go.mod`
- `build_flags.go`

The generated files are intentionally co-located so a downstream `cgo` package can compile them as one package-local unit.

## Go Module Output

Use `generate --go-module <module-path>` when you want `output.dir` to behave like a standalone Go module:

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

When enabled, `generate` also writes:

- `go.mod` with `module <module-path>` and `go 1.25`
- `build_flags.go`

Current behavior:

- `build_flags.go` always emits `#cgo CFLAGS: -I${SRCDIR}`
- `#cgo CXXFLAGS` are exported from raw `input.clang_args` only
- exported `CXXFLAGS` allow only `-I`, `-D`, and `-std=...`
- when `input.ldflags` is set, `build_flags.go` also emits `#cgo LDFLAGS`

Use this mode when the generated directory itself should be imported and built as a Go package.

## Config Options That Matter Most

You do not need many knobs to get started. These are the supported ones:

- `input.dir`: recursive input root used for header discovery and translation-unit discovery
- `input.clang_args`: extra libclang flags such as `-I...`, `-isystem...`, `-D...`, `-std=...`
- `input.owner`: qualified callable names whose pointer returns should be emitted as owned Go wrappers
- `input.ldflags`: linker flags forwarded into generated `build_flags.go`
- `output.dir`: output directory
- `output.header`, `output.source`, `output.ir`: optional explicit filenames for single-header generation

Important caveats:

- if you use multi-header generation, leave `output.header`, `output.source`, and `output.ir` at their defaults
- generated C symbol naming is fixed in source and is not configurable via YAML
- `input.clang_args` and `input.ldflags` resolve relative paths from the config file directory
- use `input.owner` only when a pointer return actually transfers ownership, for example a factory method that returns `new`-allocated objects
- `input.owner` matches by qualified callable name such as `WidgetFactory::Create`; if the same name is overloaded, every matching overload is treated as owned
- env expansion supports `$VAR`, `$(VAR)`, and `${VAR}` only

## Examples

Maintained examples:

- [`examples/simple-go`](./examples/simple-go): smallest end-to-end free-function flow
- [`examples/simple-go-struct`](./examples/simple-go-struct): handle-backed model and facade flow

Useful commands:

```bash
make -C examples/simple-go gen
make -C examples/simple-go build
make -C examples/simple-go run

make -C examples/simple-go-struct gen
make -C examples/simple-go-struct build
make -C examples/simple-go-struct run
```

## Supported Today

- free functions
- non-template classes
- constructors and destructors
- public methods with deterministic overload disambiguation
- public struct field accessors for supported field types
- primitive scalars and common fixed-width aliases
- `const char*`, `char*`, `std::string`, and `std::string_view`
- fixed-size primitive and model arrays
- primitive pointer/reference write-back in Go
- named callback typedefs used by supported APIs
- `struct timeval*` and `struct timeval&`
- handle-backed Go wrappers for supported object paths

## Not Supported Or Intentionally Limited

- operators such as `operator+` and `operator==`
- raw inline function pointer parameters such as `void (*cb)(int)`
- templates and STL-heavy APIs
- anonymous classes
- exception translation
- advanced inheritance modeling
- raw-unsafe by-value object parameters or returns

Unsupported declarations may be skipped instead of aborting the whole run. When that happens, the reason is recorded in `support.skipped_declarations` in the normalized IR.

## License

[MIT](./LICENSE)
