# About

[![Build Status](https://travis-ci.org/martinlindhe/dustbox-rs.svg?branch=master)](https://travis-ci.org/martinlindhe/dustbox-rs)

a x86 16-bit emulator with the goal of running old MS-DOS games.

This is a project I use to learn rust and improve my understanding of low level stuff.
In the current state, it runs a few simple demos and is not very impressive.
If you are looking for a more complete dos emulator, I suggest you check out [dosbox-x](https://github.com/joncampbell123/dosbox-x).

## Contributing

Any help and contributions are much welcome! 

## IRC

Join us in irc.mozilla.com #dustbox

## Tests

Run the basic tests with

```
cargo test
```

There is additional tests that are expensive, they also generate the tests/render/demo images.

In order to run the expensive tests you need to check out the dos-software-decoding repo in the parent directory and pass the `--ignored` flag to cargo:

    cd .. && git clone https://github.com/martinlindhe/dos-software-decoding && cd -
    cargo test -- --ignored


## Rust language use

We try to avoid using unstable features and unsafe code.
This is not a hard requirement.

Currently we make use of the current unstable features:
- [#[bench]](https://github.com/rust-lang/rust/issues/29553)


## License

Under [MIT](LICENSE)
