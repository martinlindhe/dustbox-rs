# About

[![Build Status](https://travis-ci.org/martinlindhe/dustbox-rs.svg?branch=master)](https://travis-ci.org/martinlindhe/dustbox-rs)

PC x86 emulator with the goal of easily running MS-DOS games on Windows, macOS and Linux.

In the current state, dustbox runs a some demos and is still in it's early stages.
If you are looking for a more complete dos emulator, I suggest you check out [dosbox-x](https://github.com/joncampbell123/dosbox-x).

## Rough status june 2019

16 bit CPU - 95%, interrupts are incomplete
32 bit CPU - 10%, some instructions supported
FPU - not started
disk - not started
PIT - 1%
PIC - 1%
ms-dos - 5%, simulating MS-DOS behavior (interrupts, command.com env)
ems/xms - 0%, extended memory manager
keyboard - 1%
mouse - 0%
cd-rom - 0%
cga - 5%
ega - 5%
vga - 5%
sound - 0%

## Contributing

Any help and contributions are much welcome!

## Running

To launch the dustbox_gtk debugger:

```
cargo run --package dustbox_gtk
```

then interact with the debugger using the input box ('help' to get started).

To launch the front-end:

```
cargo run --package dustbox_frontend path-to-dos-executable
```

## Tests

Run the basic tests with

```
cargo test --all
```

There is additional tests that are expensive, they also generate the tests/render/demo images.

In order to run the expensive tests you need to check out the dos-software-decoding repo in the parent directory and pass the `--ignored` flag to cargo:

    cd .. && git clone https://github.com/martinlindhe/dos-software-decoding && cd -
    cargo test --release -- --ignored

## License

Under [MIT](LICENSE)
