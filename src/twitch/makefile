rust_src := $(wildcard src/**)

TARGET_ARCH=arm-unknown-linux-gnueabihf

PI_IP=192.168.43.129

export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER=target/tools/arm-bcm2708/arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc


target/$(TARGET_ARCH)/release/twitch: $(rust_src) Cargo.toml target/tools
	cargo build --target=$(TARGET_ARCH) --release
	-echo "Binary built at target/$(TARGET_ARCH)/release/twitch"

target/tools:
	git clone --depth=1 https://github.com/raspberrypi/tools.git target/tools

clean:
	cargo clean

upload: target/$(TARGET_ARCH)/release/twitch
	tar cf - target/$(TARGET_ARCH)/release/twitch src/twitch_config.toml | ssh pi@$(PI_IP) "sh -c 'D=\`mktemp -d\`; tar xf - -C \$$D; \$$D/target/$(TARGET_ARCH)/release/twitch -c \$$D/src/twitch_config.toml'"
