#![allow(unused_imports)]

extern crate find_folder;

use std;
use conrod;
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::{DisplayBuild, Surface};
/*
use piston_window;
use piston_window::{PistonWindow, UpdateEvent, Window, WindowSettings};
use piston_window::{Flip, G2d, G2dTexture, Texture, TextureSettings};
use piston_window::OpenGL;
use piston_window::texture::UpdateTexture;
*/
use image;
use image::{ImageBuffer, RgbaImage, Rgba, GenericImage};
use memory::Memory;

use debugger;
use register::{AX, BX, CX, DX};

pub fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    // Construct the window.
    let display = glium::glutin::WindowBuilder::new()
        .with_vsync()
        .with_dimensions(WIDTH, HEIGHT)
        .with_title("x86emu")
        .with_multisampling(8)
        .build_glium()
        .unwrap();

    // construct our `Ui`.
    let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64])
        .theme(theme())
        .build();

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    let assets = find_folder::Search::KidsThenParents(3, 5)
        .for_folder("assets")
        .unwrap();
    let font_path = assets.join("fonts/IosevkaTerm/iosevka-term-light.ttf");
    ui.fonts.insert_from_file(font_path).unwrap();


    // Instantiate the generated list of widget identifiers.
    let ids = Ids::new(ui.widget_id_generator());

    let img = RgbaImage::new(320, 200);

    /*
    // Load the rust logo from file to a piston_window texture.
    let video_out: G2dTexture = {
        Texture::from_image(&mut window.factory, &img, &TextureSettings::new()).unwrap()
    };
*/

    // Load the rust logo from file to a piston_window texture.
    let video_tex: glium::texture::Texture2d = {
        let assets = find_folder::Search::ParentsThenKids(3, 3)
            .for_folder("assets")
            .unwrap();
        let path = assets.join("images/rust.png");
        let rgba_image = image::open(&std::path::Path::new(&path))
            .unwrap()
            .to_rgba();
        let image_dimensions = rgba_image.dimensions();
        let raw_image = glium::texture::RawImage2d::from_raw_rgba_reversed(rgba_image.into_raw(),
                                                                           image_dimensions);
        glium::texture::Texture2d::new(&display, raw_image).unwrap()
    };


    // Create our `conrod::image::Map` which describes each of our widget->image mappings.
    let mut image_map = conrod::image::Map::new();
    let video_id = image_map.insert(video_tex);

    let mut app = debugger::Debugger::new(video_id, img);


    // XXX for quick testing while building the ui
    app.load_binary("../dos-software-decoding/samples/bar/bar.com");


    // A type used for converting `conrod::render::Primitives` into `Command`s that can be used
    // for drawing to the glium `Surface`.
    //
    // Internally, the `Renderer` maintains:
    // - a `backend::glium::GlyphCache` for caching text onto a `glium::texture::Texture2d`.
    // - a `glium::Program` to use as the shader program when drawing to the `glium::Surface`.
    // - a `Vec` for collecting `backend::glium::Vertex`s generated when translating the
    // `conrod::render::Primitive`s.
    // - a `Vec` of commands that describe how to draw the vertices.
    let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();


    // Start the loop:
    //
    // - Poll the window for available events.
    // - Update the widgets via the `support::gui` fn.
    // - Render the current state of the `Ui`.
    // - Repeat.
    let mut event_loop = EventLoop::new();
    'main: loop {

        // Handle all events.
        for event in event_loop.next(&display) {

            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = conrod::backend::winit::convert(event.clone(), &display) {
                ui.handle_event(event);
                event_loop.needs_update();
            }

            match event {
                // Break from the loop upon `Escape`.
                glium::glutin::Event::KeyboardInput(_, _, Some(glium::glutin::VirtualKeyCode::Escape)) |
                    glium::glutin::Event::Closed =>
                        break 'main,
                _ => {}
            }
        }

        // Instantiate a GUI demonstrating every widget type provided by conrod.
        gui(&mut ui.set_widgets(), &ids, &mut app, &image_map);

        // Draw the `Ui`.
        if let Some(primitives) = ui.draw_if_changed() {
            renderer.fill(&display, primitives, &image_map);
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            renderer.draw(&display, &mut target, &image_map).unwrap();
            target.finish().unwrap();
        }
    }

}

/// A set of reasonable stylistic defaults that works for the `gui` below.
fn theme() -> conrod::Theme {
    use conrod::position::{Align, Direction, Padding, Position, Relative};
    conrod::Theme {
        name: "Demo Theme".to_string(),
        padding: Padding::none(),
        x_position: Position::Relative(Relative::Align(Align::Start), None),
        y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
        background_color: conrod::color::DARK_CHARCOAL,
        shape_color: conrod::color::LIGHT_CHARCOAL,
        border_color: conrod::color::BLACK,
        border_width: 0.0,
        label_color: conrod::color::WHITE,
        font_id: None,
        font_size_large: 26,
        font_size_medium: 18,
        font_size_small: 12,
        widget_styling: std::collections::HashMap::new(),
        mouse_drag_threshold: 0.0,
        double_click_threshold: std::time::Duration::from_millis(500),
    }
}



