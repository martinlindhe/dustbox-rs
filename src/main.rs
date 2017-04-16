#![feature(test)]

#![allow(dead_code)]

#[macro_use]
extern crate log;
extern crate colog;
// #[macro_use] extern crate difference;
extern crate time;
extern crate test;

extern crate image;
//extern crate vecmath;


#[macro_use]
extern crate conrod;

extern crate piston_window;

mod debugger;
mod tools;
mod cpu;
mod flags;
mod register;
mod instruction;
mod memory;
mod segment;
mod gpu;
mod renderer;
mod int10;
mod int16;
mod int21;

fn main() {
    colog::init();

    renderer::main();
}
