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
use cpu::{CPU};
use register::{AX, BX, CX, DX, CS};

pub fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let mut window = Window::new(Rect::new(0, 0, WIDTH, HEIGHT), "x86emu");


    let app = Arc::new(Mutex::new(debugger::Debugger::new())); // XXX
    let x = 10;
    let y = 10;

    let canvas = Arc::new(Mutex::new(Image::from_color(320, 200, Color::rgb(0, 0, 0)))); // XXX can the canvas live in the GPU struct?
    let canvas_copy = canvas.clone();
    canvas_copy.lock().unwrap().position(WIDTH as i32 - 340, 10);
    window.add(&canvas_copy.lock().unwrap());


    let disasm_text = app.lock().unwrap().disasm_n_instructions_to_text(20);
    let reg_text = app.lock().unwrap().cpu.print_registers();

    let disasm = Arc::new(Mutex::new(Label::new()));
    let disasm_copy = disasm.clone();
    disasm_copy.lock().unwrap().position(x, y).size(450, 20 * 20).text(disasm_text);
    window.add(&disasm_copy.lock().unwrap());

    let regs = Arc::new(Mutex::new(Label::new()));
    let regs_copy = regs.clone();
    regs_copy.lock().unwrap().position(WIDTH as i32 - 300, 300)
        .size(290, 80)
        .text(reg_text);
    window.add(&regs_copy.lock().unwrap());

    let button_width = 90;
    let pad = 10;


        let step_copy = app.clone();
        let disasm_step_copy = disasm.clone();
        let regs_step_copy = regs.clone();
        let canvas_step_copy = canvas.clone();

        let btn_step = Button::new();
        btn_step
            .position(x, HEIGHT as i32 - 50)
            .size(button_width, 30)
            .text("Step into")
            .text_offset(6, 6)
            .on_click(move |_button: &Button, _point: Point| {

                let mut shared = step_copy.lock().unwrap();

                shared.cpu.fatal_error = false;
                shared.step_into();

                // update disasm
                let disasm_text = shared.disasm_n_instructions_to_text(20);
                disasm_step_copy.lock().unwrap().text(disasm_text);

                // update regs
                let reg_text = shared.cpu.print_registers();
                regs_step_copy.lock().unwrap().text(reg_text);

                render_canvas(&canvas_step_copy.lock().unwrap(), &shared.cpu);
            });
        window.add(&btn_step);


        let step2_copy = app.clone();
        let disasm_step2_copy = disasm.clone();
        let regs_step2_copy = regs.clone();
        let canvas_step2_copy = canvas.clone();

        let btn_step2 = Button::new();
        btn_step2
            .position(x + button_width as i32 + pad, HEIGHT as i32 - 50)
            .size(button_width, 30)
            .text("Step over")
            .text_offset(6, 6)
            .on_click(move |_button: &Button, _point: Point| {

                let mut shared = step2_copy.lock().unwrap();

                shared.cpu.fatal_error = false;
                shared.step_over();

                // update disasm
                let disasm_text = shared.disasm_n_instructions_to_text(20);
                disasm_step2_copy.lock().unwrap().text(disasm_text);

                // update regs
                let reg_text = shared.cpu.print_registers();
                regs_step2_copy.lock().unwrap().text(reg_text);

                render_canvas(&canvas_step2_copy.lock().unwrap(), &shared.cpu);
            });
        window.add(&btn_step2);


    window.exec();
}

fn render_canvas(canvas: &std::sync::Arc<orbtk::Image>, cpu: &CPU) {
    let mut image = canvas.image.borrow_mut();

    // XXX rather replace image pixels
    // image = dbg.cpu.gpu.render_frame();
    // image.from_data(frame.into_data());

    // VGA, mode 13h:
    let height = 320; // dbg.cpu.gpu.height;
    let width = 240; // dbg.cpu.gpu.width;

    for y in 0..height {
        for x in 0..width {
            let offset = 0xA0000 + ((y * width) + x) as usize;
            let byte = cpu.memory.memory[offset];
            let pal = &cpu.gpu.palette[byte as usize];
            image.pixel(x as i32, y as i32, Color::rgb(pal.r, pal.g, pal.b));
        }
    }
}

