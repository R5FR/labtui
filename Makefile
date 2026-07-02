
.PHONY: debug build-release release-linux-musl test clippy clippy-pedantic install install-debug sort

ARGS=-l
# ARGS=-l -d ~/code/extern/kubernetes
# ARGS=-l -d ~/code/extern/linux
# ARGS=-l -d ~/code/git-bare-test.git -w ~/code/git-bare-test

profile:
	CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --features timing -- ${ARGS}

run-timing:
	cargo run --features=timing --release -- ${ARGS}

debug:
	RUST_BACKTRACE=true cargo run --features=timing -- ${ARGS}

build-release:
	cargo build --release --locked

release-mac: build-release
	strip target/release/labtui
	otool -L target/release/labtui
	ls -lisah target/release/labtui
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/labtui-mac.tar.gz ./labtui
	ls -lisah ./release/labtui-mac.tar.gz

release-mac-x86: build-apple-x86-release
	strip target/x86_64-apple-darwin/release/labtui
	otool -L target/x86_64-apple-darwin/release/labtui
	ls -lisah target/x86_64-apple-darwin/release/labtui
	mkdir -p release
	tar -C ./target/x86_64-apple-darwin/release/ -czvf ./release/labtui-mac-x86.tar.gz ./labtui
	ls -lisah ./release/labtui-mac-x86.tar.gz

release-win: build-release
	mkdir -p release
	tar -C ./target/release/ -czvf ./release/labtui-win.tar.gz ./labtui.exe
	cargo install cargo-wix --version 0.3.3 --locked
	cargo wix -p labtui --no-build --nocapture --output ./release/labtui-win.msi
	ls -l ./release/labtui-win.msi

release-linux-musl: build-linux-musl-release
	strip target/x86_64-unknown-linux-musl/release/labtui
	mkdir -p release
	tar -C ./target/x86_64-unknown-linux-musl/release/ -czvf ./release/labtui-linux-x86_64.tar.gz ./labtui

build-apple-x86-debug:
	cargo build --target=x86_64-apple-darwin

build-apple-x86-release:
	cargo build --release --target=x86_64-apple-darwin --locked

build-linux-musl-debug:
	cargo build --target=x86_64-unknown-linux-musl

build-linux-musl-release:
	cargo build --release --target=x86_64-unknown-linux-musl --locked

test-linux-musl:
	cargo nextest run --workspace --target=x86_64-unknown-linux-musl

release-linux-arm: build-linux-arm-release
	mkdir -p release

	aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/labtui
	arm-linux-gnueabihf-strip target/armv7-unknown-linux-gnueabihf/release/labtui
	arm-linux-gnueabihf-strip target/arm-unknown-linux-gnueabihf/release/labtui

	tar -C ./target/aarch64-unknown-linux-gnu/release/ -czvf ./release/labtui-linux-aarch64.tar.gz ./labtui
	tar -C ./target/armv7-unknown-linux-gnueabihf/release/ -czvf ./release/labtui-linux-armv7.tar.gz ./labtui
	tar -C ./target/arm-unknown-linux-gnueabihf/release/ -czvf ./release/labtui-linux-arm.tar.gz ./labtui

build-linux-arm-debug:
	cargo build --target=aarch64-unknown-linux-gnu
	cargo build --target=armv7-unknown-linux-gnueabihf
	cargo build --target=arm-unknown-linux-gnueabihf

build-linux-arm-release:
	cargo build --release --target=aarch64-unknown-linux-gnu --locked
	cargo build --release --target=armv7-unknown-linux-gnueabihf --locked
	cargo build --release --target=arm-unknown-linux-gnueabihf --locked

test:
	cargo nextest run --workspace

fmt:
	cargo fmt -- --check

clippy:
	cargo clippy --workspace --all-features

clippy-nightly:
	cargo +nightly clippy --workspace --all-features

check: fmt clippy test sort deny

check-nightly:
	cargo +nightly c
	cargo +nightly clippy --workspace --all-features
	cargo +nightly t

deny:
	cargo deny check

sort:
	tombi format --check

install:
	cargo install --path "." --offline --locked

install-timing:
	cargo install --features=timing --path "." --offline --locked

licenses:
	cargo bundle-licenses --format toml --output THIRDPARTY.toml

clean:
	cargo clean
