# stm32-flash-corruptor

Trick an STM32L4R5 chip into corrupting its error checking information for a certain flash address.

## Why?

When reading from a corrupted flash address on STM32 devices with flash ECC, you get a non-maskable ECC detection interrupt. Handling of this interrupt can be important for some applications, e.g. anything potentially exposed to radiation, such as satellites. However, actually testing your handling code on the hardware is hard, as you cannot directly tell the chip to intentionally produce a wrong flash entry.

That's why this tool offers a way to do exactly that: specifically destroy the error correction information at a specific address. It does this by automatically binary-searching a timing where a reset interferes with the flash writing process.

### How to use

Adjust the address at the top of the [`main.rs`](src/main.rs) file, plug in your debug connector for an STM32L4R5 chip, and then run `make flash`.

You should then see the blue LED of the chip blinking in intervals that get shorter. That is the binary search trying out how much it needs to wait for flash corruption to happen. The light will become seemingly off for some seconds (because the timing gets so short that the LED barely has a chance to be on), and afterwards, either the green or red LED will come on. Green means that the exact address was hit, red means that it was missed. In case of green, you can now flash the code you want to test against the ECC interrupt (make sure not to overwrite the page that now contains the error - erasing it will lead to the error going away), and in case the red LED comes on, you need to press the reset button to try again (sometimes, a power cycle to retry also helps).  

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

Take a look at the comments in [`main.rs`](src/main.rs) for more info on each register.
