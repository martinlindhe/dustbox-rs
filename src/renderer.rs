#![allow(unused_imports)]

use std;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;
use gtk::{Button, Label, Image, Window, WindowType};

use gdk::RGBA;

use memory::Memory;
use debugger;
use cpu::{CPU};
use register::{AX, BX, CX, DX, CS};

pub fn main() {

    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let builder = gtk::Builder::new_from_string(include_str!("interface.glade"));
    let window: gtk::Window = builder.get_object("main_window").unwrap();

    let button_step_into: gtk::Button = builder.get_object("button_step_into").unwrap();
    let button_step_over: gtk::Button = builder.get_object("button_step_over").unwrap();
    let button_run: gtk::Button = builder.get_object("button_run").unwrap();

    let disasm_text: gtk::TextView = builder.get_object("disasm_text").unwrap();

    // --- OLD:::

    const WIDTH: i32 = 800;
    const HEIGHT: i32 = 600;


    /*
    let window = Window::new(WindowType::Toplevel);
    window.set_title("x86emu");
    window.set_default_size(WIDTH, HEIGHT);
    */




    let app = Arc::new(Mutex::new(debugger::Debugger::new())); // XXX
    let x = 10;
    let y = 10;

/*
    let canvas = Arc::new(Mutex::new(Image::from_color(320, 200, Color::rgb(0, 0, 0)))); // XXX can the canvas live in the GPU struct?
    let canvas_copy = canvas.clone();
    canvas_copy.lock().unwrap().position(WIDTH as i32 - 340, 10);
    window.add(&canvas_copy.lock().unwrap());
*/

    //let disasm_text = app.lock().unwrap().disasm_n_instructions_to_text(20);
    let reg_text = app.lock().unwrap().cpu.print_registers();

    //let disasm_copy = disasm.clone();
    // XXX disasm_copy.lock().unwrap().position(x, y).size(450, 20 * 20);

    let regs = Label::new(reg_text.as_ref());
    let regs_copy = regs.clone();
    // XXX regs_copy.lock().unwrap().position(WIDTH as i32 - 300, 300)
        // XXX .size(290, 80)
        //.text(reg_text);
    window.add(&regs);

    let button_width = 90;
    let pad = 10;
    {
        let step_copy = app.clone();
        let disasm = disasm_text.clone();
        //let regs_step_copy = regs.clone();
        //let canvas_step_copy = canvas.clone();

        button_step_over.connect_clicked(move |_| {
            let mut shared = step_copy.lock().unwrap();

            shared.cpu.fatal_error = false;
            shared.step_over();

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            //disasm.set_label(text.as_ref());
            disasm.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            /*
            // update regs
            let reg_text = shared.cpu.print_registers();
            regs_step_copy.lock().unwrap().set_label(reg_text.as_ref());
            */

            //render_canvas(&canvas_step_copy.lock().unwrap(), &shared.cpu);
        });
    }

    {
        let step2_copy = app.clone();
        let disasm = disasm_text.clone();
        //let canvas_step2_copy = canvas.clone();

        button_step_into.connect_clicked(move |_| {
            // XXX .position(x + button_width as i32 + pad, HEIGHT as i32 - 50)
            // XXX .size(button_width, 30)

            let mut shared = step2_copy.lock().unwrap();

            shared.cpu.fatal_error = false;
            shared.step_into();

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            disasm.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            // update regs
            /*
            let reg_text = shared.cpu.print_registers();
            regs_step2_copy.lock().unwrap().set_label(reg_text.as_ref());
            */

            //render_canvas(&canvas_step2_copy.lock().unwrap(), &shared.cpu);
        });
    }

    {
        let step3_copy = app.clone();
        let disasm = disasm_text.clone();
        //let regs_step3_copy = regs.clone();
        //let canvas_step3_copy = canvas.clone();

        button_run.connect_clicked(move |_| {
            // XXX .position(x + (button_width * 2) as i32 + (pad * 2), HEIGHT as i32 - 50)
            // XXX .size(button_width, 30)
            let mut shared = step3_copy.lock().unwrap();

            shared.cpu.fatal_error = false;

            // run until bp is reached or 1M instructions was executed
            shared.step_into_n_instructions(1_000_000);

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            disasm.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            /*
            // update regs
            let reg_text = shared.cpu.print_registers();
            regs_step3_copy.lock().unwrap().set_label(reg_text.as_ref());
            */

            //render_canvas(&canvas_step3_copy.lock().unwrap(), &shared.cpu);
        });
    }

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}

fn render_canvas(canvas: &std::sync::Arc<gtk::Image>, cpu: &CPU) {
    /* XXX rewrite for rs-gtk

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
    */
}

