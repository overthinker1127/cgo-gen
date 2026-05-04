# cgo-gen

[English](../README.md) | [日本語](./README.ja.md) | [中文](./README.zh.md)

`cgo-gen`은 보수적인 C/C++ 헤더 subset을 파싱해서 아래 산출물을 만드는 Rust CLI입니다.

- C ABI wrapper header/source
- 선택적 normalized IR dump
- 같은 출력 디렉터리에 놓이는 Go `cgo` facade 파일

임의의 현대 C++ 전체를 처리하는 도구가 아니라, 통제 가능한 헤더 표면을 안정적으로 감싸는 도구에 가깝습니다.

## 빠른 시작

먼저 `check`를 실행한 뒤 같은 config로 wrapper를 생성합니다.

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

이 흐름은 아래 단계를 수행합니다.

1. YAML config 로드
2. `libclang`으로 헤더 파싱
3. 선언을 normalized IR로 정규화
4. `output.dir` 아래에 wrapper 파일 생성
5. 필요하면 생성된 `.ir.yaml` 파일 출력

## 요구사항

- Rust toolchain
- 런타임에 발견 가능한 `libclang`
- 실사용 헤더를 다룰 때는 Clang 호환 compile 환경
- 생성된 Go 패키지를 실제로 빌드할 때만 Go toolchain

### Clang 및 libclang

`cgo-gen`은 `libclang`을 사용해 C/C++ 헤더를 전처리하고, 파싱하고, 타입 체크합니다.

- `libclang`이 표준 위치에 없으면 `LIBCLANG_PATH`를 설정해야 할 수 있습니다

일반적인 설치 방법:

- Windows
  - `winget install LLVM.LLVM`
  - 필요하면 `LIBCLANG_PATH`를 LLVM `bin` 디렉터리로 설정합니다. 예: `D:\programs\LLVM\bin`
  - Mingw64 환경이라면 `pacman -S mingw64/mingw-w64-x86_64-clang`
- macOS
  - Homebrew: `brew install llvm`
  - MacPorts: `port install clang`
  - Homebrew LLVM은 `libclang.dylib`를 `$(brew --prefix llvm)/lib` 아래에 설치합니다.
    테스트 바이너리가 `libclang.dylib`를 로드하지 못하면 아래처럼 실행합니다.
    ```bash
    DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" cargo test
    ```
  - `xcode-select -p`가 오래됐거나 잘못된 Xcode 경로를 가리키면 경로를 고칩니다.
    ```bash
    sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
    ```
    sudo 없이 명령 단위로만 우회하려면 아래처럼 실행합니다.
    ```bash
    DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
      DYLD_LIBRARY_PATH="$(brew --prefix llvm)/lib" \
      cargo test --test overload_collisions
    ```
- Debian/Ubuntu 계열
  - `apt install libclang-dev`
  - 로컬에서 Clang CLI도 같이 써야 하면 `clang` 패키지도 설치하는 편이 좋습니다
- Arch
  - `pacman -S clang`
- Fedora
  - `dnf install clang-devel`
- OpenBSD
  - `pkg_add llvm`
  - 필요하면 `LIBCLANG_PATH=/usr/local/lib`를 설정합니다

패키지 매니저에서 충분히 최신 Clang/libclang을 제공하지 않으면 소스 빌드가 필요할 수 있습니다. 이 프로젝트 기준으로는 Clang 관련 구성요소만 있으면 충분합니다.

## 설치

저장소에서 바로 실행:

```bash
cargo run --bin cgo-gen -- --help
```

로컬 CLI로 설치:

```bash
cargo install --path .
cgo-gen --help
```

## 핵심 명령

현재 제공하는 서브커맨드는 세 가지입니다.

- `generate --config <path> [--dump-ir] [--go-module <module-path>]`
- `ir --config <path> [--output <path>] [--format yaml|json]`
- `check --config <path>`

일반적인 흐름은 아래 두 줄이면 충분합니다.

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

wrapper를 쓰지 않고 normalized IR만 확인하고 싶다면:

```bash
cgo-gen ir --config path/to/config.yaml --format yaml
```

## 최소 설정

현재 지원하는 config surface는 의도적으로 작습니다.

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

정확한 엔트리 헤더 목록만 감싸고 싶다면 `input.dir` 대신 `input.headers`를 사용합니다.

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

핵심 동작:

- 상대 경로는 config 파일 위치를 기준으로 해석됩니다.
- 지원하지 않는 키는 로드 시점에 오류로 처리됩니다.
- `input.dir`는 재귀적으로 스캔됩니다.
- `input.headers`는 정확한 파일 목록이며 `input.dir`와 함께 사용할 수 없습니다.
- 목록에 있는 헤더가 include하는 dependency header는 파싱에는 쓰이지만, wrapper는 `input.headers`에 명시된 파일에 대해서만 생성됩니다.
- 생성되는 `.go`, `.h`, `.cpp`, 선택적 `.ir.yaml` 파일은 모두 `output.dir` 아래에 함께 놓입니다.
- `output.go_version`은 생성되는 `go.mod`의 Go 버전을 제어하며 기본값은 `1.26`입니다.
- `--go-module <module-path>`를 주면 `generate`가 `go.mod`와 `build_flags.go`도 함께 생성합니다.

