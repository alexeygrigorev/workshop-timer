.PHONY: build run release screenshot clean kill help

help:
	@echo "Targets:"
	@echo "  build       cargo build (debug)"
	@echo "  run         build and run the overlay"
	@echo "  release     cargo build --release"
	@echo "  screenshot  run with screenshot env var; saves scripts/rendered.png"
	@echo "  kill        stop a running workshop-timer process"
	@echo "  clean       cargo clean"

build:
	cargo build

run: kill
	cargo run

release:
	cargo build --release

screenshot: kill
	WORKSHOP_TIMER_SCREENSHOT=scripts/rendered.png cargo run

kill:
	-@taskkill //IM workshop-timer.exe //F >/dev/null 2>&1 || true

clean:
	cargo clean
