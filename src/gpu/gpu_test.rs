// this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;

use tera::Context;
use image;
use image::{ImageBuffer, Rgb};

use tools;
use cpu::CPU;
use memory::mmu::MMU;
use gpu::DACPalette;

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
        "../dos-software-decoding/demo-256/lets256/lets256.com",
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
        "../dos-software-decoding/demo-512/200h/200h.com",
        "../dos-software-decoding/demo-512/bars512/bars512.com",
        "../dos-software-decoding/demo-512/basicboy/basicboy.com",
        "../dos-software-decoding/demo-512/blaze/blaze5.com",
        "../dos-software-decoding/demo-512/bmatch/bmatch.com",
        "../dos-software-decoding/demo-512/callnow/callnow.com",
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
        // "../dos-software-decoding/new/stars.exe/stars.exe", // XXX exe file
        "../dos-software-decoding/demo-512/sun/sun.com",
        "../dos-software-decoding/demo-512/superusr/superusr.com",
        "../dos-software-decoding/demo-512/tiled/tiled.com",
        "../dos-software-decoding/demo-512/triopti2/triopti2.com",
        "../dos-software-decoding/demo-512/unknown/unknown.com",
        "../dos-software-decoding/demo-512/wamma/wamma.com",
        "../dos-software-decoding/demo-512/waves/waves.com",
    ];
    run_and_save_video_frames(test_bins, "demo_512", "512");
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
        "../dos-software-decoding/games-com/Hopper (1984)(Sega Entertainment Inc)/frogger.com",
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
        "../dos-software-decoding/games-com/Slow Mo (1990)(David Perrell)/moslo.com",
        "../dos-software-decoding/games-com/Slow Mo (1990)(David Perrell)/varislow.com",
        "../dos-software-decoding/games-com/Slow Mo Deluxe (1996)(David Perrell)/moslo.com",
        "../dos-software-decoding/games-com/Space Commanders II (1985)(Columbia Data Products)/space2.com",
        "../dos-software-decoding/games-com/Triskelion (1987)(Neil Rubenking)/triskel.com",
        "../dos-software-decoding/games-com/Zaxxon (1984)(Sega)/zaxxon.com",
    ];
    run_and_save_video_frames(test_bins, "games_com", "game");
}

fn run_and_save_video_frames(mut test_bins: Vec<&str>, group: &str, name_prefix: &str) {

    let mut out_images = vec![];

    while let Some(bin) = test_bins.pop() {
        println!("{}: {}", group, bin);

        let mut cpu = CPU::new(MMU::new());
        cpu.deterministic = true;
        match tools::read_binary(bin) {
            Ok(data) => cpu.load_com(&data),
            Err(err) => panic!("failed to read {}: {}", bin, err),
        }

        for _ in 0..7_000_000 {
            cpu.execute_instruction();
            if cpu.fatal_error {
                break;
            }
        }
        let path = Path::new(bin);

        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new();
        filename.push(format!("docs/render/{}/{}_", group, name_prefix));
        filename.push(stem.to_os_string());
        filename.push(".png");

        write_video_frame_to_disk(&cpu, filename.to_str().unwrap());

        let mut pub_filename = String::new();
        pub_filename.push_str(&format!("render/{}/{}_", group, name_prefix));
        pub_filename.push_str(stem.to_str().unwrap());
        pub_filename.push_str(".png");
        out_images.push(pub_filename);
    }

    let mut tera = compile_templates!("docs/templates/**/*");

    // disable autoescaping
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
fn draw_image(frame: &[u8], width: u32, height: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let img = ImageBuffer::from_fn(width, height, |x, y| {
        let offset = 3 * ((y * width) + x) as usize;
        let r = frame[offset];
        let g = frame[offset + 1];
        let b = frame[offset + 2];
        Rgb([r, g, b])
    });
    img
}

fn write_video_frame_to_disk(cpu: &CPU, pngfile: &str) {
    let mem = cpu.mmu.dump_mem();
    let frame = cpu.gpu.render_frame(&mem);
    let img = draw_image(&frame, cpu.gpu.width, cpu.gpu.height);
    if let Err(why) = img.save(pngfile) {
        println!("save err: {:?}", why);
    }
}
