# cgo-gen

[English](../README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md)

`cgo-gen` 是一个 Rust CLI，用于解析 C/C++ 头文件中较保守的子集，并生成：

- C ABI wrapper header/source
- 可选的 normalized IR dump
- 与生成的 native wrapper 放在同一输出目录中的 Go `cgo` facade file

它面向受控的 C/C++ 头文件表面，而不是任意现代 C++ 代码库。

## Quick Start

先运行 `check`，再用同一个 config 生成 wrapper：

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

这个流程会：

1. 读取 YAML config
2. 使用 `libclang` 解析头文件
3. 将声明规范化为 normalized IR
4. 将 wrapper files 生成到 `output.dir`
5. 按需输出生成的 `.ir.yaml` file

## Requirements

- Rust toolchain
- 运行时可发现的 `libclang`
- 处理非平凡头文件时需要 Clang-compatible compile environment
- 只有在实际 build 生成的 Go packages 时才需要 Go toolchain

### Clang And libclang

`cgo-gen` 使用 `libclang` 对 C/C++ 头文件进行 preprocess、parse 和 type-check。

- 如果 `libclang` 安装在非标准位置，可能需要设置 `LIBCLANG_PATH`

常见安装方式：

- Windows
  - `winget install LLVM.LLVM`
  - 必要时将 `LIBCLANG_PATH` 设置为 LLVM 的 `bin` 目录，例如 `D:\programs\LLVM\bin`
  - Mingw64 环境可使用 `pacman -S mingw64/mingw-w64-x86_64-clang`
- macOS
  - Homebrew: `brew install llvm`
  - MacPorts: `port install clang`
  - Homebrew LLVM 会把 `libclang.dylib` 安装到 `$(brew --prefix llvm)/lib`。
    如果 test binary 无法 load `libclang.dylib`，用下面方式运行 tests：
    ```bash
    DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" cargo test
    ```
  - 如果 `xcode-select -p` 指向过期或错误的 Xcode path，可以修正它：
    ```bash
    sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
    ```
    或者按 command 临时覆盖：
    ```bash
    DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
      DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" \
      cargo test --test overload_collisions
    ```
- Debian/Ubuntu
  - `apt install libclang-dev`
  - 如果本地也需要完整 Clang CLI，也安装 `clang`
- Arch
  - `pacman -S clang`
- Fedora
  - `dnf install clang-devel`
- OpenBSD
  - `pkg_add llvm`
  - 必要时设置 `LIBCLANG_PATH=/usr/local/lib`

如果 package manager 提供的 Clang/libclang 不够新，可能需要从源码构建。对这个 project 来说，只需要 Clang 相关部分，不需要完整 LLVM optional stack。

## Install

从 repository 直接运行：

```bash
cargo run --bin cgo-gen -- --help
```

或安装到本地：

```bash
cargo install --path .
cgo-gen --help
```

## Core Commands

`cgo-gen` 当前提供三个 subcommands：

- `generate --config <path> [--dump-ir] [--go-module <module-path>]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

常规流程：

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

如果只想查看 normalized model 而不写入 wrapper files，使用 `ir`：

```bash
cgo-gen ir --config path/to/config.yaml --format yaml
```

## Minimal Config

支持的 config surface 有意保持较小：

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

如果只想 wrap 精确的 entry header 列表，请用 `input.headers` 代替 `input.dir`：

```yaml
version: 1

input:
  headers:
    - path/to/include/widget.hpp
    - path/to/include/service.hpp
  clang_args:
    - -Ipath/to/include

output:
  dir: gen
