// this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;

use tera::Context;
use image;
use image::{ImageBuffer, Rgb, Pixel, GenericImage};

use tools;
use cpu::{CPU, R};
use machine::Machine;
use memory::MMU;
use gpu::VideoModeBlock;

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
fn demo_256() {
    let test_bins = vec![
        "../dos-software-decoding/demo-256/4sum/4sum.com",
        "../dos-software-decoding/demo-256/165plasm/165plasm.com",
        "../dos-software-decoding/demo-256/244b/244b.com",
        "../dos-software-decoding/demo-256/alpc/alpc.com",
        "../dos-software-decoding/demo-256/beziesux/beziesux.com",
        "../dos-software-decoding/demo-256/blah/blah.com",
        "../dos-software-decoding/demo-256/bob/bob.com",
        "../dos-software-decoding/demo-256/chaos/chaos.com",
        "../dos-software-decoding/demo-256/conf/conf.com",
        "../dos-software-decoding/demo-256/ectotrax/ectotrax.com",
        "../dos-software-decoding/demo-256/fire/fire.com",
        "../dos-software-decoding/demo-256/fire2/fire2.com",
        "../dos-software-decoding/demo-256/fire17/fire17.com",
        "../dos-software-decoding/demo-256/flame2/flame2.com",
        "../dos-software-decoding/demo-256/flood/flood.com",
        "../dos-software-decoding/demo-256/fridge/fridge.com",
        "../dos-software-decoding/demo-256/hungecek/hungecek.com",
        "../dos-software-decoding/demo-256/julia/julia.com",
        "../dos-software-decoding/demo-256/lameland/lameland.com",
        "../dos-software-decoding/demo-256/lava/lava.com",
        "../dos-software-decoding/demo-256/leaf/leaf.com",
        "../dos-software-decoding/demo-256/luminous/luminous.com",
        "../dos-software-decoding/demo-256/lumps/lumps.com",
        "../dos-software-decoding/demo-256/miracle/miracle.com",
        "../dos-software-decoding/demo-256/nicefire/nicefire.com",
        "../dos-software-decoding/demo-256/optimize/optimize.com",
        "../dos-software-decoding/demo-256/pack/pack.com",
        "../dos-software-decoding/demo-256/phong/phong.com",
        "../dos-software-decoding/demo-256/pikku/pikku.com",
        "../dos-software-decoding/demo-256/pixelize/pixelize.com",
        "../dos-software-decoding/demo-256/plasma/plasma.com",
        "../dos-software-decoding/demo-256/plasmalr/plasmalr.com",
        "../dos-software-decoding/demo-256/plasmexp/plasmexp.com",
        "../dos-software-decoding/demo-256/platinum/platinum.com",
        "../dos-software-decoding/demo-256/proto256/proto256.com",
        "../dos-software-decoding/demo-256/riddle/riddle.com",
        "../dos-software-decoding/demo-256/saverave/saverave.com",
        "../dos-software-decoding/demo-256/snow/snow.com",
        "../dos-software-decoding/demo-256/specifi/specifi.com",
        "../dos-software-decoding/demo-256/spline/spline.com",
        "../dos-software-decoding/demo-256/sqwerz3/sqwerz3.com",
        "../dos-software-decoding/demo-256/static/static.com",
        "../dos-software-decoding/demo-256/water/water.com",
        "../dos-software-decoding/demo-256/wd95/wd95.com",
        "../dos-software-decoding/demo-256/wetwet/wetwet.com",
        "../dos-software-decoding/demo-256/x/x.com",
        "../dos-software-decoding/demo-256/zork/zork.com",
    ];
    run_and_save_video_frames(test_bins, "demo_256", "256");
}

#[test] #[ignore] // expensive test
fn demo_512() {
    let test_bins = vec![
        "../dos-software-decoding/demo-512/1/1.com",
        "../dos-software-decoding/demo-512/bars512/bars512.com",
        "../dos-software-decoding/demo-512/basicboy/basicboy.com",
        "../dos-software-decoding/demo-512/blaze/blaze5.com",
        "../dos-software-decoding/demo-512/bmatch/bmatch.com",
        "../dos-software-decoding/demo-512/dna/dna.com",
        "../dos-software-decoding/demo-512/fire/fire.com",
        "../dos-software-decoding/demo-512/jive/jive.com",
        "../dos-software-decoding/demo-512/jomppa/jomppa.com",
        "../dos-software-decoding/demo-512/kintsmef/kintsmef.com",
        "../dos-software-decoding/demo-512/legend/legend.com",
        "../dos-software-decoding/demo-512/lkccmini/lkccmini.com",
        "../dos-software-decoding/demo-512/madness/madness.com",
        "../dos-software-decoding/demo-512/mistake/mistake.com",
        "../dos-software-decoding/demo-512/morales/morales.com",
        "../dos-software-decoding/demo-512/skylight/skylight.com",
        "../dos-software-decoding/demo-512/tiled/tiled.com",
        "../dos-software-decoding/demo-512/triopti2/triopti2.com",
        "../dos-software-decoding/demo-512/unknown/unknown.com",
        "../dos-software-decoding/demo-512/wamma/wamma.com",
        "../dos-software-decoding/demo-512/waves/waves.com",
    ];
    run_and_save_video_frames(test_bins, "demo_512", "512");
}

