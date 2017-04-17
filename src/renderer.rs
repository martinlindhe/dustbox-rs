#![allow(unused_imports)]


use orbtk;
use orbtk::{Action, Button, Color, Grid, Image, Label, Menu, Point, Renderer, ProgressBar, Rect, Separator, TextBox, Window};
use orbtk::traits::{Border, Click, Enter, Place, Text};

use std;
use std::sync::{Arc, Mutex};

use memory::Memory;
use debugger;
use register::{AX, BX, CX, DX};

pub fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    let mut window = Window::new(Rect::new(0, 0, WIDTH, HEIGHT), "x86emu");


    let mut app = Arc::new(Mutex::new(debugger::Debugger::new()));


    // XXX for quick testing while building the ui
    app.lock().unwrap().load_binary("../dos-software-decoding/samples/bar/bar.com");


    let x = 10;
    let y = 10;

    let canvas = Image::from_color(320,200, Color::rgb(255, 255, 255));
    canvas.position(WIDTH as i32 - 340, 10);
    window.add(&canvas);

    
    let disasm_text = app.lock().unwrap().disasm_n_instructions_to_text(20);
    let reg_text = app.lock().unwrap().cpu.print_registers();

    let disasm = Label::new();
    disasm.position(x, y)
        .size(400, 20 * 20)
        .text(disasm_text);
    window.add(&disasm);

    let regs = Label::new();
    regs.position(WIDTH as i32 - 300, 300)
        .size(290, 80)
        .text(reg_text);
    window.add(&regs);

    let button = Button::new();
    button.position(x, HEIGHT as i32 - 50)
        .size(60, 30)
        .text("Step")
        .text_offset(6, 6)
        .on_click(move |_button: &Button, _point: Point| {
            println!("step clicked");
            app.lock().unwrap().step_into();
            // update disasm
            let disasm_text = app.lock().unwrap().disasm_n_instructions_to_text(20);
            disasm.text(disasm_text);

            // update regs
            let reg_text = app.lock().unwrap().cpu.print_registers();
            regs.text(reg_text);

            // XXX draw on img
            let mut image = canvas.image.borrow_mut();
            image.pixel(1,1,orbtk::Color::rgb(0, 0, 0) );
        });
    window.add(&button);

    window.exec();


/*

    println!("updated app.video_out");
    // XXX get ref to texture using app.video_out_id
    if let Some(img) = image_map.get_mut(app.video_out_id) {
        for y in 0..app.cpu.gpu.height {
            for x in 0..app.cpu.gpu.width {
                let offset = 0xA0000 + ((y * app.cpu.gpu.width) + x) as usize;
                let byte = app.cpu.memory.memory[offset];
                let ref pal = app.cpu.gpu.palette[byte as usize];
                img.put_pixel(x, y, Rgba([pal.r, pal.g, pal.b, 255]));
            }
        }
    }
    */


}
