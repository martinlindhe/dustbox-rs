#![allow(unused_imports)]

use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;
use std::time::Duration;
use std::thread;

use gtk::prelude::*;
use gtk::{Button, Image, Label, Window, WindowType};
use gdk::RGBA;
use gdk::enums::key;
use gdk::prelude::*;

use dustbox::cpu::{CPU, R};
use dustbox::gpu::VideoModeBlock;
use dustbox::gpu::ColorSpace;
use dustbox::gpu::ColorSpace::RGB;

use dustbox::debug::Debugger;

pub struct Interface {
    app: Rc<RefCell<Debugger>>,
    builder: Rc<RefCell<gtk::Builder>>,
}

impl Interface {
    pub fn default(app: Rc<RefCell<Debugger>>) -> Self {
        gtk::init().unwrap_or_else(|_| panic!("Failed to initialize GTK."));

        Self {
            app,
            builder: Rc::new(RefCell::new(gtk::Builder::new_from_string(
                include_str!("interface.glade"),
            ))),
        }
    }

    /// start the gtk-rs main loop
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
        input_command.set_placeholder_text(Some("Enter command (or type help)"));

        let canvas: gtk::DrawingArea = self.builder
            .borrow()
            .get_object("canvas")
            .unwrap();
        {
            let app = Rc::clone(&self.app);
            canvas.connect_draw(move |_, ctx| {
                let app = app.borrow();
                let frame = app.machine.gpu().render_frame(&app.machine.mmu);
                draw_canvas(ctx, frame.data, &frame.mode);
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
                p.set_version(Some("0.1.0"));
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
            if let Some(buffer) = disasm_text.get_buffer() {
                buffer.set_text(text.as_str())
            }

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

                app.machine.cpu.fatal_error = false;
                app.exec_command("step into 1");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                if let Some(buffer) = disasm_text.get_buffer() {
                    buffer.set_text(text.as_str())
                }

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

                app.machine.cpu.fatal_error = false;
                app.exec_command("step over 1");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                if let Some(buffer) = disasm_text.get_buffer() {
                    buffer.set_text(text.as_str())
                }

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

                app.machine.cpu.fatal_error = false;

                // run until bp is reached or 1M instructions was executed
                app.exec_command("step into 6_000_000");

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                if let Some(buffer) = disasm_text.get_buffer() {
                    buffer.set_text(text.as_str())
                }

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
                app.machine.cpu.fatal_error = false;

                // runs & draws 1 frame
                app.machine.execute_frame();

                // update disasm
                let text = app.disasm_n_instructions_to_text(20);
                if let Some(buffer) = disasm_text.get_buffer() {
                    buffer.set_text(text.as_str())
                }

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

            window.connect_key_press_event(move |_, key| {
                if let key::Return = key.get_keyval() as u32 {
                    let search_word = input_command.get_text().unwrap();
                    let mut app = app.borrow_mut();
                    app.exec_command(&search_word);
                    input_command.set_text("");

                    // update disasm
                    let text = app.disasm_n_instructions_to_text(20);
                    if let Some(buffer) = disasm_text.get_buffer() {
                        buffer.set_text(text.as_str())
                    }

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

/// render video frame to canvas `c`
fn draw_canvas(c: &cairo::Context, buf: Vec<ColorSpace>, mode: &VideoModeBlock) {
    if buf.is_empty() {
        // println!("draw_canvas: no buffer to draw!");
        return;
    }

    let mut bytes_buf: Vec<u8> = Vec::new();

    for col in buf {
        if let RGB(r, g, b) = col {
            bytes_buf.push(r);
            bytes_buf.push(g);
            bytes_buf.push(b);
        }
    }

    let pixbuf = gdk_pixbuf::Pixbuf::new_from_mut_slice(
        bytes_buf,
        gdk_pixbuf::Colorspace::Rgb,
        false,
        8,
        mode.swidth as i32,
        mode.sheight as i32,
        mode.swidth as i32 * 3);
    c.set_source_pixbuf(&pixbuf, 0., 0.);
}

fn u16_as_register_str(app: &Debugger, r: R) -> String {
    let v = app.machine.cpu.get_r16(r);
    let prev = app.prev_regs.get_r16(r);
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
    app: &mut Debugger,
    builder: &Rc<RefCell<gtk::Builder>>,
) {
    let builder = builder.borrow();
    let ax_value: gtk::Label = builder.get_object("ax_value").unwrap();
    let bx_value: gtk::Label = builder.get_object("bx_value").unwrap();
    let cx_value: gtk::Label = builder.get_object("cx_value").unwrap();
    let dx_value: gtk::Label = builder.get_object("dx_value").unwrap();

    ax_value.set_markup(&u16_as_register_str(app, R::AX));
    bx_value.set_markup(&u16_as_register_str(app, R::BX));
    cx_value.set_markup(&u16_as_register_str(app, R::CX));
    dx_value.set_markup(&u16_as_register_str(app, R::DX));

    let si_value: gtk::Label = builder.get_object("si_value").unwrap();
    let di_value: gtk::Label = builder.get_object("di_value").unwrap();
    let bp_value: gtk::Label = builder.get_object("bp_value").unwrap();
    let sp_value: gtk::Label = builder.get_object("sp_value").unwrap();

    si_value.set_markup(&u16_as_register_str(app, R::SI));
    di_value.set_markup(&u16_as_register_str(app, R::DI));
    bp_value.set_markup(&u16_as_register_str(app, R::BP));
    sp_value.set_markup(&u16_as_register_str(app, R::SP));

    let ds_value: gtk::Label = builder.get_object("ds_value").unwrap();
    let cs_value: gtk::Label = builder.get_object("cs_value").unwrap();
    let es_value: gtk::Label = builder.get_object("es_value").unwrap();
    let fs_value: gtk::Label = builder.get_object("fs_value").unwrap();

    ds_value.set_markup(&u16_as_register_str(app, R::DS));
    cs_value.set_markup(&u16_as_register_str(app, R::CS));
    es_value.set_markup(&u16_as_register_str(app, R::ES));
    fs_value.set_markup(&u16_as_register_str(app, R::FS));

    let gs_value: gtk::Label = builder.get_object("gs_value").unwrap();
    let ss_value: gtk::Label = builder.get_object("ss_value").unwrap();
    let ip_value: gtk::Label = builder.get_object("ip_value").unwrap();

    gs_value.set_markup(&u16_as_register_str(app, R::GS));
    ss_value.set_markup(&u16_as_register_str(app, R::SS));
    ip_value.set_markup(&u16_as_register_str(app, R::IP));

    // XXX: color changes for flag changes too
    let c_flag: gtk::CheckButton = builder.get_object("c_flag").unwrap();
    let z_flag: gtk::CheckButton = builder.get_object("z_flag").unwrap();
    let s_flag: gtk::CheckButton = builder.get_object("s_flag").unwrap();
    let o_flag: gtk::CheckButton = builder.get_object("o_flag").unwrap();
    let a_flag: gtk::CheckButton = builder.get_object("a_flag").unwrap();
    let p_flag: gtk::CheckButton = builder.get_object("p_flag").unwrap();
    let d_flag: gtk::CheckButton = builder.get_object("d_flag").unwrap();
    let i_flag: gtk::CheckButton = builder.get_object("i_flag").unwrap();

    c_flag.set_active(app.machine.cpu.regs.flags.carry);
    z_flag.set_active(app.machine.cpu.regs.flags.zero);
    s_flag.set_active(app.machine.cpu.regs.flags.sign);
    o_flag.set_active(app.machine.cpu.regs.flags.overflow);
    a_flag.set_active(app.machine.cpu.regs.flags.adjust);
    p_flag.set_active(app.machine.cpu.regs.flags.parity);
    d_flag.set_active(app.machine.cpu.regs.flags.direction);
    i_flag.set_active(app.machine.cpu.regs.flags.interrupt);

    // save previous values for next update
    app.prev_regs = app.machine.cpu.regs.clone();
}
