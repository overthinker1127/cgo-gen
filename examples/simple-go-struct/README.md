# simple-go-struct

Additional workflow:

- [Symlinked build workspace](./SYMLINKED_BUILD.md)

`Select/Next + handle-backed model` 패턴을 보여주는 예제입니다.

이 예제는 다음 흐름을 그대로 담습니다.

- `ThingModel` Go wrapper가 native handle 하나를 계속 들고 있음
- Go에서 `Set...`으로 같은 native 객체를 직접 수정할 수 있음
- `ThingApi.SelectThing(...)` / `ThingApi.NextThing(...)`가 그 같은 객체를 다시 채움
- `int32_t& pos` 같은 primitive reference는 Go에서 `*int32`로 write-back 됨

## 사용법

```bash
make -C examples/simple-go-struct gen
make -C examples/simple-go-struct build
make -C examples/simple-go-struct run
```

## 생성 결과

```text
pkg/demo/
  build_flags.go
  native_sources.cpp
  thing_model_wrapper.h
  thing_model_wrapper.cpp
  thing_model_wrapper.ir.yaml
  thing_model_wrapper.go
  thing_api_wrapper.h
  thing_api_wrapper.cpp
  thing_api_wrapper.ir.yaml
  thing_api_wrapper.go
```

## 예제 포인트

- `NewThingModel()`로 handle을 하나 만든 뒤 계속 재사용합니다.
- `item.SetName(...)`, `item.SetValue(...)`는 같은 native 객체에 바로 반영됩니다.
- `api.NextThing(&pos, item)`는 같은 `item.ptr`를 native에 넘겨서 내용을 덮어씁니다.
- `pos`는 Go 쪽 `*int32`로 전달되고 호출 뒤 갱신값이 다시 써집니다.
