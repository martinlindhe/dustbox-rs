extern crate sdl2;

use sdl2::event::Event;
use sdl2::pixels;
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;

const SCREEN_WIDTH: u32 = 320;
const SCREEN_HEIGHT: u32 = 200;

extern crate dustbox;
use dustbox::machine::Machine;
use dustbox::tools;

extern crate clap;
use clap::{Arg, App};

fn main() {
    let matches = App::new("dustbox-frontend")
        .version("0.1")
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to use")
            .required(true)
            .index(1))
        .get_matches();

    let filename = matches.value_of("INPUT").unwrap();

    let mut machine = Machine::default();

    match tools::read_binary(filename) {
        Ok(data) => {
            machine.load_executable(&data);
        }
        Err(what) => panic!("error {}", what),
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys.window(&format!("dustbox {}", filename), SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    println!("Using SDL_Renderer \"{}\"", canvas.info().name);

    canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut events = sdl_context.event_pump().unwrap();

    let mut frame = 0;
    'main: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} => break 'main,

                Event::KeyDown {keycode: Some(keycode), ..} => {
                    if keycode == Keycode::Escape {
                        break 'main
                    }
                }

                _ => {}
            }
        }

        // run a some instructions
        machine.execute_instructions(4000); // XXX execute N instrs, as needsed for X mhz

        frame += 1;
        if frame > 60 {
            // XXX how many frames per second?

            // render frame
            let data = machine.hw.gpu.render_frame(&machine.hw.mmu);
            let w = machine.hw.gpu.mode.swidth;

            let mut x: i16 = 0;
            let mut y: i16 = 0;
            for pix in data {
                if let dustbox::gpu::ColorSpace::RGB(r, g, b) = pix {
                    canvas.pixel(x, y, pixels::Color::RGB(r, g, b)).unwrap();
                    x += 1;
                    if x as u32 >= w {
                        x = 0;
                        y += 1;
                    }
                }
            }
            canvas.present();
            frame = 0;
        }
    }
}