#[test] #[ignore] // expensive test
fn demo_256_32bit() {
    let test_bins = vec![
        "../dos-software-decoding/demo-256-32bit/anding/anding.com",
        "../dos-software-decoding/demo-256-32bit/enchante/enchante.com",
        "../dos-software-decoding/demo-256-32bit/fire!/fire!.com",
        "../dos-software-decoding/demo-256-32bit/fire3d/fire3d.com",
        "../dos-software-decoding/demo-256-32bit/flame/flame.com",
        "../dos-software-decoding/demo-256-32bit/fractal/fractal.com",
        "../dos-software-decoding/demo-256-32bit/frcmirez/frcmirez.com",
        "../dos-software-decoding/demo-256-32bit/juls/juls.com",
        "../dos-software-decoding/demo-256-32bit/mbl/mbl.com",
        "../dos-software-decoding/demo-256-32bit/noc200/noc200.com",
        "../dos-software-decoding/demo-256-32bit/ripped/ripped.com",
        "../dos-software-decoding/demo-256-32bit/sierpins/sierpins.com",
        "../dos-software-decoding/demo-256-32bit/stars/stars.com",
        "../dos-software-decoding/demo-256-32bit/suka/suka.com",
        "../dos-software-decoding/demo-256-32bit/textaroo/textaroo.com",
        "../dos-software-decoding/demo-256-32bit/wtrfall/wtrfall.com",
        "../dos-software-decoding/demo-256-32bit/xwater/xwater.com",
    ];
    run_and_save_video_frames(test_bins, "demo_256_32bit", "256_32bit");
}

#[test] #[ignore] // expensive test
fn demo_512_32bit() {
    let test_bins = vec![
        "../dos-software-decoding/demo-512-32bit/200h/200h.com",
        "../dos-software-decoding/demo-512-32bit/blobsf/blobsf.com",
        "../dos-software-decoding/demo-512-32bit/bt7/bt7.com",
        "../dos-software-decoding/demo-512-32bit/distant/distant.com",
        "../dos-software-decoding/demo-512-32bit/ems/ems.com",
        "../dos-software-decoding/demo-512-32bit/entry2/entry2.com",
        "../dos-software-decoding/demo-512-32bit/europe/europe.com",
        "../dos-software-decoding/demo-512-32bit/fireline/fireline.com",
        "../dos-software-decoding/demo-512-32bit/fountain_of_sparks/fountain_of_sparks.com",
        "../dos-software-decoding/demo-512-32bit/fract/fract.com",
        "../dos-software-decoding/demo-512-32bit/glasenapy/glasenapy.com",
        "../dos-software-decoding/demo-512-32bit/gob4k/gob4k.com",
        "../dos-software-decoding/demo-512-32bit/grindkng/grindkng.com",
        "../dos-software-decoding/demo-512-32bit/rwater/rwater.com",
        "../dos-software-decoding/demo-512-32bit/voronoy/voronoy.com",
    ];
    run_and_save_video_frames(test_bins, "demo_512_32bit", "512_32bit");
}


#[test] #[ignore] // expensive test
fn demo_16k() {
    let test_bins = vec![
        "../dos-software-decoding/demo-16k/dreamers_bbs/dreamer.com",
        "../dos-software-decoding/demo-16k/microsoft_golf_cracktro/mgc.com",
    ];
    run_and_save_video_frames(test_bins, "demo_16k", "16k");
}

