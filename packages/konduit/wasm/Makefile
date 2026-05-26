MAKEFLAGS += -j
OUT := konduit-wasm-nightly

.PHONY: all core black-box clean

all: | core black-box

core:
	@echo "📦 Building WASM (core types only)..."
	wasm-pack build --profile wasm --no-pack --target browser -d $(OUT)-core .

black-box:
	@echo "📦 Building WASM (full black-box API)..."
	wasm-pack build --profile wasm --no-pack --target browser -d $(OUT)-black-box . -F black-box-api

clean:
	@echo "🧹 Cleaning..."
	rm -rf $(OUT)-black-box $(OUT)-core
