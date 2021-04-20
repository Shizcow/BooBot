rust_src := $(wildcard src/**)

TARGET_ARCH=arm-unknown-linux-gnueabihf

export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER=target/tools/arm-bcm2708/arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc


target/$(TARGET_ARCH)/release/twitch: $(rust_src) Cargo.toml target/tools
	cargo build --target=$(TARGET_ARCH) --release
	-echo "Binary built at target/$(TARGET_ARCH)/release/twitch"

target/tools:
	git clone --depth=1 https://github.com/raspberrypi/tools.git target/tools

clean:
	cargo clean
