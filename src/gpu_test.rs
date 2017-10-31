// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;

use cpu::CPU;
use tools;

#[test]
fn demo_256() {
    let mut test_bins = vec![
        // "../dos-software-decoding/demo-256/165plasm/debug/165plasd.com", // XXX hits corrupted op: "unknown op C8 at 085F:0164 (008754 flat), 1318 instructions executed"
        "../dos-software-decoding/demo-256/244b/244b.com",         // some gfx
        "../dos-software-decoding/demo-256/alpc/alpc.com",         // some gfx
        "../dos-software-decoding/demo-256/beziesux/beziesux.com", // some gfx
        // "../dos-software-decoding/demo-256/blah/blah.com",      // black screen, crash after 50k instr or so (executing 00:s), ip wrap
        "../dos-software-decoding/demo-256/bob/bob.com",           // black screen
        "../dos-software-decoding/demo-256/chaos/chaos.com",       // black screen, needs font data
        //"../dos-software-decoding/demo-256/conf/conf.com",       // waits for ENTER press, FIXME: inject enter key press to progress demo
        "../dos-software-decoding/demo-256/ectotrax/ectotrax.com", // black screen, needs font data
        "../dos-software-decoding/demo-256/enchante/enchante.com", // black screen
        "../dos-software-decoding/demo-256/fire/fire.com",         // black screen
        "../dos-software-decoding/demo-256/fire2/fire2.com",       // black screen, needs font data
        "../dos-software-decoding/demo-256/fire3d/fire3d.com",     // black screen
        "../dos-software-decoding/demo-256/fire17/fire17.com",     // black screen, crash because corrupted code: unknown op 1D at 085F:02A9 (008899 flat), 12799 instructions executed
        "../dos-software-decoding/demo-256/flame2/flame2.com",     // black screen
        "../dos-software-decoding/demo-256/fracscrl/fracscrl.com", // red screen
        "../dos-software-decoding/demo-256/fractal/fractal.com",   // black screen, crash because corrupted code: unknown op 19 at 085F:012F (00871F flat), 9979 instructions executed
        "../dos-software-decoding/demo-256/fridge/fridge.com",     // black screen, needs font data
        "../dos-software-decoding/demo-256/gr17/gr17.com",         // black screen
        "../dos-software-decoding/demo-256/hungecek/hungecek.com", // black screen
        "../dos-software-decoding/demo-256/julia/julia.com",       // some gfx
        // "../dos-software-decoding/demo-256/lameland/lameland.com", // black screen, NOTE: ascii gfx
        "../dos-software-decoding/demo-256/lava/lava.com",         // black screen, ip gets incorrect: op 8F unknown reg = 1: at 085F:015B
        "../dos-software-decoding/demo-256/leaf/leaf.com",         // yellow screen
        // "../dos-software-decoding/demo-256/lets256/lets256.com",   // black screen, WinXP: show text, then exits
        "../dos-software-decoding/demo-256/luminous/luminous.com", // black screen
        "../dos-software-decoding/demo-256/lumps/lumps.com",       // black screen
        "../dos-software-decoding/demo-256/mbl/mbl.com",           // black screen
        // "../dos-software-decoding/demo-256/miracle/miracle.com", // crash: ip corrupted
        "../dos-software-decoding/demo-256/nicefire/nicefire.com", // black screen
        "../dos-software-decoding/demo-256/optimize/optimize.com", // some gfx
        "../dos-software-decoding/demo-256/pack/pack.com",         // black screen
        "../dos-software-decoding/demo-256/phong/phong.com",       // black screen
        "../dos-software-decoding/demo-256/pikku/pikku.com",       // black screen
        "../dos-software-decoding/demo-256/pixelize/pixelize.com", // some gfx
        "../dos-software-decoding/demo-256/plasma/plasma.com",     // black screen
        "../dos-software-decoding/demo-256/plasmalr/plasmalr.com", // black screen
        "../dos-software-decoding/demo-256/plasmexp/plasmexp.com", // some gfx - looks good ?
        "../dos-software-decoding/demo-256/platinum/platinum.com", // black screen
        "../dos-software-decoding/demo-256/proto256/proto256.com", // black screen
        "../dos-software-decoding/demo-256/riddle/riddle.com",     // black screen
        "../dos-software-decoding/demo-256/ripped/ripped.com",     // black screen
        "../dos-software-decoding/demo-256/saverave/saverave.com", // black screen
        // "../dos-software-decoding/demo-256/sierpins/sierpins.com", // crash, ip wrap
        "../dos-software-decoding/demo-256/snow/snow.com",         // black screen
        // "../dos-software-decoding/demo-256/specifi/specifi.com", // crash, ip wrap
        "../dos-software-decoding/demo-256/spline/spline.com",     // black screen
        "../dos-software-decoding/demo-256/sqwerz3/sqwerz3.com",   // some gfx
        "../dos-software-decoding/demo-256/static/static.com",     // black screen
        "../dos-software-decoding/demo-256/twisted/twisted.com",   // black screen, ip & code gets corrupted: op C6 unknown reg = 2 at 085F:0120 (008710 flat), 15830 instructions executed
        "../dos-software-decoding/demo-256/water/water.com",       // black screen
        // "../dos-software-decoding/demo-256/wd95/wd95.com",      // crash, ip wrap
        "../dos-software-decoding/demo-256/wetwet/wetwet.com",     // black screen
        "../dos-software-decoding/demo-256/x/x.com",               // black screen, ip gets corrupted: disasm: unknown op C2 at 085F:0165
        "../dos-software-decoding/demo-256/xwater/xwater.com",     // black screen
        "../dos-software-decoding/demo-256/zork/zork.com",         // black screen
        
    ];

    while let Some(bin) = test_bins.pop() {
        println!("demo_256: {}", bin);

        let mut cpu = CPU::new();
        let code = tools::read_binary(bin);
        cpu.load_com(&code);

        cpu.execute_n_instructions(100_000);
        let path = Path::new(bin);

        let stem = path.file_stem().unwrap_or(OsStr::new(""));
        let mut filename = OsString::new();
        filename.push("tests/render/demo/256_");
        filename.push(stem.to_os_string());
        filename.push("_100k.png");

        cpu.gpu.test_render_frame(&cpu.memory.memory, filename.to_str().unwrap());
        // XXX cpu.test_expect_memory_md5(x)
    }
}
