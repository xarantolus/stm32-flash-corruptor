### Debug

In one shell, run

```sh
st-util
```

Then attach using gdb:
```sh
arm-none-eabi-gdb -ex "target extended-remote :4242" target/thumbv7em-none-eabi/release/stm32-flash-corruptor
```

Show backup register content:

```gdb
x/32w 0x40002850
```
