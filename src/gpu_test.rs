// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;

use tera::Context;

use debugger::Debugger;
use tools;

#[test] #[ignore] // it is too expensive
fn demo_256() {
    let mut test_bins = vec![
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
        // "../dos-software-decoding/demo-256/hungecek/hungecek.com", // ip wraps
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

    let mut out_images = vec![];

    while let Some(bin) = test_bins.pop() {
        println!("demo_256: {}", bin);

        let mut debugger = Debugger::new();
        debugger.cpu.deterministic = true;
        let code = tools::read_binary(bin).unwrap();
        debugger.cpu.load_com(&code);

        debugger.step_into_n_instructions(5_000_000);
        let path = Path::new(bin);

        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new();
        filename.push("docs/render/demo-256/256_");
        filename.push(stem.to_os_string());
        filename.push(".png");

        //just to make the test a bit slower (sorry)
        let mem_dump = debugger.cpu.mmu.dump_mem();

        debugger.cpu.gpu.test_render_frame(&mem_dump, filename.to_str().unwrap());

        let mut pub_filename = String::new();
        pub_filename.push_str("render/demo-256/256_");
        pub_filename.push_str(stem.to_str().unwrap());
        pub_filename.push_str(".png");
        out_images.push(pub_filename);
        // XXX cpu.test_expect_memory_md5(x)
    }

    let mut tera = compile_templates!("docs/templates/**/*");

    // disable autoescaping completely
    tera.autoescape_on(vec![]);

    let mut context = Context::new();
    out_images.sort();
    context.add("out_images", &out_images);
    // add stuff to context
    match tera.render("demo256.html", &context) {
        Ok(res) => {
            use std::fs::File;
            use std::io::Write;
            let mut f = File::create("docs/demo256.html").expect("Unable to create file");
            f.write_all(res.as_bytes()).expect("Unable to write data");
        }
        Err(why) => println!("ERROR = {}", why),
    }
}