#[test] #[ignore] // expensive test
fn games_com() {
    let test_bins = vec![
        "../dos-software-decoding/games-com/8088 Othello (1985)(Bayley)/8088_othello.com",
        "../dos-software-decoding/games-com/Apple Panic (1982)(Broderbund Software Inc)/panic.com",
        "../dos-software-decoding/games-com/Astro Dodge (1982)(Digital Marketing Corporation)/astroids.com",
        "../dos-software-decoding/games-com/Beast (1984)(Dan Baker)/beast.com",
        "../dos-software-decoding/games-com/Blort (1987)(Hennsoft)/blort.com",
        "../dos-software-decoding/games-com/Crossfire (1982)(Sierra Online)/cfire.com",
        "../dos-software-decoding/games-com/Dig Dug (1982)(Namco)/digdug.com",
        "../dos-software-decoding/games-com/F15 Strike Eagle I (1986)(Microprose Software Inc)/f15.com",
        "../dos-software-decoding/games-com/Fire Fighter (1999)(Freeware)/firef.com",
        "../dos-software-decoding/games-com/Galaxian (1983)(Atari Inc)/galaxian.com",
        "../dos-software-decoding/games-com/Gnafu (1986)(Anonymous)/gnafu.com",
        "../dos-software-decoding/games-com/Gooku (1987)(Anonymous)/go-moku.com",
        "../dos-software-decoding/games-com/Hard Hat Mack (1984)(Electronic Arts Inc)/hhm.com",
        // "../dos-software-decoding/games-com/Hopper (1984)(Sega Entertainment Inc)/frogger.com", // also does not work well in dosbox-x
        "../dos-software-decoding/games-com/Invaders (1995)(Paul Reid)/invaders.com",
        "../dos-software-decoding/games-com/Kenguru (1997)(Pig Games)/keng.com",
        "../dos-software-decoding/games-com/Logical (1991)(Rainbow Arts)/logctrn1.com",
        "../dos-software-decoding/games-com/Madball (1985)(Microtec)/madball.com",
        "../dos-software-decoding/games-com/Megapede (1992)(Dom Early)/megapede.com",
        "../dos-software-decoding/games-com/Mind Field (1985)(Everett Kaser)/mine.com",
        "../dos-software-decoding/games-com/Ms Pacman (1983)(Atari Inc)/mspacman.com",
        "../dos-software-decoding/games-com/Mummies (1985)(Iain Brown)/mummies.com",
        "../dos-software-decoding/games-com/Paratrooper (1982)(Orion Software)/ptrooper.com",
        "../dos-software-decoding/games-com/Pc Man (1982)(Orion Software)/pcmanv1.com",
        "../dos-software-decoding/games-com/Pc Man (1982)(Orion Software)/pcmanv2.com",
        "../dos-software-decoding/games-com/Pente (1984)(Michael Leach)/pente.com",
        "../dos-software-decoding/games-com/Pipes (1983)(Creative Software)/pipes.com",
        "../dos-software-decoding/games-com/Pong (1986)(Imagine)/pong21.com",
        "../dos-software-decoding/games-com/Triskelion (1987)(Neil Rubenking)/triskel.com",
        "../dos-software-decoding/games-com/Zaxxon (1984)(Sega)/zaxxon.com",
    ];
    run_and_save_video_frames(test_bins, "games_com", "game");
}

fn run_and_save_video_frames(mut test_bins: Vec<&str>, group: &str, name_prefix: &str) {

    let mut out_images = vec![];

    while let Some(bin) = test_bins.pop() {
        println!("{}: {}", group, bin);

        let mut machine = Machine::default();
        machine.cpu.deterministic = true;
        match tools::read_binary(bin) {
            Ok(data) => machine.load_executable(&data),
            Err(err) => panic!("failed to read {}: {}", bin, err),
        }

        for _ in 0..7_000_000 {
            machine.execute_instruction();
            if machine.cpu.fatal_error {
                break;
            }
        }
        let path = Path::new(bin);

        let _ = fs::create_dir(&format!("docs/render/{}", group));
        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new(); // XXX base on dirname
        filename.push(format!("docs/render/{}/{}_", group, name_prefix));
        filename.push(stem.to_os_string());
        filename.push(".png");

        if write_video_frame_to_disk(&machine, filename.to_str().unwrap()) {
            let mut pub_filename = String::new();
            pub_filename.push_str(&format!("render/{}/{}_", group, name_prefix));
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
    context.add("out_images", &out_images);
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
fn draw_image(frame: &[u8], mode: &VideoModeBlock) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let img = ImageBuffer::from_fn(mode.swidth, mode.sheight, |x, y| {
        let offset = 3 * ((y * mode.swidth) + x) as usize;
        let r = frame[offset];
        let g = frame[offset + 1];
        let b = frame[offset + 2];
        Rgb([r, g, b])
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
