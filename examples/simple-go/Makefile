default: build

GO              := go
PYTHON          := python
CURRENT_DIR     := .
BUILD_DIR       := $(CURRENT_DIR)/build
BUILD_BIN       := $(BUILD_DIR)/bin
GO_CACHE        := $(abspath $(CURRENT_DIR)/.gocache)
TARGET          := simple-go
CMD             := $(CURRENT_DIR)/cmd/simple-go
LINK_WORKSPACE  := $(BUILD_DIR)/linked-workspace
LINK_MODULE_DIR := $(LINK_WORKSPACE)/simplego
LINK_HELPER     := ../_scripts/link_example_workspace.py

.PHONY: gen
gen:
	@echo "Generating wrapper files..."
	@cd ../.. && cargo run --bin cgo-gen -- generate --config examples/simple-go/config.yaml --dump-ir
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
		--link simplego/go.mod=go.mod \
		--link simplego/cmd/simple-go=cmd/simple-go \
		--link simplego/pkg/foo=pkg/foo \
		--link simple-cpp=../simple-cpp
	@echo "Linked workspace ready: $(LINK_MODULE_DIR)"

.PHONY: build-linked
build-linked: link-workspace
	@echo "Building Go application from symlinked workspace..."
	@mkdir -p $(BUILD_BIN)
	@mkdir -p $(GO_CACHE)
	@cd $(LINK_MODULE_DIR) && GOCACHE=$(GO_CACHE) CGO_ENABLED=1 $(GO) build -o $(abspath $(BUILD_BIN))/$(TARGET)-linked ./cmd/simple-go
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
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.h
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.cpp
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.ir.yaml
	@echo "Clean complete"
