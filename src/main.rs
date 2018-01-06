#![feature(test)]

#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use] extern crate log;

extern crate colog;
extern crate time;
extern crate test;
extern crate gtk;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate cairo;
extern crate raster;
#[macro_use] extern crate tera;

mod debugger;
mod tools;
mod cpu;
mod flags;
mod register;
mod instruction;
mod memory;
mod segment;
mod gpu;
mod interface;
mod int10;
mod int16;
mod int21;
mod int33;

use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    colog::init();

    let app = Rc::new(RefCell::new(debugger::Debugger::new()));

    let mut gui = interface::Interface::new(app);
    gui.main();
}

