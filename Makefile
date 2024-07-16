.PHONY: corrupter flash

corrupter:
	cargo build --release --target=thumbv7em-none-eabi && \
	cargo objcopy --release --target=thumbv7em-none-eabi -- \
		-Obinary target/thumbv7em-none-eabi/release/stm32_flash_corruptor.bin

flash: corrupter
	st-flash --reset --flash=0x200000 write target/thumbv7em-none-eabi/release/stm32_flash_corruptor.bin 0x8000000
