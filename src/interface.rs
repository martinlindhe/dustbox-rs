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

struct PrevRegs {
    pub ip: u16,
    pub r16: [register::Register16; 8], // general purpose registers
    pub sreg16: [u16; 6], // segment registers
    pub flags: flags::Flags,
}

pub struct Interface {
    app: std::sync::Arc<std::sync::Mutex<debugger::Debugger>>,
    builder: std::sync::Arc<std::sync::Mutex<gtk::Builder>>,
    prev_regs: PrevRegs,
    pixbuf: gdk_pixbuf::Pixbuf,
}

impl Interface {
    pub fn new(app: std::sync::Arc<std::sync::Mutex<debugger::Debugger>>) -> Self {

        gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

        let ip = app.lock().unwrap().cpu.ip;
        let r16 = app.lock().unwrap().cpu.r16;
        let sreg16 = app.lock().unwrap().cpu.sreg16;
        let flags = app.lock().unwrap().cpu.flags;

        let colorspace = 0; // XXX: gdk_pixbuf_sys::GDK_COLORSPACE_RGB = 0
        Self {
            app: app,
            builder: Arc::new(Mutex::new(gtk::Builder::new_from_string(include_str!("interface.glade")))),
            prev_regs: PrevRegs{
                ip: ip,
                r16: r16,
                sreg16: sreg16,
                flags: flags,
            },
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

        self.update_registers();

        {
            let app = self.app.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_step_into.connect_clicked(move |_| {
                let mut shared = app.lock().unwrap();

                shared.cpu.fatal_error = false;
                shared.step_into();

                // update disasm
                let text = shared.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

                //self.update_registers(); // XXX some lifetime error.-.. ?!?!!
            });
        }

        {
            let app = self.app.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_step_over.connect_clicked(move |_| {
                let mut app = app.lock().unwrap();

                app.cpu.fatal_error = false;
                app.step_over();

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

                //self.update_registers();
            });
        }

        {
            let app = self.app.clone();
            let disasm_text = disasm_text.clone();
            //let canvas = canvas.clone();

            button_run.connect_clicked(move |_| {
                let mut shared = app.lock().unwrap();

                shared.cpu.fatal_error = false;

                // run until bp is reached or 1M instructions was executed
                shared.step_into_n_instructions(1_000_000);

                // update disasm
                let text = shared.disasm_n_instructions_to_text(20);
                disasm_text.get_buffer().map(|buffer| buffer.set_text(text.as_str()));

                // update regs
                //self.update_registers(&builder);
            });
        }

        window.show_all();

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        gtk::main();
    }

    fn update_registers(&mut self) {

        let app = self.app.lock().unwrap();
        let builder = self.builder.lock().unwrap();

        let ax_value: gtk::Label = builder.get_object("ax_value").unwrap();
        let bx_value: gtk::Label = builder.get_object("bx_value").unwrap();
        let cx_value: gtk::Label = builder.get_object("cx_value").unwrap();
        let dx_value: gtk::Label = builder.get_object("dx_value").unwrap();

        ax_value.set_markup(&app.cpu.r16[AX].as_hex_string());
        bx_value.set_markup(&app.cpu.r16[BX].as_hex_string());
        cx_value.set_markup(&app.cpu.r16[CX].as_hex_string());
        dx_value.set_markup(&app.cpu.r16[DX].as_hex_string());

        let si_value: gtk::Label = builder.get_object("si_value").unwrap();
        let di_value: gtk::Label = builder.get_object("di_value").unwrap();
        let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
        let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

        si_value.set_markup(&app.cpu.r16[SI].as_hex_string());
        di_value.set_markup(&app.cpu.r16[DI].as_hex_string());
        bp_value.set_markup(&app.cpu.r16[BP].as_hex_string());
        sp_value.set_markup(&app.cpu.r16[SP].as_hex_string());

        let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
        let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
        let es_value: gtk::Label = builder.get_object("es_value").unwrap();
        let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

        ds_value.set_markup(&app.cpu.r16[DS].as_hex_string());
        cs_value.set_markup(&app.cpu.r16[CS].as_hex_string());
        es_value.set_markup(&app.cpu.r16[ES].as_hex_string());
        fs_value.set_markup(&app.cpu.r16[FS].as_hex_string());

        let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
        let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
        let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

        gs_value.set_markup(&app.cpu.r16[GS].as_hex_string());
        ss_value.set_markup(&app.cpu.r16[SS].as_hex_string());
        // XXX change color for changed values
        let ip = format!("<span color=\"#cf8c0b\" font_desc=\"mono\">{:04X}</span>", &app.cpu.ip);
        ip_value.set_markup(&ip);

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

