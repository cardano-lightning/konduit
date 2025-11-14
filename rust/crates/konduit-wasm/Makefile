MAKEFLAGS += -j
OUT := konduit-wasm

.PHONY: all browser nodejs clean

all: browser nodejs

browser:
	@echo "📦 Building WASM (browser)..."
	wasm-pack build --release --target browser -d $(OUT)-browser

nodejs:
	@echo "📦 Building WASM (nodejs)..."
	wasm-pack build --release --target nodejs -d $(OUT)-nodejs

clean:
	@echo "🧹 Cleaning..."
	rm -rf $(OUT)-browser $(OUT)-nodejs
