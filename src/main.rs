#![feature(test)]

#![allow(dead_code)]
#![allow(unused_attributes)]
#![allow(unused_imports)]
#[macro_use]
#[macro_use(assert_diff)]

extern crate log;
extern crate colog;
extern crate regex;
extern crate difference;
extern crate time;
extern crate test;

mod debugger;
mod tools;

mod cpu;
mod flags;
mod register;
mod instruction;
mod segment;
mod gpu;

mod int10;
mod int16;
mod int21;

fn main() {
    colog::init();

    let mut debugger = debugger::new();
    debugger.start();
}
