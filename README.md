# About

[![Build Status](https://travis-ci.org/martinlindhe/dustbox-rs.svg?branch=master)](https://travis-ci.org/martinlindhe/dustbox-rs)

Early WIP PC x86 emulator with the goal of easily running MS-DOS games on Windows, macOS and Linux.

In the current state, dustbox runs a some demos and is still in it's early stages.
If you are looking for a more complete dos emulator, I suggest you check out [dosbox-x](https://github.com/joncampbell123/dosbox-x).

## Rough status june 2019

| Component  | Status | Notes                                                    |
| ---------- | ------ | -------------------------------------------------------- |
| 16 bit CPU | 95%    | interrupts are incomplete                                |
| 32 bit CPU | 20%    | some instructions supported                              |
| FPU        | -      | not started                                              |
| disk       | -      | not started                                              |
| PIT        | 1%     |                                                          |
| PIC        | 1%     |                                                          |
| MS-DOS     | 5%     | simulating MS-DOS behavior (interrupts, command.com env) |
| EMS/XMS    | -      | extended memory managers                                 |
| Keyboard   | 1%     |                                                          |
| Mouse      | 25%    |                                                          |
| CD-ROM     | -      | not started                                              |
| CGA        | 5%     |                                                          |
| EGA        | 5%     |                                                          |
| VGA        | 5%     |                                                          |s
| Sound      | -      | not started                                              |

## Contributing

Any help and contributions are much welcome!

## Dependencies

On Ubuntu, use `sudo apt install nasm libgtk-3-dev libcairo2-dev libpango1.0-dev libatk1.0-dev gdk-pixbuf2.0-dev libsdl2-dev libsdl2-gfx-dev`

On Windows, use `vcpkg install sdl2 gtk` + `scoop install nasm`

On macOS, use `brew install nasm sdl2 sdl2_gfx`

## Running

To launch the debugger:

```sh
cargo run --package dustbox_debugger
```

then interact with the debugger using the input box ('help' to get started).

To launch the front-end:

```sh
cargo run --package dustbox_frontend path-to-dos-executable
```

## Tests

To run all normal tests

```sh
cargo test --all
```

There is additional tests that are expensive, they also generate the tests/render/demo images.

In order to run the expensive tests you need to check out the dos-software-decoding repo in the parent directory and pass the `--ignored` flag to cargo:

```sh
cd .. && git clone --depth 1 https://github.com/martinlindhe/dos-software-decoding && cd -
cargo test --release -- --ignored
```

## License

Under [MIT](LICENSE)
