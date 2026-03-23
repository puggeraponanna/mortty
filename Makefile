.PHONY: all build run build-release run-release check fmt clean

all: build

build:
	cargo build

run:
	cargo run

build-release:
	cargo build --release

run-release:
	cargo run --release

bundle: build-release
	rm -rf Mortty.app
	mkdir -p Mortty.app/Contents/MacOS
	mkdir -p Mortty.app/Contents/Resources
	cp target/release/mortty Mortty.app/Contents/MacOS/
	cp Info.plist Mortty.app/Contents/
	@echo "Mortty.app created! You can double-click it in Finder or run 'open Mortty.app'"

check:
	cargo check
	cargo clippy

fmt:
	cargo fmt

clean:
	cargo clean
