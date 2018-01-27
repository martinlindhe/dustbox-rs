#![allow(unused_imports)]
#![allow(dead_code)]

extern crate image;
#[macro_use]
extern crate tera;
extern crate time;

pub mod tools;

pub mod cpu;
pub mod decoder;
pub mod instruction;
pub mod register;
pub mod flags;
pub mod mmu;
pub mod memory;
pub mod segment;
pub mod gpu;
pub mod pit;
mod palette;

mod int10;
mod int16;
mod int21;
mod int33;
