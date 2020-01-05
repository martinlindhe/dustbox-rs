extern crate dustbox;

extern crate image;
extern crate tera;

use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use tera::{Tera, Context};

use dustbox::machine::Machine;
use dustbox::tools;

fn main() {
    // XXX TODO read harness listing from text file + take filename as cli argument
    demo_com_16bit();
    demo_com_32bit();
    games_commercial();
}


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
        path.to_owned() + "unknown/unknown.com",
        path.to_owned() + "wamma/wamma.com",
        path.to_owned() + "waves/waves.com",

        path.to_owned() + "dreamers_bbs/dreamer.com",
        path.to_owned() + "microsoft_golf_cracktro/mgc.com",
    ];

    run_and_save_video_frames(test_bins, "demo_com_16bit", "");
}

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
        path.to_owned() + "fireline/fireline.com",
        path.to_owned() + "fountain_of_sparks/fountain_of_sparks.com",
        path.to_owned() + "glasenapy/glasenapy.com",
        path.to_owned() + "gob4k/gob4k.com",
        path.to_owned() + "rwater/rwater.com",
        path.to_owned() + "voronoy/voronoy.com",
    ];
    run_and_save_video_frames(test_bins, "demo_com_32bit", "");
}

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
        path.to_owned() + "Paratrooper (1982)(Orion Software)/ptrooper.com",
        path.to_owned() + "Pc Man (1982)(Orion Software)/pcmanv1.com",
        path.to_owned() + "Pc Man (1982)(Orion Software)/pcmanv2.com",
        path.to_owned() + "Pente (1984)(Michael Leach)/pente.com",
        path.to_owned() + "Pipes (1983)(Creative Software)/pipes.com",
        path.to_owned() + "Pong (1986)(Imagine)/pong21.com",
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

        let mut machine = Machine::deterministic();
        match tools::read_binary(&bin) {
            Ok(data) => machine.load_executable(&data, 0x085F),
            Err(err) => panic!("failed to read {}: {}", bin, err),
        }

        machine.execute_instructions(7_000_000);

        if !Path::new(&format!("docs/render/{}", group)).exists() {
            if let Err(e) = fs::create_dir(&format!("docs/render/{}", group)) {
                panic!("create_dir failed {}", e);
            }
        }

        let path = Path::new(&bin);
        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new(); // XXX base on dirname
        let outname = &format!("render/{}/{:02x}_{}", group, machine.gpu_mut().unwrap().mode.mode, name_prefix);
        filename.push(format!("docs/{}", outname));
        filename.push(stem.to_os_string());
        filename.push(".png");

        if write_video_frame_to_disk(&mut machine, filename.to_str().unwrap()) {
            let mut pub_filename = String::new();
            pub_filename.push_str(&outname);
            pub_filename.push_str(stem.to_str().unwrap());
            pub_filename.push_str(".png");
            out_images.push(pub_filename);
        } else {
            println!("failed to write {} to disk", filename.to_str().unwrap());
        }
    }

    let mut tera = match Tera::new("harness/templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // disable auto-escaping
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    out_images.sort();
    context.insert("out_images", &out_images);
    // add stuff to context
    match tera.render("test_category.tpl.html", &context) {
        Ok(res) => {
            let mut f = File::create(format!("docs/{}.html", group)).expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => panic!(format!("{}", why)),
    }
}

// returns true on success
fn write_video_frame_to_disk(machine: &mut Machine, pngfile: &str) -> bool {
    let frame = machine.gpu().unwrap().render_frame(&machine.mmu);
    if frame.data.is_empty() {
        println!("ERROR: no frame rendered");
        return false;
    }
    let img = frame.draw_image();
    if let Err(why) = img.save(pngfile) {
        println!("save err: {:?}", why);
        return false;
    }
    true
}

