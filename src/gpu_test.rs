// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use cpu::CPU;
//use instruction::seg_offs_as_flat;
//use register::{AX, CS};
use tools;

/*
#[test]
fn demo_256_165plasm() {
    // STATUS: demo corrupts program code
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/165plasm/debug/165plasd.com");
    cpu.load_com(&code);

    // debug: run until ip = 0133
    let cs = cpu.sreg16[CS];
    cpu.add_breakpoint(seg_offs_as_flat(cs, 0x0133));
    cpu.execute_n_instructions(1000);
    println!("{}", ".");
    println!("{}", ".");
    println!("{}", ".");
    cpu.clear_breakpoints();

    cpu.execute_n_instructions(400); // XXX hits corrupted op: "unknown op C8 at 085F:0164 (008754 flat), 1318 instructions executed"

// XXX write gfx frame as png
    //assert_eq!(0xFFFD, cpu.r16[AX].val);
}
*/

#[test]
fn demo_256_244b() {
    // STATUS: pixels are rendered, but the effect is not right (25 oct, 2017)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/244b/244b.com");
    cpu.load_com(&code);

    cpu.execute_n_instructions(1000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_244b_1k.png");
    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(49000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_244b_50k.png");
}

#[test]
fn demo_256_alpc() {
    // STATUS: pixels are rendered, but the effect is not right (25 oct, 2017)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/alpc/alpc.com");
    cpu.load_com(&code);

    cpu.execute_n_instructions(50_000);
    // XXX cpu.test_expect_memory_md5(x)
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_alpc_50k.png");
}

#[test]
fn demo_256_beziesux() {
    // STATUS: pixels are rendered, but the effect is not right (26 oct, 2017)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/beziesux/beziesux.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(500_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_beziesux_500k.png");
}

#[test]
fn demo_256_blah() {
    // STATUS: black screen, crash after 50k instr or so (executing 00:s)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/blah/blah.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(30_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_blah_30k.png");
}

#[test]
fn demo_256_bob() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/bob/bob.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(30_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_bob_30k.png");
}

#[test]
fn demo_256_bumpgeci() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/bumpgeci/bumpgeci.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(70_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_bumpgeci_70k.png");
}

#[test]
fn demo_256_chaos() {
    // STATUS: black screen, needs font data accessible, i think
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/chaos/chaos.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_chaos_50k.png");
}

/*
#[test]
fn demo_256_conf() {
    // STATUS: waits for ENTER press
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/conf/conf.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    // XXX FIXME: inject enter key press to progress demo

    cpu.execute_n_instructions(20_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_conf_20k.png");
}
*/

#[test]
fn demo_256_ectotrax() {
    // STATUS: black screen, needs font data accessible, i think
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/ectotrax/ectotrax.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_ectotrax_50k.png");
}

#[test]
fn demo_256_enchante() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/enchante/enchante.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_enchante_50k.png");
}

#[test]
fn demo_256_fire() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fire/fire.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fire_50k.png");
}

#[test]
fn demo_256_fire2() {
    // STATUS: black screen, needs font data
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fire2/fire2.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fire2_50k.png");
}

#[test]
fn demo_256_fire3d() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fire3d/fire3d.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fire3d_50k.png");
}

#[test]
fn demo_256_fire17() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fire17/fire17.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fire17_50k.png");
}

#[test]
fn demo_256_flame2() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/flame2/flame2.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_flame2_50k.png");
}

#[test]
fn demo_256_fracscrl() {
    // STATUS: red screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fracscrl/fracscrl.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fracscrl_50k.png");
}

#[test]
fn demo_256_fractal() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fractal/fractal.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fractal_50k.png");
}

#[test]
fn demo_256_fridge() {
    // STATUS: black screen, needs font data
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/fridge/fridge.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_fridge_50k.png");
}

#[test]
fn demo_256_gr17() {
    // STATUS: black screen
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/gr17/gr17.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(50_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_gr17_50k.png");
}
