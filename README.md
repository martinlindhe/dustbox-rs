# About

[![Join the chat at https://gitter.im/martinlindhe/dustbox-rs](https://badges.gitter.im/martinlindhe/dustbox-rs.svg)](https://gitter.im/martinlindhe/dustbox-rs?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) [![Build Status](https://travis-ci.org/martinlindhe/dustbox-rs.svg?branch=master)](https://travis-ci.org/martinlindhe/dustbox-rs)

A x86 emulator with the goal of running old MS-DOS games.

This is a project I use to learn rust and improve my understanding of low level stuff.
In the current state, it runs a few simple demos and is not very impressive.
If you are looking for a more complete dos emulator, I suggest you check out [dosbox-x](https://github.com/joncampbell123/dosbox-x).

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
