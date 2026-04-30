# 03 C++ Inventory

Two-header C++ example where a service writes into an item by reference.

```bash
cargo run --bin cgo-gen -- check --config examples/03-cpp-inventory/config.yaml
cargo run --bin cgo-gen -- generate --config examples/03-cpp-inventory/config.yaml --dump-ir
```

- `input/inventory_item.hpp`: input item class
- `input/inventory_service.hpp`: input service class
- `input/inventory_item.cpp`, `input/inventory_service.cpp`: matching implementations for reference
- `generated/`: committed generator output
