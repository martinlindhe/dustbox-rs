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
use register::{AX, BX, CX, DX, SI, DI,BP, SP, DS, CS, ES, FS, GS, SS};

fn update_registers(app: &debugger::Debugger, builder: &gtk::Builder) {

    let ax_value: gtk::Label = builder.get_object("ax_value").unwrap();
    let bx_value: gtk::Label = builder.get_object("bx_value").unwrap();
    let cx_value: gtk::Label = builder.get_object("cx_value").unwrap();
    let dx_value: gtk::Label = builder.get_object("dx_value").unwrap();

    ax_value.set_text(&app.cpu.r16[AX].as_hex_string());
    bx_value.set_text(&app.cpu.r16[BX].as_hex_string());
    cx_value.set_text(&app.cpu.r16[CX].as_hex_string());
    dx_value.set_text(&app.cpu.r16[DX].as_hex_string());

    let si_value: gtk::Label = builder.get_object("si_value").unwrap();
    let di_value: gtk::Label = builder.get_object("di_value").unwrap();
    let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
    let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

    si_value.set_text(&app.cpu.r16[SI].as_hex_string());
    di_value.set_text(&app.cpu.r16[DI].as_hex_string());
    bp_value.set_text(&app.cpu.r16[BP].as_hex_string());
    sp_value.set_text(&app.cpu.r16[SP].as_hex_string());

    let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
    let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
    let es_value: gtk::Label = builder.get_object("es_value").unwrap();
    let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

    ds_value.set_text(&app.cpu.r16[DS].as_hex_string());
    cs_value.set_text(&app.cpu.r16[CS].as_hex_string());
    es_value.set_text(&app.cpu.r16[ES].as_hex_string());
    fs_value.set_text(&app.cpu.r16[FS].as_hex_string());

    let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
    let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
    let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

    gs_value.set_text(&app.cpu.r16[GS].as_hex_string());
    ss_value.set_text(&app.cpu.r16[SS].as_hex_string());
    let ip = format!("{:04X}", &app.cpu.ip);
    ip_value.set_text(&ip);

    // flags
    let c_flag: gtk::CheckButton = builder.get_object("c_flag").unwrap();
    let z_flag: gtk::CheckButton = builder.get_object("z_flag").unwrap();
    let s_flag: gtk::CheckButton = builder.get_object("s_flag").unwrap();
    let o_flag: gtk::CheckButton = builder.get_object("o_flag").unwrap();
    let a_flag: gtk::CheckButton = builder.get_object("a_flag").unwrap();
    let p_flag: gtk::CheckButton = builder.get_object("p_flag").unwrap();
    let d_flag: gtk::CheckButton = builder.get_object("d_flag").unwrap();
    let i_flag: gtk::CheckButton = builder.get_object("i_flag").unwrap();

    c_flag.set_active(app.cpu.flags.carry);
    z_flag.set_active(app.cpu.flags.zero);
    s_flag.set_active(app.cpu.flags.sign);
    o_flag.set_active(app.cpu.flags.overflow);
    a_flag.set_active(app.cpu.flags.auxiliary_carry);
    p_flag.set_active(app.cpu.flags.parity);
    d_flag.set_active(app.cpu.flags.direction);
    i_flag.set_active(app.cpu.flags.interrupt);
}

pub fn main() {

    gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

    let builder = gtk::Builder::new_from_string(include_str!("interface.glade"));
    let window: gtk::Window = builder.get_object("main_window").unwrap();

    let button_step_into: gtk::Button = builder.get_object("button_step_into").unwrap();
    let button_step_over: gtk::Button = builder.get_object("button_step_over").unwrap();
    let button_run: gtk::Button = builder.get_object("button_run").unwrap();

    let disasm_text: gtk::TextView = builder.get_object("disasm_text").unwrap();

    window.set_title("x86emu");


    let app = Arc::new(Mutex::new(debugger::Debugger::new()));

/*
    let canvas = Arc::new(Mutex::new(Image::from_color(320, 200, Color::rgb(0, 0, 0)))); // XXX can the canvas live in the GPU struct?
    let canvas_copy = canvas.clone();
    canvas_copy.lock().unwrap().position(WIDTH as i32 - 340, 10);
    window.add(&canvas_copy.lock().unwrap());
*/

    // update disasm
    let text = app.lock().unwrap().disasm_n_instructions_to_text(20);
    disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

    // update regs
    update_registers(&app.lock().unwrap(), &builder);

    //let reg_text = app.lock().unwrap().cpu.print_registers();
    //let regs = Label::new(reg_text.as_ref());
    //let regs_copy = regs.clone();
    // XXX regs_copy.lock().unwrap().position(WIDTH as i32 - 300, 300).text(reg_text);

    {
        let app = app.clone();
        let builder = builder.clone();
        let disasm_text = disasm_text.clone();
        //let regs_step_copy = regs.clone();
        //let canvas_step_copy = canvas.clone();

        button_step_over.connect_clicked(move |_| {
            let mut shared = app.lock().unwrap();

            shared.cpu.fatal_error = false;
            shared.step_over();

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            // update regs
            update_registers(&shared, &builder);
        });
    }

    {
        let app = app.clone();
        let builder = builder.clone();
        let disasm_text = disasm_text.clone();
        //let canvas_step2_copy = canvas.clone();

        button_step_into.connect_clicked(move |_| {
            let mut shared = app.lock().unwrap();

            shared.cpu.fatal_error = false;
            shared.step_into();

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            // update regs
            update_registers(&shared, &builder);
        });
    }

    {
        let app = app.clone();
        let builder = builder.clone();
        let disasm_text = disasm_text.clone();
        //let regs_step3_copy = regs.clone();
        //let canvas_step3_copy = canvas.clone();

        button_run.connect_clicked(move |_| {
            let mut shared = app.lock().unwrap();

            shared.cpu.fatal_error = false;

            // run until bp is reached or 1M instructions was executed
            shared.step_into_n_instructions(1_000_000);

            // update disasm
            let text = shared.disasm_n_instructions_to_text(20);
            disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

            // update regs
            update_registers(&shared, &builder);
        });
    }

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}

/*
fn render_canvas(canvas: &std::sync::Arc<gtk::Image>, cpu: &CPU) {
    XXX rewrite for rs-gtk

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
*/

