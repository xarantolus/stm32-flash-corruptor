# stm32-flash-corruptor

Trick an STM32L4R5 chip into corrupting its error checking information for a certain flash address.

## Why?

When reading from a corrupted flash address on STM32 devices with flash ECC, you get a non-maskable ECC detection interrupt. Handling of this interrupt can be important for some applications, e.g. anything potentially exposed to radiation, such as satellites. However, actually testing your handling code on the hardware is hard, as you cannot directly tell the chip to intentionally produce a wrong flash entry.

That's why this tool offers a way to do exactly that: specifically destroy the error correction information at a specific address. It does this by automatically binary-searching a timing where a reset interferes with the flash writing process.

### How to verify the exact address of the corrupted block

If the green LED comes on, the program has hit the correct spot. To verify the exact address, you can also attach via GDB, and then read the content of the `FLASH_ECCR` register:

```gdb
(gdb) x/wx 0x40022018
0x40022018:     0x80006000
```

Corruption worked if the first hex letter is `8` or `2`. The last 20 bit (or 5 hex letters) are the memory address that got corrupted. In this case, `0x6000` was given to corrupt, and it got hit exactly.

### Debug (STM32L4R5)

In one shell, run

```sh
st-util
```

Then attach using gdb:

```sh
arm-none-eabi-gdb -ex "target extended-remote :4242" target/thumbv7em-none-eabi/release/stm32-flash-corruptor
```

Use this command to show backup register content:

```gdb
x/32w 0x40002850
```
