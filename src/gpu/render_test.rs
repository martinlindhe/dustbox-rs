// this is a collection of graphic tests using classic ms-dos demos

use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::panic;

use tera::Context;
use image;
use image::{ImageBuffer, Rgb, Pixel, GenericImage};

use tools;
use cpu::{CPU, R};
use machine::Machine;
use memory::MMU;
use gpu::VideoModeBlock;
use gpu::palette::ColorSpace;

#[test]
fn can_get_palette_entry() {
    let mut machine = Machine::default();
    let code: Vec<u8> = vec![
        0xB3, 0x03,         // mov bl,0x3
        0xB8, 0x15, 0x10,   // mov ax,0x1015
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x00, machine.cpu.get_r8(R::DH)); // red
    assert_eq!(0x2A, machine.cpu.get_r8(R::CH)); // green
    assert_eq!(0x2A, machine.cpu.get_r8(R::CL)); // blue
}

#[test]
fn can_set_palette_entry() {
    let mut machine = Machine::default();
    let code: Vec<u8> = vec![
        0xBB, 0x03, 0x00,   // mov bx,0x3
        0xB5, 0x3F,         // mov ch,0x3f      ; red
        0xB1, 0x3F,         // mov cl,0x3f      ; green
        0xB6, 0x3F,         // mov dh,0x3f      ; blue
        0xB8, 0x10, 0x10,   // mov ax,0x1010
        0xCD, 0x10,         // int 0x10

        0xB3, 0x03,         // mov bl,0x3
        0xB8, 0x15, 0x10,   // mov ax,0x1015
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x3F, machine.cpu.get_r8(R::DH)); // red
    assert_eq!(0x3F, machine.cpu.get_r8(R::CH)); // green
    assert_eq!(0x3F, machine.cpu.get_r8(R::CL)); // blue
}

#[test]
fn can_get_font_info() {
    let mut machine = Machine::default();
    let code: Vec<u8> = vec![
        0xB8, 0x30, 0x11,   // mov ax,0x1130  ; 1130 = get font info
        0xB7, 0x06,         // mov bh,0x6     ; get ROM 8x16 font (MCGA, VGA)
        0xCD, 0x10,         // int 0x10       ; es:bp = c000:1700 i dosbox
    ];
    machine.load_executable(&code);

    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0xC000, machine.cpu.get_r16(R::ES));
    assert_eq!(0x1700, machine.cpu.get_r16(R::BP));
}

#[test]
fn can_int10_put_pixel() {
    let mut machine = Machine::default();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,   // mov ax,0x13
        0xCD, 0x10,         // int 0x10
        0xB4, 0x0C,         // mov ah,0xc       ; int 10h, ah = 0Ch
        0xB7, 0x00,         // mov bh,0x0
        0xB0, 0x0D,         // mov al,0xd       color
        0xB9, 0x01, 0x00,   // mov cx,0x1       x
        0xBA, 0x04, 0x00,   // mov dx,0x4       y
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(2);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x0113, machine.cpu.regs.ip);

    let frame = machine.hw.gpu.render_frame(&machine.hw.mmu);
    let mut img = draw_image(&frame, &machine.hw.gpu.mode);
    let img = img.sub_image(0, 0, 6, 6).to_image();
    assert_eq!("\
......
......
......
......
.O....
......
", draw_ascii(&img));
}

#[test]
fn can_write_vga_text() {
let mut machine = Machine::default();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,   // mov ax,0x13
        0xCD, 0x10,         // int 0x10
        0xB4, 0x0A,         // mov ah,0xa       ; int 10h, ah = 0Ah
        0xB0, 0x53,         // mov al,'S'       ; char
        0xB7, 0x00,         // mov bh,0x0       ; page
        0xB3, 0x01,         // mov bl,0x1       ; attrib
        0xB9, 0x01, 0x00,   // mov cx,0x1       ; count
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(2);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x0112, machine.cpu.regs.ip);

    let frame = machine.hw.gpu.render_frame(&machine.hw.mmu);
    let mut img = draw_image(&frame, &machine.hw.gpu.mode);
    let img = img.sub_image(0, 0, 8, 8).to_image();
    assert_eq!("\
.,,,,...
,,..,,..
,,,.....
.,,,....
...,,,..
,,..,,..
.,,,,...
........
", draw_ascii(&img));
}

