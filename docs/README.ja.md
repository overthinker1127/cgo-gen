# cgo-gen

[English](../README.md) | [한국어](./README.ko.md) | [中文](./README.zh.md)

`cgo-gen` は、C/C++ ヘッダーの保守的なサブセットを解析し、次の生成物を出力する Rust CLI です。

- C ABI wrapper の header/source
- 任意の normalized IR dump
- 生成された native wrapper と同じ出力ディレクトリに置かれる Go `cgo` facade file

任意の現代的な C++ コードベース全体を処理するためのツールではなく、制御されたヘッダー表面を安定して包むためのツールです。

## Quick Start

まず `check` を実行し、同じ config で wrapper を生成します。

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

この流れでは次を行います。

1. YAML config を読み込む
2. `libclang` でヘッダーを解析する
3. 宣言を normalized IR に正規化する
4. wrapper files を `output.dir` に生成する
5. 必要に応じて生成された `.ir.yaml` file を出力する

## Requirements

- Rust toolchain
- 実行時に見つけられる `libclang`
- 実用的なヘッダーを扱う場合は Clang 互換の compile environment
- 生成された Go package を実際に build する場合のみ Go toolchain

### Clang And libclang

`cgo-gen` は `libclang` を使って C/C++ ヘッダーを preprocess、parse、type-check します。

- `libclang` が標準位置にない場合は `LIBCLANG_PATH` の設定が必要になることがあります

一般的なインストール方法:

- Windows
  - `winget install LLVM.LLVM`
  - 必要なら `LIBCLANG_PATH` を LLVM の `bin` ディレクトリに設定します。例: `D:\programs\LLVM\bin`
  - Mingw64 では `pacman -S mingw64/mingw-w64-x86_64-clang`
- macOS
  - Homebrew: `brew install llvm`
  - MacPorts: `port install clang`
  - Homebrew LLVM は `libclang.dylib` を `$(brew --prefix llvm)/lib` 以下にインストールします。
    test binary が `libclang.dylib` を load できない場合は次のように実行します。
    ```bash
    DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" cargo test
    ```
  - `xcode-select -p` が古い、または誤った Xcode path を指している場合は修正します。
    ```bash
    sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
    ```
    sudo なしで command 単位に回避する場合:
    ```bash
    DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
      DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" \
      cargo test --test overload_collisions
    ```
- Debian/Ubuntu
  - `apt install libclang-dev`
  - ローカルで Clang CLI も必要なら `clang` package もインストールします
- Arch
  - `pacman -S clang`
- Fedora
  - `dnf install clang-devel`
- OpenBSD
  - `pkg_add llvm`
  - 必要なら `LIBCLANG_PATH=/usr/local/lib` を設定します

package manager が十分に新しい Clang/libclang を提供しない場合は source build が必要になることがあります。この project では Clang 関連の構成要素だけで十分です。

## Install

repository から直接実行:

```bash
cargo run --bin cgo-gen -- --help
```

ローカル CLI としてインストール:

```bash
cargo install --path .
cgo-gen --help
```

## Core Commands

`cgo-gen` は現在 3 つの subcommand を提供します。