```

关键行为：

- relative paths 会按 config file 所在位置解析
- unknown keys 会在 load 时被拒绝
- `input.dir` 会被递归扫描
- `input.headers` 是精确 file list，不能和 `input.dir` 同时使用
- listed headers include 的 dependency headers 会参与 parse，但只会为 `input.headers` 中列出的 files 生成 wrappers
- 生成的 `.go`, `.h`, `.cpp` 和可选 `.ir.yaml` files 都会写入 `output.dir`
- `output.go_version` 控制生成的 `go.mod` 中的 Go version，默认值是 `1.26`
- 设置 `--go-module <module-path>` 时，`generate` 也会写入 `go.mod` 和 `build_flags.go`

## Generated Output

对每个支持的 entry header，`generate` 可以输出：

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- 启用 `--dump-ir` 时输出 `<name>_wrapper.ir.yaml`

设置 `--go-module` 时还会输出：

- `go.mod`
- `build_flags.go`

这些 generated files 会被有意放在同一目录，方便 downstream `cgo` package 作为一个 package-local unit 编译。

## Go Module Output

如果希望 `output.dir` 表现为一个 standalone Go module，使用 `generate --go-module <module-path>`：

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

启用后，`generate` 还会写入：

- 包含 `module <module-path>` 和 `go <output.go_version>` 的 `go.mod`；默认值是 `1.26`
- `build_flags.go`

当前行为：

- `build_flags.go` 总是输出 `#cgo CFLAGS: -I${SRCDIR}`
- `#cgo CXXFLAGS` 只从 raw `input.clang_args` 导出
- 导出的 `CXXFLAGS` 只允许 `-I`, `-D`, `-std=...`
- 设置 `input.ldflags` 时，`build_flags.go` 也会输出 `#cgo LDFLAGS`

当生成目录本身需要被 import 并 build 为 Go package 时使用这个 mode。

## Config Options That Matter Most

刚开始不需要了解很多 knobs。当前支持的核心 keys 如下：

- `input.dir`: 用于 header discovery 和 translation-unit discovery 的递归 input root
- `input.headers`: 按 config file 所在位置解析的精确 entry header list；与 `input.dir` mutually exclusive
- `input.clang_args`: 额外的 libclang flags，例如 `-I...`, `-isystem...`, `-D...`, `-std=...`
- `input.owner`: 其 pointer return 应生成为 owned Go wrappers 的 qualified callable names
- `input.ldflags`: 转发到生成的 `build_flags.go` 的 linker flags
- `output.dir`: output directory
- `output.header`, `output.source`, `output.ir`: single-header generation 的可选显式 filenames

重要限制：

- 使用 multi-header generation 时，让 `output.header`, `output.source`, `output.ir` 保持默认值
- 生成的 C symbol naming 固定在 source 中，不能通过 YAML 配置
- `input.headers`, `input.clang_args`, `input.ldflags` 的 relative paths 会按 config file directory 解析
- 只有在 pointer return 确实转移 ownership 时才使用 `input.owner`，例如返回 `new` 分配对象的 factory method
- `input.owner` 按 `WidgetFactory::Create` 这样的 qualified callable name 匹配；如果同名 overload 存在，所有匹配 overload 都会被视为 owned
- env expansion 只支持 `$VAR`, `$(VAR)`, `${VAR}`

对于大型 library，请把想 wrap 的小型 header surface 放到 adapter directory 并用 `input.dir` 指向那里，或用 `input.headers` 指定精确 entry headers。

## Supported Today

- free functions
- non-template classes
- constructors and destructors
- 带 deterministic overload disambiguation 的 public methods
- 支持字段类型的 public struct field accessors
- primitive scalars 和常见 fixed-width aliases
- `const char*`, `char*`, `std::string`, `std::string_view`
- fixed-size primitive arrays 和 model arrays
- Go 中的 primitive pointer/reference write-back
- 支持的 API 使用的 named callback typedefs
- `struct timeval*` 和 `struct timeval&`
- 支持 object paths 的 handle-backed Go wrappers

## Not Supported Or Intentionally Limited

- `operator+` 和 `operator==` 等 operators
- `void (*cb)(int)` 这样的 raw inline function pointer parameters
- templates 和 STL-heavy APIs
- anonymous classes
- exception translation
- advanced inheritance modeling
- raw-unsafe 的 by-value object parameters or returns

不支持的 declarations 可能会被 skip，而不是 abort 整个 run。发生这种情况时，原因会记录在 normalized IR 的 `support.skipped_declarations` 中。

## License

[MIT](../LICENSE)