fn draw_ascii(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> String {
    let mut res = String::new();
    for y in 0..img.height() {
        for x in 0..img.width() {
            let pixel = img.get_pixel(x, y);
            res.push(pixel_256_to_ascii(pixel));
        }
        res.push('\n');
    }
    res
}

fn pixel_256_to_ascii(v: &image::Rgb<u8>) -> char {
    let vals: [char; 9] = ['.', ',', '+', 'o', '5', '6', 'O', '0', '#'];
	let col = v.to_rgb();
    let avg = (col.data[0] as f64 + col.data[1] as f64 + col.data[2] as f64) / 3.;
    let n = scale(avg, 0., 255., 0., (vals.len() - 1) as f64) as usize;
    assert_eq!(true, n <= vals.len());

    vals[n]
}

fn scale(value_in:f64, base_min:f64, base_max:f64, limit_min:f64, limit_max:f64) -> f64 {
	((limit_max - limit_min) * (value_in - base_min) / (base_max - base_min)) + limit_min
}

#[test] #[ignore] // expensive test
fn demo_com_16bit() {
    let path = "../dos-software-decoding/demo-com-16bit/";
    let test_bins = vec![
        path.to_owned() + "4sum/4sum.com",
        path.to_owned() + "165plasm/165plasm.com",
        path.to_owned() + "244b/244b.com",
        path.to_owned() + "alpc/alpc.com",
        path.to_owned() + "beziesux/beziesux.com",
        path.to_owned() + "blah/blah.com",
        path.to_owned() + "bob/bob.com",
        path.to_owned() + "chaos/chaos.com",
        path.to_owned() + "conf/conf.com",
        path.to_owned() + "ectotrax/ectotrax.com",
        path.to_owned() + "fire/fire.com",
        path.to_owned() + "fire2/fire2.com",
        path.to_owned() + "fire17/fire17.com",
        path.to_owned() + "flame2/flame2.com",
        path.to_owned() + "flood/flood.com",
        path.to_owned() + "fridge/fridge.com",
        path.to_owned() + "hungecek/hungecek.com",
        path.to_owned() + "julia/julia.com",
        path.to_owned() + "lameland/lameland.com",
        path.to_owned() + "lava/lava.com",
        path.to_owned() + "leaf/leaf.com",
        path.to_owned() + "luminous/luminous.com",
        path.to_owned() + "lumps/lumps.com",
        path.to_owned() + "miracle/miracle.com",
        path.to_owned() + "nicefire/nicefire.com",
        path.to_owned() + "optimize/optimize.com",
        path.to_owned() + "pack/pack.com",
        path.to_owned() + "phong/phong.com",
        path.to_owned() + "pikku/pikku.com",
        path.to_owned() + "pixelize/pixelize.com",
        path.to_owned() + "plasma/plasma.com",
        path.to_owned() + "plasmalr/plasmalr.com",
        path.to_owned() + "plasmexp/plasmexp.com",
        path.to_owned() + "platinum/platinum.com",
        path.to_owned() + "proto256/proto256.com",
        path.to_owned() + "riddle/riddle.com",
        path.to_owned() + "saverave/saverave.com",
        path.to_owned() + "snow/snow.com",
        path.to_owned() + "specifi/specifi.com",
        path.to_owned() + "spline/spline.com",
        path.to_owned() + "sqwerz3/sqwerz3.com",
        path.to_owned() + "static/static.com",
        path.to_owned() + "water/water.com",
        path.to_owned() + "wd95/wd95.com",
        path.to_owned() + "wetwet/wetwet.com",
        path.to_owned() + "x/x.com",
        path.to_owned() + "zork/zork.com",

        path.to_owned() + "1/1.com",
        path.to_owned() + "bars512/bars512.com",
        path.to_owned() + "basicboy/basicboy.com",
        path.to_owned() + "blaze/blaze5.com",
        path.to_owned() + "bmatch/bmatch.com",
        path.to_owned() + "fire/fire.com",
        path.to_owned() + "jive/jive.com",
        path.to_owned() + "jomppa/jomppa.com",
        // path.to_owned() + "kintsmef/kintsmef.com", // XXX depacker runs incorrectly, dosbox-x handles it
        path.to_owned() + "legend/legend.com",
        path.to_owned() + "lkccmini/lkccmini.com",
        path.to_owned() + "madness/madness.com",
        path.to_owned() + "mistake/mistake.com",
        path.to_owned() + "morales/morales.com",
        path.to_owned() + "skylight/skylight.com",
        path.to_owned() + "tiled/tiled.com",
        path.to_owned() + "triopti2/triopti2.com",
        path.to_owned() + "unknown/unknown.com",
        path.to_owned() + "wamma/wamma.com",
        path.to_owned() + "waves/waves.com",

        path.to_owned() + "dreamers_bbs/dreamer.com",
        path.to_owned() + "microsoft_golf_cracktro/mgc.com",
    ];

    run_and_save_video_frames(test_bins, "demo_com_16bit", "");
}

#[test] #[ignore] // expensive test
fn demo_com_32bit() {
    let path = "../dos-software-decoding/demo-com-32bit/";
    let test_bins = vec![
        path.to_owned() + "anding/anding.com",
        path.to_owned() + "enchante/enchante.com",
        path.to_owned() + "fire!/fire!.com",
        path.to_owned() + "fire3d/fire3d.com",
        path.to_owned() + "flame/flame.com",
        path.to_owned() + "fractal/fractal.com",
        path.to_owned() + "frcmirez/frcmirez.com",
        path.to_owned() + "juls/juls.com",
        path.to_owned() + "mbl/mbl.com",
        path.to_owned() + "noc200/noc200.com",
        path.to_owned() + "ripped/ripped.com",
        path.to_owned() + "sierpins/sierpins.com",
        path.to_owned() + "stars/stars.com",
        path.to_owned() + "suka/suka.com",
        path.to_owned() + "textaroo/textaroo.com",
        path.to_owned() + "wtrfall/wtrfall.com",
        path.to_owned() + "xwater/xwater.com",

        path.to_owned() + "200h/200h.com",
        path.to_owned() + "blobsf/blobsf.com",
        path.to_owned() + "bt7/bt7.com",
        path.to_owned() + "distant/distant.com",
        path.to_owned() + "ems/ems.com",
        path.to_owned() + "entry2/entry2.com",
        path.to_owned() + "europe/europe.com",
        path.to_owned() + "fireline/fireline.com",
        path.to_owned() + "fountain_of_sparks/fountain_of_sparks.com",
        path.to_owned() + "glasenapy/glasenapy.com",
        path.to_owned() + "gob4k/gob4k.com",
        path.to_owned() + "grindkng/grindkng.com",
        path.to_owned() + "rwater/rwater.com",
        path.to_owned() + "voronoy/voronoy.com",
    ];
    run_and_save_video_frames(test_bins, "demo_com_32bit", "");
}

#[test] #[ignore] // expensive test
fn games_commercial() {
    let path = "../dos-software-decoding/games-com-commercial/";
    let test_bins = vec![
        path.to_owned() + "8088 Othello (1985)(Bayley)/8088_othello.com",
        path.to_owned() + "Apple Panic (1982)(Broderbund Software Inc)/panic.com",
        path.to_owned() + "Astro Dodge (1982)(Digital Marketing Corporation)/astroids.com",
        path.to_owned() + "Beast (1984)(Dan Baker)/beast.com",
        path.to_owned() + "Blort (1987)(Hennsoft)/blort.com",
        path.to_owned() + "Crossfire (1982)(Sierra Online)/cfire.com",
        path.to_owned() + "Dig Dug (1982)(Namco)/digdug.com",
        path.to_owned() + "F15 Strike Eagle I (1986)(Microprose Software Inc)/f15.com",
        path.to_owned() + "Fire Fighter (1999)(Freeware)/firef.com",
        path.to_owned() + "Galaxian (1983)(Atari Inc)/galaxian.com",
        path.to_owned() + "Gnafu (1986)(Anonymous)/gnafu.com",
        path.to_owned() + "Gooku (1987)(Anonymous)/go-moku.com",
        path.to_owned() + "Hard Hat Mack (1984)(Electronic Arts Inc)/hhm.com",
        path.to_owned() + "Invaders (1995)(Paul Reid)/invaders.com",
        path.to_owned() + "Kenguru (1997)(Pig Games)/keng.com",
        path.to_owned() + "Logical (1991)(Rainbow Arts)/logctrn1.com",
        path.to_owned() + "Madball (1985)(Microtec)/madball.com",
        path.to_owned() + "Mind Field (1985)(Everett Kaser)/mine.com",
        path.to_owned() + "Ms Pacman (1983)(Atari Inc)/mspacman.com",
        path.to_owned() + "Mummies (1985)(Iain Brown)/mummies.com",
        path.to_owned() + "Paratrooper (1982)(Orion Software)/ptrooper.com",
        path.to_owned() + "Pc Man (1982)(Orion Software)/pcmanv1.com",
        path.to_owned() + "Pc Man (1982)(Orion Software)/pcmanv2.com",
        path.to_owned() + "Pente (1984)(Michael Leach)/pente.com",
        path.to_owned() + "Pipes (1983)(Creative Software)/pipes.com",
        path.to_owned() + "Pong (1986)(Imagine)/pong21.com",
        path.to_owned() + "Star Chamber (1987)(Russco)/starcham.com",
        path.to_owned() + "Snake Game (1992)(Freeware)/snake.com",
        path.to_owned() + "Sky Runner (1987)(Anonymous)/sky1.com",
        path.to_owned() + "Sky Runner (1987)(Anonymous)/sky2.com",
        path.to_owned() + "Shamus (1984)(Synapse Software)/shamus.com",
        path.to_owned() + "Rollo And The Brush Brothers (1983)(Windwill Software)/rollo.com",
        path.to_owned() + "Robotron 2084 (1984)(Williams Electronics)/rt2084.com",
        path.to_owned() + "Turbo Bridge (1985)(Anonymous)/tbridge.com",
        path.to_owned() + "Triskelion (1987)(Neil Rubenking)/triskel.com",
        path.to_owned() + "Vlak (1993)(Miroslav Nemecek)/vlak.com",
        path.to_owned() + "Yatzy (1984)(Jan Ivar Gundersen)/yatzy.com",
        path.to_owned() + "Zaxxon (1984)(Sega)/zaxxon.com",
        path.to_owned() + "Zyll (1984)(Marshal Linder)/zyll.com",
    ];
    run_and_save_video_frames(test_bins, "games_com", "");
}

fn run_and_save_video_frames(mut test_bins: Vec<String>, group: &str, name_prefix: &str) {

    let mut out_images = vec![];

    while let Some(bin) = test_bins.pop() {
        println!("{}: {}", group, bin);

        let mut machine = Machine::default();
        machine.cpu.deterministic = true;
        match tools::read_binary(&bin) {
            Ok(data) => machine.load_executable(&data),
            Err(err) => panic!("failed to read {}: {}", bin, err),
        }

        machine.execute_instructions(7_000_000);

        let path = Path::new(&bin);

        let _ = fs::create_dir(&format!("docs/render/{}", group));
        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new(); // XXX base on dirname
        let outname = &format!("render/{}/{:02x}_{}", group, machine.hw.gpu.mode.mode, name_prefix);
        filename.push(format!("docs/{}", outname));
        filename.push(stem.to_os_string());
        filename.push(".png");

        if write_video_frame_to_disk(&machine, filename.to_str().unwrap()) {
            let mut pub_filename = String::new();
            pub_filename.push_str(&outname);
            pub_filename.push_str(stem.to_str().unwrap());
            pub_filename.push_str(".png");
            out_images.push(pub_filename);
            /*
            let frame = machine.hw.gpu.render_frame(&machine.hw.mmu);
            let img = draw_image(&frame, &machine.hw.gpu.mode);
            print!("{}", draw_ascii(&img));
            */
        } else {
            println!("failed to write {} to disk", filename.to_str().unwrap());
        }
    }

    let mut tera = compile_templates!("docs/templates/**/*");

    // disable auto-escaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    out_images.sort();
    context.insert("out_images", &out_images);
    // add stuff to context
    match tera.render("test_category.tpl.html", &context) {
        Ok(res) => {
            use std::fs::File;
            use std::io::Write;
            let mut f = File::create(format!("docs/{}.html", group)).expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => println!("ERROR = {}", why),
    }
}

// converts a video frame to a ImageBuffer, used for saving video frame to disk in gpu_test
fn draw_image(frame: &[ColorSpace], mode: &VideoModeBlock) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let img = ImageBuffer::from_fn(mode.swidth, mode.sheight, |x, y| {
        let offset = ((y * mode.swidth) + x) as usize;

        if let ColorSpace::RGB(r, g, b) = frame[offset] {
            Rgb([r, g, b])
        } else {
            println!("error unhandled colorspace not RGB");
            Rgb([0, 0, 0])
        }
    });
    img
}

// returns true on success
fn write_video_frame_to_disk(machine: &Machine, pngfile: &str) -> bool {
    let frame = machine.hw.gpu.render_frame(&machine.hw.mmu);
    if frame.len() == 0 {
        println!("ERROR: no frame rendered");
        return false;
    }
    let img = draw_image(&frame, &machine.hw.gpu.mode);
    if let Err(why) = img.save(pngfile) {
        println!("save err: {:?}", why);
        return false;
    }
    return true;
}
