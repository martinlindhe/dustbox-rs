#![allow(dead_code)]

#[macro_use]
extern crate serde_derive;

#[cfg(test)]
extern crate pretty_assertions;

pub mod bios;
pub mod cmos;
pub mod codepage;
pub mod cpu;
pub mod debug;
pub mod format;
pub mod gpu;
pub mod hex;
pub mod keyboard;
pub mod machine;
pub mod memory;
pub mod mouse;
pub mod ndisasm;
pub mod pic;
pub mod pit;
pub mod storage;
pub mod string;
pub mod tools;

mod interrupt;
