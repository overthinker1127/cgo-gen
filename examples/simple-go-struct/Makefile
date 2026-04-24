default: build

GO              := go
PYTHON          := python
CURRENT_DIR     := .
BUILD_DIR       := $(CURRENT_DIR)/build
BUILD_BIN       := $(BUILD_DIR)/bin
GO_CACHE        := $(abspath $(CURRENT_DIR)/.gocache)
TARGET          := simple-go-struct
CMD             := $(CURRENT_DIR)/cmd/simple-go-struct
LINK_WORKSPACE  := $(BUILD_DIR)/linked-workspace
LINK_MODULE_DIR := $(LINK_WORKSPACE)/simplegostruct
LINK_HELPER     := ../_scripts/link_example_workspace.py

.PHONY: gen
gen:
	@echo "Generating handle-backed model/facade wrappers..."
	@cd ../.. && cargo run --bin cgo-gen -- generate --config examples/simple-go-struct/config.yaml --dump-ir
	@echo "Generation complete"

.PHONY: build
build: gen
	@echo "Building Go application..."
	@mkdir -p $(BUILD_BIN)
	@mkdir -p $(GO_CACHE)
	@GOCACHE=$(GO_CACHE) CGO_ENABLED=1 $(GO) build -o $(BUILD_BIN)/$(TARGET) $(CMD)
	@echo "Build complete: $(BUILD_BIN)/$(TARGET)"

.PHONY: link-workspace
link-workspace: gen
	@echo "Preparing symlinked build workspace..."
	@mkdir -p $(LINK_WORKSPACE)
	@$(PYTHON) $(LINK_HELPER) \
		--example-root $(CURRENT_DIR) \
		--workspace $(LINK_WORKSPACE) \
		--link simplegostruct/go.mod=go.mod \
		--link simplegostruct/cmd/simple-go-struct=cmd/simple-go-struct \
		--link simplegostruct/pkg/demo=pkg/demo \
		--link simplegostruct/cpp=cpp
	@echo "Linked workspace ready: $(LINK_MODULE_DIR)"

.PHONY: build-linked
build-linked: link-workspace
	@echo "Building Go application from symlinked workspace..."
	@mkdir -p $(BUILD_BIN)
	@mkdir -p $(GO_CACHE)
	@cd $(LINK_MODULE_DIR) && GOCACHE=$(GO_CACHE) CGO_ENABLED=1 $(GO) build -o $(abspath $(BUILD_BIN))/$(TARGET)-linked ./cmd/simple-go-struct
	@echo "Build complete: $(BUILD_BIN)/$(TARGET)-linked"

.PHONY: run-linked
run-linked: build-linked
	@echo "Running Go application from symlinked workspace..."
	@$(BUILD_BIN)/$(TARGET)-linked

.PHONY: run
run: build
	@echo "Running Go application..."
	@$(BUILD_BIN)/$(TARGET)

.PHONY: clean
clean:
	@echo "Cleaning up..."
	@rm -rf $(BUILD_DIR)
	@rm -rf $(GO_CACHE)
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_model_wrapper.go
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_api_wrapper.go
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_model_wrapper.h
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_model_wrapper.cpp
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_model_wrapper.ir.yaml
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_api_wrapper.h
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_api_wrapper.cpp
	@rm -f $(CURRENT_DIR)/pkg/demo/thing_api_wrapper.ir.yaml
	@echo "Clean complete"
