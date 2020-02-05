use std::time::{Duration, SystemTime};
use std::thread::sleep;

use sdl2::event::Event;
use sdl2::pixels;
use sdl2::pixels::PixelFormatEnum;

#[macro_use]
extern crate clap;
use clap::{Arg, App};

use dustbox::machine::Machine;
use dustbox::mouse::MouseButton;

const DEBUG_PERFORMANCE: bool = true;

fn main() {
    let matches = App::new("dustbox-frontend")
        .version("0.1")
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to use")
            .required(true)
            .index(1))
        .arg(Arg::with_name("SCALE")
            .help("Scale the window resolution")
            .takes_value(true)
            .long("scale"))
        .arg(Arg::with_name("NOSQUARE")
            .help("Don't make pixels square by stretching (default)")
            .long("no-square"))
        .arg(Arg::with_name("DETERMINISTIC")
            .help("Enables deterministic mode (debugging)")
            .long("deterministic"))
        .arg(Arg::with_name("SEGMENT")
            .help("Segment to load program into")
            .takes_value(true)
            .long("segment"))
        .arg(Arg::with_name("TRACEFILE")
            .help("Output a instruction trace similar to dosbox LOGS (debugging)")
            .takes_value(true)
            .long("trace"))
        .arg(Arg::with_name("TRACECOUNT")
            .help("Limits the trace to a number of instructions (debugging)")
            .takes_value(true)
            .long("tracecount"))
        .get_matches();

    let filename = matches.value_of("INPUT").unwrap();

    let mut machine = if matches.is_present("DETERMINISTIC") {
        Machine::deterministic()
    } else {
        Machine::default()
    };

    if matches.is_present("TRACEFILE") {
        let tracename = matches.value_of("TRACEFILE").unwrap();
        println!("Instruction trace will be written to {}", tracename);
        machine.write_trace_to(tracename);
    }
    if matches.is_present("TRACECOUNT") {
        machine.set_trace_count(value_t!(matches, "TRACECOUNT", usize).unwrap());
    }

    let segment = if matches.is_present("SEGMENT") {
        // XXX parse "0x" hex segment
        value_t!(matches, "SEGMENT", u16).unwrap()
    } else {
        0x0329
    };

    println!("loading program into segment {:04X}", segment);

    if let Some(e) = machine.load_executable_file(filename, segment) {
        panic!("error {}", e);
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();

    let scale_factor = value_t!(matches, "SCALE", f32).unwrap_or(2.);

    let initial_screen_width  = (320. * scale_factor) as u32;
    let initial_screen_height = (200. * scale_factor) as u32;
    let window = video_subsys.window(&format!("dustbox - {}", filename), initial_screen_width, initial_screen_height)
        .position_centered()
        .opengl()
        .allow_highdpi()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    // println!("renderer: sdl2 \"{}\"", canvas.info().name);

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

    let square_pixels = !matches.is_present("NOSQUARE");

    let mut frame_num = 0;
    'main: loop {
        let event_start = SystemTime::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit {..} => break 'main,

                Event::KeyDown {keycode: Some(keycode), keymod: modifier, ..} => {
                    if keycode == sdl2::keyboard::Keycode::Escape {
                        // break 'main
                    }

                    machine.keyboard_mut().add_keypress(keycode, modifier);
                }
                Event::MouseMotion {x, y, ..} => machine.mouse_mut().set_position(x, y),
                Event::MouseButtonDown {mouse_btn, ..} => {
                    match mouse_btn {
                        sdl2::mouse::MouseButton::Left => machine.mouse_mut().set_button(MouseButton::Left, true),
                        sdl2::mouse::MouseButton::Right => machine.mouse_mut().set_button(MouseButton::Right, true),
                        sdl2::mouse::MouseButton::Middle => machine.mouse_mut().set_button(MouseButton::Middle, true),
                        _ => {},
                    }
                }
                Event::MouseButtonUp {mouse_btn, ..} => {
                    match mouse_btn {
                        sdl2::mouse::MouseButton::Left => machine.mouse_mut().set_button(MouseButton::Left, false),
                        sdl2::mouse::MouseButton::Right => machine.mouse_mut().set_button(MouseButton::Right, false),
                        sdl2::mouse::MouseButton::Middle => machine.mouse_mut().set_button(MouseButton::Middle, false),
                        _ => {},
                    }
                }
                _ => {}
            }
        }

        let event_time = event_start.elapsed().unwrap();
        frame_event_sum += event_time;

        let frame_start = SystemTime::now();

        let locked_fps = 60;

        let frame = machine.gpu().render_frame(&machine.mmu);

        let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, frame.mode.swidth, frame.mode.sheight).unwrap();

        {
            // resize window to current screen mode sizes
            if frame.mode.mode != last_video_mode {
                let (internal_scale_x, internal_scale_y) = if square_pixels {
                    (scale_factor * frame.mode.scale_x, scale_factor * frame.mode.scale_y)
                } else {
                    (scale_factor, scale_factor)
                };

                // window size is the display size
                let window_width = (frame.mode.swidth as f32 * internal_scale_x) as u32;
                let window_height = (frame.mode.sheight as f32 * internal_scale_y) as u32;

                println!("Resizing window for mode {:02x} to {}x{} pixels, {}x{} frame size, scale factor {}x, internal scale x:{}, y:{}",
                    frame.mode.mode, window_width, window_height, frame.mode.swidth, frame.mode.sheight, scale_factor, internal_scale_x, internal_scale_y);

                let window = canvas.window_mut();
                window.set_size(window_width, window_height).unwrap();

                // XXX logical size is needed for correct mouse coordinates without having to divide them by scale, but it gives black top+bottom bars on win10
                let logical_w = (frame.mode.swidth as f32 * frame.mode.scale_x) as u32;
                let logical_h = (frame.mode.sheight as f32 * frame.mode.scale_y) as u32;
                canvas.set_logical_size(logical_w, logical_h).unwrap();

                last_video_mode = frame.mode.mode;
            }

            // run some instructions and progress scanline until screen is drawn
            for _ in 0..frame.mode.swidth {
                // XXX calculate the number cycles to execute for (1/30th sec ) / scanlines
                // XXX measure by instruction cycles
                let num_instr = 400;
                machine.execute_instructions(num_instr);
                if machine.cpu.fatal_error {
                    println!("cpu fatal error occured. stopping execution after {} instructions executed", machine.cpu.instruction_count);
                    break 'main;
                }
                machine.gpu_mut().progress_scanline();
            }
            let exec_time = frame_start.elapsed().unwrap();

            frame_exec_sum += exec_time;

            let render_start = SystemTime::now();

            let mut x: usize = 0;
            let mut y: usize = 0;

            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for pix in frame.data {
                    if let dustbox::gpu::ColorSpace::RGB(r, g, b) = pix {
                        let offset = y*pitch + x*3;
                        buffer[offset] = r;
                        buffer[offset + 1] = g;
                        buffer[offset + 2] = b;
                        x += 1;
                        if x >= frame.mode.swidth as usize {
                            x = 0;
                            y += 1;
                        }
                    }
                }
            }).unwrap();

            let render_time = render_start.elapsed().unwrap();
            frame_render_sum += render_time;

            // sleep for 1/60:th of a second, minus time it took to get here
            let mut sleep_time = Duration::new(0, 1_000_000_000 / locked_fps);
            if sleep_time >= event_time {
                sleep_time -= event_time;
            } else {
                println!("WARN: event handling is slow {:#?}", event_time);
                sleep_time = Duration::new(0, 0);
            }
            if sleep_time >= render_time {
                sleep_time -= render_time;
            } else {
                println!("WARN: render is slow {:#?}", render_time);
                sleep_time = Duration::new(0, 0);
            }
            if sleep_time >= exec_time {
                sleep_time -= exec_time;
            } else {
                println!("WARN: exec is slow {:#?}", exec_time);
                sleep_time = Duration::new(0, 0);
            }

            if DEBUG_PERFORMANCE {
                frame_num += 1;
                // println!("-- frame {}: sleep {:#?}, exec {:#?}, render {:#?}", frame_num, sleep_time, exec_time, render_time);
                if frame_num >= locked_fps {
                    frame_num = 0;
                    let frame_tot_sum = frame_event_sum + frame_exec_sum + frame_render_sum + frame_sleep_sum;

                    // in seconds
                    let frames = (frame_tot_sum.as_millis() as f64) / 1_000.;
                    let elapsed = (app_start.elapsed().unwrap().as_millis() as f64) / 1_000.;
                    let event = (frame_event_sum.as_millis() as f64) / 1_000.;
                    let exec = (frame_exec_sum.as_millis() as f64) / 1_000.;
                    let render = (frame_render_sum.as_millis() as f64) / 1_000.;
                    let sleep = (frame_sleep_sum.as_millis() as f64) / 1_000.;
                    println!("{} frames in {:.2}s after {:.2}s. event {:.2}s, exec {:.2}s, render {:.2}s, sleep {:.2}s",
                        locked_fps, frames, elapsed, event, exec, render, sleep);
                    frame_event_sum = Duration::new(0, 0);
                    frame_exec_sum = Duration::new(0, 0);
                    frame_render_sum = Duration::new(0, 0);
                    frame_sleep_sum = Duration::new(0, 0);
                }
            }

            sleep(sleep_time);
            frame_sleep_sum += sleep_time;
        }

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
