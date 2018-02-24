#![allow(unused_imports)]
#![allow(dead_code)]

extern crate time;
extern crate rand;
extern crate tempdir;
extern crate image;

#[macro_use]
extern crate tera;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate simple_error;

#[cfg(test)] #[macro_use]
extern crate pretty_assertions;

pub mod machine;
pub mod hardware;
pub mod cpu;
pub mod memory;
pub mod gpu;
pub mod pic;
pub mod pit;
pub mod cmos;
pub mod bios;
pub mod codepage;
pub mod tools;
pub mod hex;

mod interrupt;
