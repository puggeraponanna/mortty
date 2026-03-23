.PHONY: all build run check fmt clean

all: build

build:
	cargo build

run:
	cargo run

check:
	cargo check
	cargo clippy

fmt:
	cargo fmt

clean:
	cargo clean
