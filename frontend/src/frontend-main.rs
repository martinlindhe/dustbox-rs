extern crate sdl2;

use sdl2::event::Event;
use sdl2::pixels;
use sdl2::pixels::PixelFormatEnum;

const SCREEN_WIDTH: u32 = 320;
const SCREEN_HEIGHT: u32 = 200;

extern crate dustbox;
use dustbox::machine::Machine;
use dustbox::tools;

extern crate clap;
use clap::{Arg, App};

use std::time::{Duration, SystemTime};
use std::thread::sleep;

fn main() {
    let matches = App::new("dustbox-frontend")
        .version("0.1")
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to use")
            .required(true)
            .index(1))
        .arg(Arg::with_name("deterministic")
            .help("Enables deterministic mode (debugging)")
            .short("d"))
        .get_matches();

    let filename = matches.value_of("INPUT").unwrap();

    let mut machine = if matches.is_present("deterministic") {
        Machine::deterministic()
    } else {
        Machine::default()
    };

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
        .allow_highdpi()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    //println!("Using SDL_Renderer \"{}\"", canvas.info().name);

    canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();

    let mut events = sdl_context.event_pump().unwrap();

    let app_start = SystemTime::now();
    let mut frame_event_sum = Duration::new(0, 0);
    let mut frame_exec_sum = Duration::new(0, 0);
    let mut frame_render_sum = Duration::new(0, 0);
    let mut frame_sleep_sum = Duration::new(0, 0);
    let mut last_video_mode = 0;

    let mut frame = 0;
    'main: loop {
        let event_start = SystemTime::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} => break 'main,

                Event::KeyDown {keycode: Some(keycode), keymod: modifier, ..} => {
                    /*
                    if keycode == Keycode::Escape {
                        break 'main
                    }
                    */

                    machine.keyboard_mut().unwrap().add_keypress(keycode, modifier);
                }

                _ => {}
            }
        }

        let event_time = event_start.elapsed().unwrap();
        frame_event_sum += event_time;

        let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, machine.gpu.mode.swidth, machine.gpu.mode.sheight).unwrap();
        let frame_start = SystemTime::now();


        let locked_fps = 30;

        {
            let window = canvas.window_mut();

            // resize window to current screen mode sizes
            if last_video_mode != machine.gpu.mode.mode {
                let resize_start = SystemTime::now();
                window.set_size(machine.gpu.mode.swidth, machine.gpu.mode.sheight).unwrap();
                let resize_time = resize_start.elapsed().unwrap();
                println!("Resized window for mode {:02x} in {:#?}", machine.gpu.mode.mode, resize_time);
                last_video_mode = machine.gpu.mode.mode;
            }

            // run some instructions and progress scanline until screen is drawn
            for _ in 0..machine.gpu.mode.swidth {
                // XXX calculate the number cycles to execute for (1/30th sec ) / scanlines
                // XXX measure by instruction cycles
                let num_instr = 300;
                machine.execute_instructions(num_instr);
                if machine.cpu.fatal_error {
                    println!("cpu fatal error occured. stopping execution");
                    break 'main;
                }
                machine.gpu.progress_scanline();
            }
            let exec_time = frame_start.elapsed().unwrap();

            frame += 1;
            frame_exec_sum += exec_time;

            let render_start = SystemTime::now();

            // render frame
            let data = machine.gpu.render_frame(&machine.mmu);
            let w = machine.gpu.mode.swidth as usize;

            let mut x: usize = 0;
            let mut y: usize = 0;

            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for pix in data {
                    if let dustbox::gpu::ColorSpace::RGB(r, g, b) = pix {
                        let offset = y*pitch + x*3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                        x += 1;
                        if x >= w {
                            x = 0;
                            y += 1;
                        }
                    }
                }
            }).unwrap();

            let render_time = render_start.elapsed().unwrap();
            frame_render_sum += render_time;

            // sleep for 1/30:th of a second, minus time it took to get here
            let mut sleep_time = Duration::new(0, 1_000_000_000 / 30);
            if sleep_time > exec_time {
                sleep_time -= exec_time;
            } else {
                println!("WARN: exec is slow {:#?}", exec_time);
                sleep_time = Duration::new(0, 0);
            }
            if sleep_time > render_time {
                sleep_time -= render_time;
            } else {
                println!("WARN: render is slow {:#?}", render_time);
                sleep_time = Duration::new(0, 0);
            }
            if sleep_time > event_time {
                sleep_time -= event_time;
            } else {
                println!("WARN: event handling is slow {:#?}", event_time);
                sleep_time = Duration::new(0, 0);
            }
            // println!("   sleep {:#?}, event {:#?}", sleep_time, event_time);

            sleep(sleep_time);
            frame_sleep_sum += sleep_time;

            if frame >= locked_fps {
                frame = 0;
                let frame_tot_sum = frame_event_sum + frame_exec_sum + frame_render_sum + frame_sleep_sum;
                println!("{} frames in {:#?} after {:#?}. event {:#?}, exec {:#?}, render {:#?}, sleep {:#?}", locked_fps, frame_tot_sum, app_start.elapsed().unwrap(), frame_event_sum, frame_exec_sum, frame_render_sum, frame_sleep_sum);
                frame_event_sum = Duration::new(0, 0);
                frame_exec_sum = Duration::new(0, 0);
                frame_render_sum = Duration::new(0, 0);
                frame_sleep_sum = Duration::new(0, 0);
            }
        }

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
