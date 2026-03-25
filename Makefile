.PHONY: dev tui build-tui build-tauri build

dev:
	cd tauri && npm run tauri dev

tui:
	cargo run

build-tui:
	cargo build --release

build-tauri:
	cd tauri && npm run tauri build

build: build-tui build-tauri
