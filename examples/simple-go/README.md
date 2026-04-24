# simple-go

Additional workflow:

- [Symlinked build workspace](./SYMLINKED_BUILD.md)

`simple-cpp`를 Go + cgo에서 소비하는 가장 단순한 예제입니다.

목표:

- `Makefile`에서 `cgo-gen` wrapper 생성
- `go build`로 cgo + C++ wrapper 컴파일 확인
- 현재 구조가 end-to-end로 동작하는지 빠르게 검증

## 구조

- `config.yaml`
  - `examples/simple-cpp`의 `foo.hpp`를 대상으로 wrapper 생성
- `pkg/foo/foo.go`
  - generated `foo_wrapper.h/.cpp`를 사용하는 cgo 바인딩
- `pkg/foo/foo_source.cpp`
  - 원본 C++ 구현(`examples/simple-cpp/src/foo.cpp`)을 example package 안에서 함께 컴파일하도록 연결
- `cmd/simple-go/main.go`
  - 간단한 실행 예제
- `Makefile`
  - `gen`, `build`, `run`, `clean` 제공

## 사용법

```bash
make -C examples/simple-go gen
make -C examples/simple-go build
make -C examples/simple-go run
```

## 기대 결과

- `pkg/foo/foo_wrapper.h`
- `pkg/foo/foo_wrapper.cpp`
- `pkg/foo/foo_wrapper.ir.yaml`
- `build/bin/simple-go`

이 예제는 외부 전용 라이브러리 없이도,
우리 wrapper 생성기와 cgo 빌드 흐름이 맞물리는지 확인하는 목적입니다.
