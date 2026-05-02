# 09 Struct Fields

Public C/C++ struct fields exposed as Go getter and setter methods.

`SensorReading` has scalar fields, a fixed `char` buffer, and a const field. The generated Go facade exposes methods such as `GetSampleId`, `SetSampleId`, `GetTemperatureC`, `SetTemperatureC`, `GetLabel`, and `SetLabel`. The const `schema_version` field is read-only, so only `GetSchemaVersion` is emitted.

```bash
cargo run --bin cgo-gen -- check --config examples/09-struct-fields/config.yaml
cargo run --bin cgo-gen -- generate --config examples/09-struct-fields/config.yaml --dump-ir
```

- `input/sensor_reading.hpp`: input header with public struct fields
- `generated/`: committed generator output