## 생성 결과

지원되는 엔트리 헤더마다 `generate`는 보통 아래 파일들을 만듭니다.

- `<name>_wrapper.h`
- `<name>_wrapper.cpp`
- `<name>_wrapper.go`
- `--dump-ir` 사용 시 `<name>_wrapper.ir.yaml`

`--go-module`을 사용하면 추가로 아래 파일도 생성합니다.

- `go.mod`
- `build_flags.go`

이 파일들을 한 디렉터리에 모아두는 이유는 downstream `cgo` 패키지가 한 위치에서 함께 빌드할 수 있게 하기 위해서입니다.
IR `source_headers` 항목은 생성된 `.ir.yaml` 파일 기준 상대경로로 기록되어, checked-in 예제가 clone 위치에 묶이지 않습니다.

## Go Module 출력

`output.dir` 자체를 독립적인 Go module처럼 쓰고 싶다면 `generate --go-module <module-path>`를 사용합니다.

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

이 옵션을 주면 추가로:

- `module <module-path>`와 `go <output.go_version>`이 들어간 `go.mod`; 기본값은 `1.26`
- `build_flags.go`

가 생성됩니다.

현재 동작은 다음과 같습니다.

- `build_flags.go`는 항상 `#cgo CFLAGS: -I${SRCDIR}`를 포함합니다.
- `#cgo CXXFLAGS`는 raw `input.clang_args`에서만 추출합니다.
- export되는 `CXXFLAGS`는 `-I`, `-D`, `-std=...`만 허용합니다.
- `input.ldflags`가 있으면 `build_flags.go`에 `#cgo LDFLAGS`도 생성합니다.

생성 디렉터리 자체를 Go 패키지로 import하고 빌드하려는 경우 이 모드를 사용하면 됩니다.

## 자주 쓰는 설정 키

처음에는 많은 옵션을 알 필요가 없습니다. 현재 지원하는 핵심 키는 아래 정도입니다.

- `input.dir`: header discovery와 translation-unit discovery에 쓰이는 재귀 입력 루트
- `input.headers`: config 파일 위치 기준으로 해석되는 정확한 엔트리 헤더 목록; `input.dir`와 상호 배타적
- `input.clang_args`: `-I`, `-isystem`, `-D`, `-std=...` 같은 추가 libclang 인자
- `input.owner`: pointer return을 owned Go wrapper로 강제할 qualified callable name 목록
- `input.ldflags`: 생성되는 `build_flags.go`에 전달할 링커 플래그
- `output.dir`: 출력 디렉터리
- `output.header`, `output.source`, `output.ir`: single-header 생성에서만 쓰는 선택적 파일명 override

주의할 점:

- multi-header generation에서는 `output.header`, `output.source`, `output.ir`를 기본값으로 두는 편이 안전합니다.
- 생성되는 C symbol naming은 코드에 고정돼 있으며 YAML로 바꿀 수 없습니다.
- `input.headers`, `input.clang_args`, `input.ldflags`의 상대 경로는 config 파일 위치 기준으로 해석됩니다.
- `input.owner`는 factory method처럼 pointer return이 실제로 ownership을 넘기는 경우에만 사용해야 합니다.
- `input.owner`는 `WidgetFactory::Create` 같은 qualified callable name으로 매칭되며, 같은 이름의 overload가 있으면 모두 owned로 처리됩니다.
- env 확장은 `$VAR`, `$(VAR)`, `${VAR}`만 지원합니다.

큰 라이브러리는 감쌀 대상만 담은 작은 adapter header directory를 만들고 `input.dir`로 지정하거나, `input.headers`로 정확한 엔트리 헤더를 지정하는 방식이 권장됩니다.

## 현재 지원 범위

- free function
- non-template class
- constructor / destructor
- deterministic overload disambiguation이 적용되는 public method
- 지원되는 필드 타입에 대한 public struct field accessor
- primitive scalar와 일반적인 fixed-width alias
- `const char*`, `char*`, `std::string`, `std::string_view`
- fixed-size primitive array와 model array
- Go에서의 primitive pointer/reference write-back
- 지원되는 API에서 사용되는 named callback typedef
- `struct timeval*`, `struct timeval&`
- 지원되는 object 경로에 대한 handle-backed Go wrapper

## 비지원 또는 의도적 제한

- `operator+`, `operator==` 같은 operator
- `void (*cb)(int)` 같은 raw inline function pointer parameter
- template와 STL-heavy API
- anonymous class
- exception translation
- advanced inheritance modeling
- raw-unsafe한 by-value object parameter / return

일부 비지원 선언은 전체 실행을 abort 하지 않고 skip 됩니다. 이런 경우 이유는 normalized IR의 `support.skipped_declarations`에 기록됩니다.

## 라이선스

[MIT](../LICENSE)