// Generate a unique `WidgetId` for each widget.
widget_ids! {
    pub struct Ids {
        canvas,

        // The disasm text
        disasm,

        // debugger buttons
        button_canvas, // container
        button_step,
        button_run,

        registers_bg,
        registers,

        video_out,
    }
}


/// Instantiate a GUI demonstrating every widget available in conrod.
fn gui<T>(ui: &mut conrod::UiCell,
          ids: &Ids,
          app: &mut debugger::Debugger,
          image_map: &conrod::image::Map<T>)
    where T: conrod::backend::glium::TextureDimensions
{
    use conrod::{widget, Colorable, Labelable, Positionable, Sizeable, Widget};
    use std::iter::once;

    const MARGIN: conrod::Scalar = 20.0;
    const DISASM_SIZE: conrod::FontSize = 12;

    // `Canvas` is a widget that provides some basic functionality for laying out children widgets.
    // By default, its size is the size of the window.
    widget::Canvas::new().pad(MARGIN).set(ids.canvas, ui);


    widget::Canvas::new()
        .align_bottom_of(ids.canvas)
        .kid_area_w_of(ids.canvas)
        .h(360.0)
        .color(conrod::color::TRANSPARENT)
        .pad(MARGIN)
        .set(ids.button_canvas, ui);

    let btn_step = widget::Button::new()
        .mid_left_of(ids.button_canvas)
        .w_h(80.0, 30.0)
        .label("Step");

    for _click in btn_step.set(ids.button_step, ui) {
        app.cpu.execute_instruction();
    }

    /*
    println!("updated app.video_out");
    // XXX get ref to texture using app.video_out_id
    if let Some(img) = image_map.get_mut(app.video_out_id) {
        for y in 0..app.cpu.gpu.height {
            for x in 0..app.cpu.gpu.width {
                let offset = 0xA0000 + ((y * app.cpu.gpu.width) + x) as usize;
                let byte = app.cpu.memory.memory[offset];
                let ref pal = app.cpu.gpu.palette[byte as usize];
                img.put_pixel(x, y, Rgba([pal.r, pal.g, pal.b, 255]));
            }
        }
    }
    */



    // video output
    widget::Image::new(app.video_out_id)
        .w_h(320.0, 200.0)
        .top_right_of(ids.canvas)
        .set(ids.video_out, ui);



    // group of buttons at the bottom
    let btn_run = widget::Button::new()
        .middle_of(ids.button_canvas)
        .w_h(80.0, 30.0)
        .label("Run");

    for _click in btn_run.set(ids.button_run, ui) {
        println!("XXXX run");
    }

    widget::RoundedRectangle::fill([300.0, 80.0], 10.0)
        .color(conrod::color::CHARCOAL.alpha(0.25))
        .mid_right_of(ids.canvas)
        .set(ids.registers_bg, ui);

    let reg_color = conrod::color::YELLOW; // XXX change color for changed regs

    let regs = app.cpu.print_registers();
    widget::Text::new(regs.as_ref())
        .font_size(DISASM_SIZE)
        .color(reg_color)
        .middle_of(ids.registers_bg)
        .set(ids.registers, ui);


    let disasm = app.disasm_n_instructions_to_text(20);

    widget::Text::new(disasm.as_ref())
        .font_size(DISASM_SIZE)
        .top_left_of(ids.canvas)
        .set(ids.disasm, ui);
}



pub struct EventLoop {
    ui_needs_update: bool,
    last_update: std::time::Instant,
}

impl EventLoop {
    pub fn new() -> Self {
        EventLoop {
            last_update: std::time::Instant::now(),
            ui_needs_update: true,
        }
    }

    /// Produce an iterator yielding all available events.
    pub fn next(&mut self, display: &glium::Display) -> Vec<glium::glutin::Event> {
        // We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
        // since the last yield.
        let last_update = self.last_update;
        let sixteen_ms = std::time::Duration::from_millis(16);
        let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
        if duration_since_last_update < sixteen_ms {
            std::thread::sleep(sixteen_ms - duration_since_last_update);
        }

        // Collect all pending events.
        let mut events = Vec::new();
        events.extend(display.poll_events());

        // If there are no events and the `Ui` does not need updating, wait for the next event.
        if events.is_empty() && !self.ui_needs_update {
            events.extend(display.wait_events().next());
        }

        self.ui_needs_update = false;
        self.last_update = std::time::Instant::now();

        events
    }

    /// Notifies the event loop that the `Ui` requires another update whether or not there are any
    /// pending events.
    ///
    /// This is primarily used on the occasion that some part of the `Ui` is still animating and
    /// requires further updates to do so.
    pub fn needs_update(&mut self) {
        self.ui_needs_update = true;
    }
}
