#![allow(unused_imports)]

use std;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

use gtk;
use gtk::prelude::*;
use gtk::{Button, Label, Image, Window, WindowType};

use gdk::RGBA;
use gdk_pixbuf;

use memory::Memory;
use debugger;
use register;
use flags;
use cpu::{CPU};
use register::{AX, BX, CX, DX, SI, DI,BP, SP, DS, CS, ES, FS, GS, SS};

pub struct Interface {
    app: std::sync::Arc<std::sync::Mutex<debugger::Debugger>>,
    builder: std::sync::Arc<std::sync::Mutex<gtk::Builder>>,
    pixbuf: gdk_pixbuf::Pixbuf,
}

impl Interface {
    pub fn new(app: std::sync::Arc<std::sync::Mutex<debugger::Debugger>>) -> Self {

        gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

        let colorspace = 0; // XXX: gdk_pixbuf_sys::GDK_COLORSPACE_RGB = 0
        Self {
            app: app,
            builder: Arc::new(Mutex::new(gtk::Builder::new_from_string(include_str!("interface.glade")))),
            pixbuf: unsafe { gdk_pixbuf::Pixbuf::new(colorspace, false, 8, 320, 240).unwrap() },
        }
    }

    // start the gtk-rs main loop
    pub fn main(&mut self) {

        let window: gtk::Window = self.builder.lock().unwrap().get_object("main_window").unwrap();

        let button_step_into: gtk::Button = self.builder.lock().unwrap().get_object("button_step_into").unwrap();
        let button_step_over: gtk::Button = self.builder.lock().unwrap().get_object("button_step_over").unwrap();
        let button_run: gtk::Button = self.builder.lock().unwrap().get_object("button_run").unwrap();

        let disasm_text: gtk::TextView = self.builder.lock().unwrap().get_object("disasm_text").unwrap();
        // disasm_text.width = 400; // XXX set fixed width of disasm box, so it wont resize ...

        let image_video: gtk::Image = self.builder.lock().unwrap().get_object("image_video").unwrap();
        
        // XXX map the pixbuf into image_video
        // image_video = gtk::Image::new_from_pixbuf(&self.pixbuf);
        

        // menu items
        let file_quit: gtk::MenuItem = self.builder.lock().unwrap().get_object("file_quit").unwrap();
        let help_about: gtk::MenuItem = self.builder.lock().unwrap().get_object("help_about").unwrap();

        window.set_title("x86emu");

        file_quit.connect_activate(move |_| {
            gtk::main_quit();
        });

        {
            let window = window.clone();
            help_about.connect_activate(move |_| {
                let p = gtk::AboutDialog::new();
                p.set_program_name("x86emu");
                p.set_version("0.1.0");
                p.set_authors(&["Martin Lindhe"]);
                p.set_website_label(Some("My website"));
                p.set_website(Some("http://example.com"));
                p.set_comments(Some("A x86 debugger / emulator"));
                p.set_copyright(Some("Under MIT license"));
                p.set_transient_for(Some(&window));
                p.run();
                p.destroy();
            });
        }

    /*
        let canvas = Arc::new(Mutex::new(Image::from_color(320, 200, Color::rgb(0, 0, 0)))); // XXX can the canvas live in the GPU struct?
        let canvas_copy = canvas.clone();
        canvas_copy.lock().unwrap().position(WIDTH as i32 - 340, 10);
        window.add(&canvas_copy.lock().unwrap());
    */

        // update disasm
        let text = self.app.lock().unwrap().disasm_n_instructions_to_text(20);
        disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

        let app = self.app.clone();
        let builder = self.builder.clone();
        update_registers(app, builder);

        {
            let app = self.app.clone();
            let builder = self.builder.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_step_into.connect_clicked(move |_| {
                let app2 = app.clone();
                let builder = builder.clone();
                update_registers(app2, builder);

                let mut shared = app.lock().unwrap();

                shared.cpu.fatal_error = false;
                shared.step_into();

                // update disasm
                let text = shared.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));
            });
        }

        {
            let app = self.app.clone();
            let builder = self.builder.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_step_over.connect_clicked(move |_| {
                let app2 = app.clone();
                let builder = builder.clone();
                update_registers(app2, builder);

                let mut app = app.lock().unwrap();

                app.cpu.fatal_error = false;
                app.step_over();

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));
            });
        }

        {
            let app = self.app.clone();
            let builder = self.builder.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_run.connect_clicked(move |_| {
                let app2 = app.clone();
                let builder = builder.clone();
                update_registers(app2, builder);

                let mut shared = app.lock().unwrap();

                shared.cpu.fatal_error = false;

                // run until bp is reached or 1M instructions was executed
                shared.step_into_n_instructions(1_000_000);

                // update disasm
                let text = shared.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));
            });
        }

        window.show_all();

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        gtk::main();
    }
}

