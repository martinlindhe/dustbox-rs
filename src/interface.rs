#![allow(unused_imports)]

use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;

use gtk;
use gtk::prelude::*;
use gtk::{Button, Image, Label, Window, WindowType};
use gdk::enums::key;

use gdk::RGBA;
use gdk_pixbuf;

use memory::Memory;
use debugger;
use register;
use flags;
use cpu::CPU;
use register::{AX, BP, BX, CS, CX, DI, DS, DX, ES, FS, GS, SI, SP, SS};
use instruction::seg_offs_as_flat;

pub struct Interface {
    app: Rc<RefCell<debugger::Debugger>>,
    builder: Rc<RefCell<gtk::Builder>>,
}

impl Interface {
    // XXX rename to DebugWindow
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
            .get_object("button_run")
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
                app.cpu.gpu.draw_canvas(ctx, &app.cpu.memory.memory);
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
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            button_run.connect_clicked(move |_| {
                let mut app = app.borrow_mut();

                app.cpu.fatal_error = false;

                // run until bp is reached or 1M instructions was executed
                app.exec_command("step into 1_000_000");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                disasm_text
                    .get_buffer()
                    .map(|buffer| buffer.set_text(text.as_str()));

                update_registers(&mut app, &builder);
            });
        }

        {
            let app = Rc::clone(&self.app);
            button_dump_memory.connect_clicked(move |_| {
                let mut app = app.borrow_mut();
                app.exec_command("bindump 0x085F 0x0000 0xFFFF emu_mem.bin");
            });
        }

        {
            let app = Rc::clone(&self.app);
            let builder = Rc::clone(&self.builder);
            let disasm_text = disasm_text.clone();

            window.connect_key_press_event(move |_, key| {
                
                match key.get_keyval() as u32 {
                    key::Escape => gtk::main_quit(),
                    key::Return => {
                        let search_word = input_command.get_text().unwrap();
                        println!("> {}", search_word);
                        let mut app = app.borrow_mut();
                        app.exec_command(&search_word);
                        input_command.set_text("");

                        // update disasm
                        let text = app.disasm_n_instructions_to_text(20);
                        disasm_text
                            .get_buffer()
                            .map(|buffer| buffer.set_text(text.as_str()));

                        update_registers(&mut app, &builder);
                    },
                    _ => ()
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
        app.cpu.r16[AX].val,
        app.prev_regs.r16[AX].val,
    ));
    bx_value.set_markup(&u16_as_register_str(
        app.cpu.r16[BX].val,
        app.prev_regs.r16[BX].val,
    ));
    cx_value.set_markup(&u16_as_register_str(
        app.cpu.r16[CX].val,
        app.prev_regs.r16[CX].val,
    ));
    dx_value.set_markup(&u16_as_register_str(
        app.cpu.r16[DX].val,
        app.prev_regs.r16[DX].val,
    ));

    let si_value: gtk::Label = builder.get_object("si_value").unwrap();
    let di_value: gtk::Label = builder.get_object("di_value").unwrap();
    let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
    let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

    si_value.set_markup(&u16_as_register_str(
        app.cpu.r16[SI].val,
        app.prev_regs.r16[SI].val,
    ));
    di_value.set_markup(&u16_as_register_str(
        app.cpu.r16[DI].val,
        app.prev_regs.r16[DI].val,
    ));
    bp_value.set_markup(&u16_as_register_str(
        app.cpu.r16[BP].val,
        app.prev_regs.r16[BP].val,
    ));
    sp_value.set_markup(&u16_as_register_str(
        app.cpu.r16[SP].val,
        app.prev_regs.r16[SP].val,
    ));

    let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
    let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
    let es_value: gtk::Label = builder.get_object("es_value").unwrap();
    let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

    ds_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[DS],
        app.prev_regs.sreg16[DS],
    ));
    cs_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[CS],
        app.prev_regs.sreg16[CS],
    ));
    es_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[ES],
        app.prev_regs.sreg16[ES],
    ));
    fs_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[FS],
        app.prev_regs.sreg16[FS],
    ));

    let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
    let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
    let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

    gs_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[GS],
        app.prev_regs.sreg16[GS],
    ));
    ss_value.set_markup(&u16_as_register_str(
        app.cpu.sreg16[SS],
        app.prev_regs.sreg16[SS],
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
