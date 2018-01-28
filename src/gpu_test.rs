// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

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
use mmu::MMU;
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
        "../dos-software-decoding/demo-512/blaze/blaze5.com",
        "../dos-software-decoding/demo-512/bmatch/bmatch.com",
        "../dos-software-decoding/demo-512/callnow/callnow.com",
        "../dos-software-decoding/demo-512/entry/entry.com",
        "../dos-software-decoding/demo-512/entry2/entry2.com",
        "../dos-software-decoding/demo-512/explsion/explsion.com",
        "../dos-software-decoding/demo-512/flower/flower.com",
        "../dos-software-decoding/demo-512/fountain_of_sparks/fountain_of_sparks.com",
        "../dos-software-decoding/demo-512/jomppa/jomppa.com",
        "../dos-software-decoding/demo-512/kintsmef/kintsmef.com",
        "../dos-software-decoding/demo-512/kpara8/kpara8.com",
        "../dos-software-decoding/demo-512/madness/madness.com",
        "../dos-software-decoding/demo-512/panyo/panyo.com",
        //"../dos-software-decoding/demo-512/plasmas/plasmas.com", // XXX crashes: index out of bounds: the len is 6 but the index is 6
        //"../dos-software-decoding/demo-512/rwater/rwater.com", // XXX self-extracting, needs another look
        "../dos-software-decoding/demo-512/triopti2/triopti2.com",
        "../dos-software-decoding/demo-512/wamma/wamma.com",
        "../dos-software-decoding/demo-512/waves/waves.com",
    ];

    run_and_save_video_frames(test_bins, "demo_512", "512");
}

fn run_and_save_video_frames(mut test_bins: Vec<&str>, group: &str, name_prefix: &str) {

    let mut out_images = vec![];

    while let Some(bin) = test_bins.pop() {
        println!("{}: {}", group, bin);

        let mut cpu = CPU::new(MMU::new());
        cpu.deterministic = true;
        let code = tools::read_binary(bin).unwrap();
        cpu.load_com(&code);

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

        let mem_dump = cpu.mmu.dump_mem();
        write_video_frame_to_disk(
            &mem_dump,
            filename.to_str().unwrap(),
            cpu.gpu.width,
            cpu.gpu.height,
            &cpu.gpu.pal,
        );

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

// render video frame, used for saving video frame to disk
fn draw_image(memory: &[u8], width: u32, height: u32, pal: &[DACPalette]) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let img = ImageBuffer::from_fn(width, height, |x, y| {
        let offset = 0xA_0000 + ((y * width) + x) as usize;
        let byte = memory[offset];
        let p = &pal[byte as usize];
        Rgb([p.r, p.g, p.b])
    });
    img
}

fn write_video_frame_to_disk(memory: &[u8], pngfile: &str, width: u32, height: u32, pal: &[DACPalette]) {
    let img = draw_image(memory, width, height, pal);
    if let Err(why) = img.save(pngfile) {
        println!("save err: {:?}", why);
    }
}