fn u16_as_register_str(v: u16, prev: u16) -> String {
    if v == prev {
        format!("<span font_desc=\"mono\">{:04X}</span>", v)
    } else {
        format!("<span color=\"#cf8c0b\" font_desc=\"mono\">{:04X}</span>", v)
    }
}

fn update_registers(app: std::sync::Arc<std::sync::Mutex<debugger::Debugger>>, builder: std::sync::Arc<std::sync::Mutex<gtk::Builder>>) {

    let mut app = app.lock().unwrap();
    let builder = builder.lock().unwrap();

    let ax_value: gtk::Label = builder.get_object("ax_value").unwrap();
    let bx_value: gtk::Label = builder.get_object("bx_value").unwrap();
    let cx_value: gtk::Label = builder.get_object("cx_value").unwrap();
    let dx_value: gtk::Label = builder.get_object("dx_value").unwrap();

    ax_value.set_markup(&u16_as_register_str(app.cpu.r16[AX].val, app.prev_regs.r16[AX].val));
    bx_value.set_markup(&u16_as_register_str(app.cpu.r16[BX].val, app.prev_regs.r16[BX].val));
    cx_value.set_markup(&u16_as_register_str(app.cpu.r16[CX].val, app.prev_regs.r16[CX].val));
    dx_value.set_markup(&u16_as_register_str(app.cpu.r16[DX].val, app.prev_regs.r16[DX].val));

    let si_value: gtk::Label = builder.get_object("si_value").unwrap();
    let di_value: gtk::Label = builder.get_object("di_value").unwrap();
    let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
    let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

    si_value.set_markup(&u16_as_register_str(app.cpu.r16[SI].val, app.prev_regs.r16[SI].val));
    di_value.set_markup(&u16_as_register_str(app.cpu.r16[DI].val, app.prev_regs.r16[DI].val));
    bp_value.set_markup(&u16_as_register_str(app.cpu.r16[BP].val, app.prev_regs.r16[BP].val));
    sp_value.set_markup(&u16_as_register_str(app.cpu.r16[SP].val, app.prev_regs.r16[SP].val));

    let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
    let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
    let es_value: gtk::Label = builder.get_object("es_value").unwrap();
    let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

    ds_value.set_markup(&u16_as_register_str(app.cpu.r16[DS].val, app.prev_regs.r16[DS].val));
    cs_value.set_markup(&u16_as_register_str(app.cpu.r16[CS].val, app.prev_regs.r16[CS].val));
    es_value.set_markup(&u16_as_register_str(app.cpu.r16[ES].val, app.prev_regs.r16[ES].val));
    fs_value.set_markup(&u16_as_register_str(app.cpu.r16[FS].val, app.prev_regs.r16[FS].val));

    let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
    let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
    let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

    gs_value.set_markup(&u16_as_register_str(app.cpu.r16[GS].val, app.prev_regs.r16[GS].val));
    ss_value.set_markup(&u16_as_register_str(app.cpu.r16[SS].val, app.prev_regs.r16[SS].val));
    ip_value.set_markup(&u16_as_register_str(app.cpu.ip, app.prev_regs.ip));

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

    // save previous regs for next update
    app.prev_regs.ip = app.cpu.ip;
    app.prev_regs.r16 = app.cpu.r16;
    app.prev_regs.sreg16 = app.cpu.sreg16;
    app.prev_regs.flags = app.cpu.flags;
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

