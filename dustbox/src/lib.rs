#![allow(dead_code)]

#[macro_use]
extern crate serde_derive;

#[cfg(test)]
extern crate pretty_assertions;

pub mod machine;
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
pub mod debug;
pub mod ndisasm;
pub mod string;
pub mod keyboard;
pub mod mouse;
pub mod storage;

mod interrupt;
