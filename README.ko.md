# cgo-gen

[English](./README.md)

`cgo-gen`은 보수적인 C/C++ 헤더 subset을 파싱해서 아래 산출물을 만드는 Rust CLI입니다.

- C ABI wrapper header/source
- 선택적 normalized IR dump
- 같은 출력 디렉터리에 놓이는 Go `cgo` facade 파일

임의의 현대 C++ 전체를 처리하는 도구가 아니라, 통제 가능한 헤더 표면을 안정적으로 감싸는 도구에 가깝습니다.

## 빠른 시작

현재 저장소에서 실제로 유지되는 가장 짧은 흐름은 예제 하나를 그대로 돌려보는 것입니다.

```bash
cargo run --bin cgo-gen -- check --config examples/01-c-library/config.yaml
cargo run --bin cgo-gen -- generate --config examples/01-c-library/config.yaml --dump-ir
```

이 흐름은 저장소의 현재 지원 경로를 그대로 보여줍니다.

1. YAML config 로드
2. `libclang`으로 헤더 파싱
3. 선언을 normalized IR로 정규화
4. `output.dir` 아래에 wrapper 파일 생성
5. 커밋된 `.h`, `.cpp`, `.go`, `.ir.yaml` 생성 결과 확인

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

핵심 동작:

- 상대 경로는 config 파일 위치를 기준으로 해석됩니다.
- 지원하지 않는 키는 로드 시점에 오류로 처리됩니다.
- `input.dir`는 재귀적으로 스캔됩니다.
- 생성되는 `.go`, `.h`, `.cpp`, 선택적 `.ir.yaml` 파일은 모두 `output.dir` 아래에 함께 놓입니다.
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

## Go Module 출력

`output.dir` 자체를 독립적인 Go module처럼 쓰고 싶다면 `generate --go-module <module-path>`를 사용합니다.

```bash
cgo-gen generate --config path/to/config.yaml --go-module example.com/acme/foo
```

이 옵션을 주면 추가로:

- `module <module-path>`와 `go 1.25`가 들어간 `go.mod`
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
- `input.clang_args`: `-I`, `-isystem`, `-D`, `-std=...` 같은 추가 libclang 인자
- `input.owner`: pointer return을 owned Go wrapper로 강제할 qualified callable name 목록
- `input.ldflags`: 생성되는 `build_flags.go`에 전달할 링커 플래그
- `output.dir`: 출력 디렉터리
- `output.header`, `output.source`, `output.ir`: single-header 생성에서만 쓰는 선택적 파일명 override

주의할 점:

- multi-header generation에서는 `output.header`, `output.source`, `output.ir`를 기본값으로 두는 편이 안전합니다.
- 생성되는 C symbol naming은 코드에 고정돼 있으며 YAML로 바꿀 수 없습니다.
- `input.clang_args`와 `input.ldflags`의 상대 경로는 config 파일 위치 기준으로 해석됩니다.
- `input.owner`는 factory method처럼 pointer return이 실제로 ownership을 넘기는 경우에만 사용해야 합니다.
- `input.owner`는 `WidgetFactory::Create` 같은 qualified callable name으로 매칭되며, 같은 이름의 overload가 있으면 모두 owned로 처리됩니다.
- env 확장은 `$VAR`, `$(VAR)`, `${VAR}`만 지원합니다.

## 예제

작게 시작해서 점진적으로 넓히는 예제입니다.

- [`examples/01-c-library`](./examples/01-c-library): C 스타일 free function
- [`examples/02-cpp-class`](./examples/02-cpp-class): C++ class와 free function
- [`examples/03-cpp-inventory`](./examples/03-cpp-inventory): service가 item reference를 채우는 두 개의 C++ header 예제
- [`examples/04-go-module`](./examples/04-go-module): `--go-module`을 붙인 생성 결과 예제

```bash
cargo run --bin cgo-gen -- check --config examples/01-c-library/config.yaml
cargo run --bin cgo-gen -- generate --config examples/01-c-library/config.yaml --dump-ir
cargo run --bin cgo-gen -- check --config examples/02-cpp-class/config.yaml
cargo run --bin cgo-gen -- generate --config examples/02-cpp-class/config.yaml --dump-ir
cargo run --bin cgo-gen -- check --config examples/03-cpp-inventory/config.yaml
cargo run --bin cgo-gen -- generate --config examples/03-cpp-inventory/config.yaml --dump-ir
cargo run --bin cgo-gen -- check --config examples/04-go-module/config.yaml
cargo run --bin cgo-gen -- generate --config examples/04-go-module/config.yaml --dump-ir --go-module example.com/cgo-gen/examples/04-go-module/generated
```

큰 라이브러리는 현재 `cgo-gen`이 `input.dir`를 재귀적으로 스캔합니다. 지금은 감쌀 대상만 담은 작은 adapter header directory를 만들고 그 경로를 `input.dir`로 지정하는 방식이 권장됩니다. 명시적인 header/function 선택 config는 이후 개선 항목입니다.

### cwrap과 비교

[`cwrap`](https://github.com/h12w/cwrap)은 C 라이브러리용 Go wrapper generator이고, package struct 기반 API를 사용합니다. README에는 `NamePattern`, `Excluded`, `TypeRule`, `BoolTypes` 같은 선택/커스터마이즈 필드가 나오며, 예제에는 [GMime](https://github.com/h12w/cwrap/blob/master/examples/gmime/gen_test.go) 같은 실제 라이브러리 케이스가 있습니다.

`cgo-gen`은 현재 더 작은 YAML surface와 directory 전체 스캔을 사용합니다. 지금의 권장 흐름은 작은 adapter header directory에서 시작하고, `generated/*.ir.yaml`을 확인한 뒤 노출 범위를 의도적으로 넓히는 것입니다.

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

[MIT](./LICENSE)
