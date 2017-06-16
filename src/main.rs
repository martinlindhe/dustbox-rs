#![feature(test)]

#![allow(dead_code)]

#[macro_use]
extern crate log;
extern crate colog;
// #[macro_use] extern crate difference;
extern crate time;
extern crate test;
extern crate image;
extern crate gtk;
extern crate gdk;

mod debugger;
mod tools;
mod cpu;
mod flags;
mod register;
mod instruction;
mod memory;
mod segment;
mod gpu;
mod gui;
mod int10;
mod int16;
mod int21;

use std::sync::{Arc, Mutex};

fn main() {
    colog::init();

    let app = Arc::new(Mutex::new(debugger::Debugger::new()));

    let mut gui = gui::GUI::new(&app);

    gui.main(&app);
}
