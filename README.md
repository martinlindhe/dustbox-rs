# About

[![Build Status](https://travis-ci.org/martinlindhe/dustbox-rs.svg?branch=master)](https://travis-ci.org/martinlindhe/dustbox-rs)

a x86 16-bit emulator with the goal of running old MS-DOS games.

This is a project I use to learn rust and improve my understanding of low level stuff.
In the current state, it runs a few simple demos and is not very impressive.
If you are looking for a more complete dos emulator, I suggest you check out [dosbox-x](https://github.com/joncampbell123/dosbox-x).

## Contributing

Any help and contributions are much welcome!


## Roadmap

- Finish 16-bit cpu core
- Get all the demos in https://github.com/martinlindhe/dos-software-decoding/tree/master/demo-256 running correctly
- Text mode debugger (separate binary)
- Implement 32-bit instructions
- Dynamic recompilation?
- Bundle as a .app on macOS (should fix 'main window opens in the background', as described in https://stackoverflow.com/a/44220855)


## Tests

The test framework requires https://github.com/martinlindhe/dos-software-decoding to be checked out in the parent directory.


## License

Under [MIT](LICENSE)
