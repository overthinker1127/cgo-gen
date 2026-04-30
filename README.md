# cgo-gen

`cgo-gen` is a Rust CLI that parses a conservative subset of C/C++ headers and generates C ABI wrappers, optional normalized IR dumps, and Go `cgo` facade files.

English is the default project README. Full user documentation lives under `docs/`.

## Documentation

- [English documentation](./docs/README.md)
- [한국어 문서](./docs/README.ko.md)
- [日本語ドキュメント](./docs/README.ja.md)
- [中文文档](./docs/README.zh.md)

## Quick Start

```bash
cgo-gen check --config path/to/config.yaml
cgo-gen generate --config path/to/config.yaml --dump-ir
```

## License

[MIT](./LICENSE)
