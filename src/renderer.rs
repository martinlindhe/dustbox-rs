#![allow(unused_imports)]


use orbtk;
use orbtk::{Action, Button, Color, Grid, Image, Label, Menu, Point, Renderer, ProgressBar, Rect,
            Separator, TextBox, Window};
use orbtk::traits::{Border, Click, Enter, Place, Text};

use std;
use std::sync::{Arc, Mutex};

use std::rc::Rc;
use std::cell::RefCell;

use memory::Memory;
use debugger;
use cpu;
use register::{AX, BX, CX, DX};

pub fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let mut window = Window::new(Rect::new(0, 0, WIDTH, HEIGHT), "x86emu");


    let app = Arc::new(Mutex::new(debugger::Debugger::new()));

    let x = 10;
    let y = 10;

    let canvas = Image::from_color(320, 200, Color::rgb(0, 0, 0));
    canvas.position(WIDTH as i32 - 340, 10);
    window.add(&canvas);


    let disasm_text = app.lock().unwrap().disasm_n_instructions_to_text(20);
    let reg_text = app.lock().unwrap().cpu.print_registers();

    let disasm = Label::new();
    disasm.position(x, y).size(450, 20 * 20).text(disasm_text);
    window.add(&disasm);

    let regs = Label::new();
    regs.position(WIDTH as i32 - 300, 300)
        .size(290, 80)
        .text(reg_text);
    window.add(&regs);

    let btn_step = Button::new();
    btn_step
        .position(x, HEIGHT as i32 - 50)
        .size(60, 30)
        .text("Step")
        .text_offset(6, 6)
        .on_click(move |_button: &Button, _point: Point| {

            let mut dbg = app.lock().unwrap();
            // XXX have separate "step into" & "step over" buttons
            //for _ in 0..500000 {
            //   dbg.step_into();
            //}
            dbg.step_over();
            println!("Executed {} instructions", dbg.cpu.instruction_count);

            // update disasm
            let disasm_text = dbg.disasm_n_instructions_to_text(20);
            disasm.text(disasm_text);

            // update regs
            let reg_text = dbg.cpu.print_registers();
            regs.text(reg_text);

            // draw on img
            let mut image = canvas.image.borrow_mut();

            // XXX rather replace image pixels
            // image = dbg.cpu.gpu.render_frame();
            // image.from_data(frame.into_data());

            let height = dbg.cpu.gpu.height;
            let width = dbg.cpu.gpu.width;

            for y in 0..height {
                for x in 0..width {
                    let offset = 0xA0000 + ((y * width) + x) as usize;
                    let byte = dbg.cpu.memory.memory[offset];
                    let pal = &dbg.cpu.gpu.palette[byte as usize];
                    image.pixel(x as i32, y as i32, Color::rgb(pal.r, pal.g, pal.b));
                }
            }

        });
    window.add(&btn_step);

    window.exec();

}