- `generate --config <path> [--dump-ir] [--go-module <module-path>]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

通常の流れ:

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

wrapper file を書き出さず normalized model だけ確認したい場合は `ir` を使います。

```bash
cgo-gen ir --config path/to/config.yaml --format yaml
```

## Minimal Config

対応している config surface は意図的に小さくしています。

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

正確な entry header list だけを wrap したい場合は、`input.dir` の代わりに `input.headers` を使います。

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

主な動作:

- relative path は config file の場所を基準に解決されます
- 未対応の key は load 時に error になります
- `input.dir` は再帰的に scan されます
- `input.headers` は正確な file list で、`input.dir` と同時には使えません
- list された header が include する dependency header は parse されますが、wrapper は `input.headers` に明示した file にだけ生成されます
- 生成される `.go`, `.h`, `.cpp`, 任意の `.ir.yaml` file はすべて `output.dir` に置かれます
- `output.go_version` は生成される `go.mod` の Go version を制御し、default は `1.26` です
- `--go-module <module-path>` を指定すると、`generate` は `go.mod` と `build_flags.go` も出力します

## Generated Output

対応している entry header ごとに、`generate` は通常次を出力できます。

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `--dump-ir` 有効時の `<name>_wrapper.ir.yaml`

`--go-module` を使う場合は追加で次も出力します。

- `go.mod`
- `build_flags.go`

これらの file は downstream `cgo` package が同じ package-local unit として compile できるよう、意図的に同じ場所へまとめられます。

## Go Module Output

`output.dir` 自体を standalone Go module のように扱いたい場合は `generate --go-module <module-path>` を使います。

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

有効にすると、`generate` は追加で次を出力します。

- `module <module-path>` と `go <output.go_version>` を含む `go.mod`; default は `1.26`
- `build_flags.go`

現在の動作:

- `build_flags.go` は常に `#cgo CFLAGS: -I${SRCDIR}` を出力します
- `#cgo CXXFLAGS` は raw `input.clang_args` からのみ export されます
- export される `CXXFLAGS` は `-I`, `-D`, `-std=...` だけを許可します
- `input.ldflags` が設定されている場合、`build_flags.go` は `#cgo LDFLAGS` も出力します

生成ディレクトリ自体を Go package として import/build したい場合にこの mode を使います。

## Config Options That Matter Most

最初から多くの knob を知る必要はありません。現在対応している主な key は次の通りです。

- `input.dir`: header discovery と translation-unit discovery に使う再帰 input root
- `input.headers`: config file の場所を基準に解決される正確な entry header list; `input.dir` とは mutually exclusive
- `input.clang_args`: `-I...`, `-isystem...`, `-D...`, `-std=...` などの追加 libclang flags
- `input.owner`: pointer return を owned Go wrapper として出力する qualified callable name
- `input.ldflags`: 生成される `build_flags.go` に渡す linker flags
- `output.dir`: output directory
- `output.header`, `output.source`, `output.ir`: single-header generation 用の任意の explicit filename

重要な注意点:

- multi-header generation を使う場合、`output.header`, `output.source`, `output.ir` は default のままにしてください
- 生成される C symbol naming は source に固定されており、YAML では変更できません
- `input.headers`, `input.clang_args`, `input.ldflags` の relative path は config file directory を基準に解決されます
- `input.owner` は factory method のように pointer return が実際に ownership を渡す場合だけ使ってください
- `input.owner` は `WidgetFactory::Create` のような qualified callable name で match します。同名 overload がある場合は、すべて owned として扱われます
- env expansion は `$VAR`, `$(VAR)`, `${VAR}` のみ対応します

大きな library では、wrap したい小さな header surface を adapter directory に置いて `input.dir` に指定するか、`input.headers` で正確な entry header を指定してください。

## Supported Today

- free functions
- non-template classes
- constructors and destructors
- deterministic overload disambiguation が適用される public methods
- 対応 field type に対する public struct field accessors
- primitive scalars と一般的な fixed-width aliases
- `const char*`, `char*`, `std::string`, `std::string_view`
- fixed-size primitive arrays と model arrays
- Go での primitive pointer/reference write-back
- 対応 API で使われる named callback typedefs
- `struct timeval*` と `struct timeval&`
- 対応 object path に対する handle-backed Go wrappers

## Not Supported Or Intentionally Limited

- `operator+` や `operator==` などの operators
- `void (*cb)(int)` のような raw inline function pointer parameters
- templates と STL-heavy APIs
- anonymous classes
- exception translation
- advanced inheritance modeling
- raw-unsafe な by-value object parameters or returns

未対応の宣言は、全体の実行を abort せず skip されることがあります。その場合、理由は normalized IR の `support.skipped_declarations` に記録されます。

## License

[MIT](../LICENSE)
