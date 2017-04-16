#![allow(unused_imports)]

extern crate find_folder;

use std;
use conrod;
use piston_window;
use piston_window::{PistonWindow, UpdateEvent, Window, WindowSettings};
use piston_window::{Flip, G2d, G2dTexture, Texture, TextureSettings};
use piston_window::OpenGL;
use piston_window::texture::UpdateTexture;
use image::ImageBuffer;
use memory::Memory;

use debugger;
use register::{AX, BX, CX, DX};

pub fn main() {
    const WIDTH: u32 = 800;
    const HEIGHT: u32 = 600;

    // Construct the window.
    let mut window: PistonWindow = WindowSettings::new("x86emu", [WIDTH, HEIGHT])
                .opengl(OpenGL::V3_2) // If not working, try `OpenGL::V2_1`.
                .samples(4)
                .exit_on_esc(true)
                .vsync(true)
                .build()
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

    // Create a texture to use for efficiently caching text on the GPU.
    let mut text_vertex_data = Vec::new();
    let (mut glyph_cache, mut text_texture_cache) = {
        const SCALE_TOLERANCE: f32 = 0.1;
        const POSITION_TOLERANCE: f32 = 0.1;
        let cache =
            conrod::text::GlyphCache::new(WIDTH, HEIGHT, SCALE_TOLERANCE, POSITION_TOLERANCE);
        let buffer_len = WIDTH as usize * HEIGHT as usize;
        let init = vec![128; buffer_len];
        let settings = TextureSettings::new();
        let factory = &mut window.factory;
        let texture = G2dTexture::from_memory_alpha(factory, &init, WIDTH, HEIGHT, &settings)
            .unwrap();
        (cache, texture)
    };

    let pixels = vec![0u8; 320*200*4]; // XXXX this should happen in render_frame()
    let img = ImageBuffer::from_raw(320, 200, pixels).unwrap();

    // Load the rust logo from file to a piston_window texture.
    let video_out: G2dTexture = {
        Texture::from_image(&mut window.factory, &img, &TextureSettings::new()).unwrap()
    };

    // Create our `conrod::image::Map` which describes each of our widget->image mappings.
    // In our case we only have one image, however the macro may be used to list multiple.
    let mut image_map = conrod::image::Map::new();

    // Instantiate the generated list of widget identifiers.
    let ids = Ids::new(ui.widget_id_generator());
    let video_id = image_map.insert(video_out);

    let mut app = debugger::Debugger::new(video_id);


    // XXX for quick testing while building the ui
    app.load_binary("../dos-software-decoding/samples/bar/bar.com");

    // Poll events from the window.
    while let Some(event) = window.next() {

        // Convert the piston event to a conrod event.
        let size = window.size();
        let (win_w, win_h) = (size.width as conrod::Scalar, size.height as conrod::Scalar);
        if let Some(e) = conrod::backend::piston::event::convert(event.clone(), win_w, win_h) {
            ui.handle_event(e);
        }

        event.update(|_| {
                         let mut ui = ui.set_widgets();
                         gui(&mut ui, &ids, &mut app);
                     });

        window.draw_2d(&event, |context, graphics| {
            if let Some(primitives) = ui.draw_if_changed() {

                // A function used for caching glyphs to the texture cache.
                let cache_queued_glyphs = |graphics: &mut G2d,
                                           cache: &mut G2dTexture,
                                           rect: conrod::text::rt::Rect<u32>,
                                           data: &[u8]| {
                    let offset = [rect.min.x, rect.min.y];
                    let size = [rect.width(), rect.height()];
                    let format = piston_window::texture::Format::Rgba8;
                    let encoder = &mut graphics.encoder;
                    text_vertex_data.clear();
                    text_vertex_data.extend(data.iter().flat_map(|&b| vec![255, 255, 255, b]));
                    UpdateTexture::update(cache,
                                          encoder,
                                          format,
                                          &text_vertex_data[..],
                                          offset,
                                          size)
                            .expect("failed to update texture")
                };

                // Specify how to get the drawable texture from the image. In this case, the image
                // *is* the texture.
                fn texture_from_image<T>(img: &T) -> &T {
                    img
                }

                // Draw the conrod `render::Primitives`.
                conrod::backend::piston::draw::primitives(primitives,
                                                          context,
                                                          graphics,
                                                          &mut text_texture_cache,
                                                          &mut glyph_cache,
                                                          &image_map,
                                                          cache_queued_glyphs,
                                                          texture_from_image);
            }
        });
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
fn gui(ui: &mut conrod::UiCell, ids: &Ids, app: &mut debugger::Debugger) {
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

    // video output
    widget::Image::new(app.video_out_id) // XXX
        .w_h(320.0, 200.0)
        .down(60.0)
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

    widget::RoundedRectangle::fill([300.0, 90.0], 10.0)
        .color(conrod::color::CHARCOAL.alpha(0.25))
        .mid_right_of(ids.canvas)
        .set(ids.registers_bg, ui);

    let regs = app.cpu.print_registers();
    widget::Text::new(regs.as_ref())
        .font_size(DISASM_SIZE)
        .middle_of(ids.registers_bg)
        .set(ids.registers, ui);


    let disasm = app.disasm_n_instructions_to_text(20);

    widget::Text::new(disasm.as_ref())
        .font_size(DISASM_SIZE)
        .top_left_of(ids.canvas)
        .set(ids.disasm, ui);
}



pub struct Renderer {}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {}
    }

    // draws a VGA frame. XXX should be called in a callback from render loop
    fn draw_frame(&mut self, memory: &mut Memory) {
        /*
        // println!("redraw_window");

        let mut canvas = ImageBuffer::new(self.width, self.height);

        let mut texture =
            Texture::from_image(&mut self.window.factory, &canvas, &TextureSettings::new())
                .unwrap();

        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA0000 + ((y * self.width) + x) as usize;
                let byte = memory.memory[offset];
                let ref pal = self.palette[byte as usize];
                canvas.put_pixel(x, y, Rgba([pal.r, pal.g, pal.b, 255]));
            }
        }

        texture
            .update(&mut self.window.encoder, &canvas)
            .unwrap();

        // HACK to redraw window without locking up
        for _ in 0..3 {
            match self.window.next() {
                Some(e) => {
                    self.window
                        .draw_2d(&e, |c, g| {
                            clear([1.0; 4], g);
                            image(&texture, c.transform, g);
                        });
                }
                None => {}
            }
        }
        */
    }
}
