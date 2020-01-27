#![allow(dead_code)]
#![allow(clippy::single_match)]
#![allow(clippy::verbose_bit_mask)]
#![allow(clippy::cognitive_complexity)]

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
pub mod dos;
pub mod storage;
pub mod string;
pub mod tools;
