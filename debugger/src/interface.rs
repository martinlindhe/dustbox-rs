#![allow(unused_imports)]

use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;
use std::time::Duration;
use std::thread;

use gtk;
use gtk::prelude::*;
use gtk::{Button, Image, Label, Window, WindowType};
use gdk::RGBA;
use gdk::enums::key;
use gdk::prelude::*;
use gdk_pixbuf;
use cairo;

use dustbox::memory::Memory;
use dustbox::cpu::CPU;
use dustbox::cpu;
use dustbox::cpu::register::{R8, R16, SR};
use dustbox::gpu::palette::DACPalette;

use debugger;

pub struct Interface {
    app: Rc<RefCell<debugger::Debugger>>,
    builder: Rc<RefCell<gtk::Builder>>,
}

impl Interface {
    pub fn new(app: Rc<RefCell<debugger::Debugger>>) -> Self {
        gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

        Self {
            app: app,
            builder: Rc::new(RefCell::new(gtk::Builder::new_from_string(
                include_str!("interface.glade"),
            ))),
        }
    }

    // start the gtk-rs main loop
    pub fn main(&mut self) {
        let window: gtk::Window = self.builder
            .borrow()
            .get_object("main_window")
            .unwrap();
        let button_step_into: gtk::Button = self.builder
            .borrow()
            .get_object("button_step_into")
            .unwrap();
        let button_step_over: gtk::Button = self.builder
            .borrow()
            .get_object("button_step_over")
            .unwrap();
        let button_run: gtk::Button = self.builder
            .borrow()
            .get_object("button_run") // XXX currently runs 1 frame
            .unwrap();
        let button_run_to_breakpoint: gtk::Button = self.builder
            .borrow()
            .get_object("button_run_to_breakpoint")
            .unwrap();
        let button_list_breakpoints: gtk::Button = self.builder
            .borrow()
            .get_object("button_list_breakpoints")
            .unwrap();
        let button_dump_memory: gtk::Button = self.builder
            .borrow()
            .get_object("button_dump_memory")
            .unwrap();
        let disasm_text: gtk::TextView = self.builder
            .borrow()
            .get_object("disasm_text")
            .unwrap();
        let input_command: gtk::Entry = self.builder
            .borrow()
            .get_object("input_command")
            .unwrap();
        input_command.set_placeholder_text("Enter command (or type help)");

        let canvas: gtk::DrawingArea = self.builder
            .borrow()
            .get_object("canvas")
            .unwrap();
        {
            let app = Rc::clone(&self.app);
            canvas.connect_draw(move |_, ctx| {
                let app = app.borrow();
                //This makes a copy for every draw, maybe not a problem
                //but it's stupid, and we shouldn't do this
                let mem = app.cpu.mmu.dump_mem();
                draw_canvas(ctx, &mem, app.cpu.gpu.width, app.cpu.gpu.height, &app.cpu.gpu.pal);
                ctx.paint();
                Inhibit(false)
            });
        }

        // menu items
        let file_quit: gtk::MenuItem = self.builder
            .borrow()
            .get_object("file_quit")
            .unwrap();
        let help_about: gtk::MenuItem = self.builder
            .borrow()
            .get_object("help_about")
            .unwrap();

        window.set_title("dustbox");

        file_quit.connect_activate(move |_| {
            gtk::main_quit();
        });

        {
            let window = window.clone();
            help_about.connect_activate(move |_| {
                let p = gtk::AboutDialog::new();
                p.set_program_name("dustbox");
                p.set_version("0.1.0");
                p.set_authors(&["Martin Lindhe"]);
                p.set_website(Some("https://martinlindhe.github.io/dustbox-rs"));
                p.set_comments(Some("A MS-DOS debugger / emulator"));
                p.set_copyright(Some("MIT license"));
                p.set_transient_for(Some(&window));
                p.run();
                p.destroy();
            });
        }

        {
            // update disasm
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let text = app.borrow_mut().disasm_n_instructions_to_text(20);
            disasm_text
                .get_buffer()
                .map(|buffer| buffer.set_text(text.as_str()));

            {
                let mut app = app.borrow_mut();
                update_registers(&mut app, &builder);
                update_canvas(&builder);
            }
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            button_step_into.connect_clicked(move |_| {
                let mut app = app.borrow_mut();

                app.cpu.fatal_error = false;
                app.exec_command("step into 1");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text
                    .get_buffer()
                    .map(|buffer| buffer.set_text(text.as_str()));

                update_registers(&mut app, &builder);
                canvas.queue_draw();
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            button_step_over.connect_clicked(move |_| {
                let mut app = app.borrow_mut();

                app.cpu.fatal_error = false;
                app.exec_command("step over 1");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text
                    .get_buffer()
                    .map(|buffer| buffer.set_text(text.as_str()));

                update_registers(&mut app, &builder);
                update_canvas(&builder);
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            button_run_to_breakpoint.connect_clicked(move |_| {
                let mut app = app.borrow_mut();

                app.cpu.fatal_error = false;

                // run until bp is reached or 1M instructions was executed
                app.exec_command("step into 6_000_000");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text
                    .get_buffer()
                    .map(|buffer| buffer.set_text(text.as_str()));

                update_registers(&mut app, &builder);
                update_canvas(&builder);
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            button_run.connect_clicked(move |_| {
                let mut app = app.borrow_mut();
                app.cpu.fatal_error = false;

                // runs & draws 1 frame
                app.cpu.execute_frame();

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text
                    .get_buffer()
                    .map(|buffer| buffer.set_text(text.as_str()));

                update_registers(&mut app, &builder);
                update_canvas(&builder);
            });
        }

        {
            let app = Rc::clone(&self.app);
            button_list_breakpoints.connect_clicked(move |_| {
                let mut app = app.borrow_mut();
                app.exec_command("bp list");
            });
        }

        {
            let app = Rc::clone(&self.app);
            button_dump_memory.connect_clicked(move |_| {
                let mut app = app.borrow_mut();
                app.exec_command("bindump cs:0x0000 0xFFFF emu_mem.bin");
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            window.connect_key_press_event(move |_, key| {
                if let key::Return = key.get_keyval() as u32 {
                    let search_word = input_command.get_text().unwrap();
                    let mut app = app.borrow_mut();
                    app.exec_command(&search_word);
                    input_command.set_text("");

                    // update disasm
                    let text = app.disasm_n_instructions_to_text(20);
                    disasm_text
                        .get_buffer()
                        .map(|buffer| buffer.set_text(text.as_str()));

                    update_registers(&mut app, &builder);
                    update_canvas(&builder);
                }
                Inhibit(false)
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

// render video frame to canvas `c`
fn draw_canvas(c: &cairo::Context, memory: &[u8], width: u32, height: u32, pal: &[DACPalette]) {
    let mut buf = vec![0u8; (width * height * 3) as usize];
    for y in 0..height {
        for x in 0..width {
            let offset = 0xA_0000 + ((y * width) + x) as usize;
            let byte = memory[offset];
            let pal = &pal[byte as usize];
            let i = ((y * width + x) * 3) as usize;
            buf[i] = pal.r;
            buf[i+1] = pal.g;
            buf[i+2] = pal.b;
        }
    }

    let pixbuf = gdk_pixbuf::Pixbuf::new_from_vec(
        buf,
        0,
        false,
        8,
        width as i32,
        height as i32,
        width as i32 * 3,
    );
    c.set_source_pixbuf(&pixbuf, 0., 0.);
}

fn u16_as_register_str(v: u16, prev: u16) -> String {
    if v == prev {
        format!("<span font_desc=\"mono\">{:04X}</span>", v)
    } else {
        format!(
            "<span color=\"#cf8c0b\" font_desc=\"mono\">{:04X}</span>",
            v
        )
    }
}

fn update_canvas(builder: &Rc<RefCell<gtk::Builder>>) {
    let canvas: gtk::DrawingArea = builder
            .borrow()
            .get_object("canvas")
            .unwrap();
    canvas.queue_draw();
}

fn update_registers(
    app: &mut debugger::Debugger,
    builder: &Rc<RefCell<gtk::Builder>>,
) {
    let builder = builder.borrow();
    let ax_value: gtk::Label = builder.get_object("ax_value").unwrap();
    let bx_value: gtk::Label = builder.get_object("bx_value").unwrap();
    let cx_value: gtk::Label = builder.get_object("cx_value").unwrap();
    let dx_value: gtk::Label = builder.get_object("dx_value").unwrap();

    ax_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::AX),
        app.prev_regs.r16[R16::AX.index()].val,
    ));
    bx_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::BX),
        app.prev_regs.r16[R16::BX.index()].val,
    ));
    cx_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::CX),
        app.prev_regs.r16[R16::CX.index()].val,
    ));
    dx_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::DX),
        app.prev_regs.r16[R16::DX.index()].val,
    ));

    let si_value: gtk::Label = builder.get_object("si_value").unwrap();
    let di_value: gtk::Label = builder.get_object("di_value").unwrap();
    let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
    let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

    si_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::SI),
        app.prev_regs.r16[R16::SI.index()].val,
    ));
    di_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::DI),
        app.prev_regs.r16[R16::DI.index()].val,
    ));
    bp_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::BP),
        app.prev_regs.r16[R16::BP.index()].val,
    ));
    sp_value.set_markup(&u16_as_register_str(
        app.cpu.get_r16(&R16::SP),
        app.prev_regs.r16[R16::SP.index()].val,
    ));

    let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
    let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
    let es_value: gtk::Label = builder.get_object("es_value").unwrap();
    let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

    ds_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::DS),
        app.prev_regs.sreg16[SR::DS.index()],
    ));
    cs_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::CS),
        app.prev_regs.sreg16[SR::CS.index()],
    ));
    es_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::ES),
        app.prev_regs.sreg16[SR::ES.index()],
    ));
    fs_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::FS),
        app.prev_regs.sreg16[SR::FS.index()],
    ));

    let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
    let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
    let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

    gs_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::GS),
        app.prev_regs.sreg16[SR::GS.index()],
    ));
    ss_value.set_markup(&u16_as_register_str(
        app.cpu.get_sr(&SR::SS),
        app.prev_regs.sreg16[SR::SS.index()],
    ));
    ip_value.set_markup(&u16_as_register_str(app.cpu.ip, app.prev_regs.ip));

    // XXX: color changes for flag changes too
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

    // save previous values for next update
    app.prev_regs.ip = app.cpu.ip;
    app.prev_regs.r16 = app.cpu.r16;
    app.prev_regs.sreg16 = app.cpu.sreg16;
    app.prev_regs.flags = app.cpu.flags;
}
