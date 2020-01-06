#[macro_use]
extern crate clap;
use clap::{Arg, App};

extern crate dustbox;
extern crate dustbox_fuzzer;

extern crate rand;
extern crate rand_xorshift;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use dustbox::cpu::Op;
use dustbox_fuzzer::fuzzer::{fuzz_ops, FuzzConfig, CodeRunner};

fn main() {
    let matches = App::new("dustbox-fuzzer")
        .version("0.1")
        .arg(Arg::with_name("mutations")
            .help("Number of mutations per instruction")
            .takes_value(true)
            .long("mutations"))
        .arg(Arg::with_name("ip")
            .help("Remote IP for supersafe client")
            .takes_value(true)
            .long("ip"))
        .arg(Arg::with_name("seed")
            .help("Specify PRNG seed for reproducibility")
            .takes_value(true)
            .long("seed"))
        .get_matches();

    let ops_to_fuzz = vec!(
        Op::Cmp8,

        // ENCODING NOT IMPLEMENTED:
        //Op::Cmpsw,

        // ERROR - regs differ vs dosbox, regs match vs winxp! - overflow flag is wrong in both:
        // Op::Shld, Op::Shrd,

        /*
        // UNSURE: overflow is identical to bochs and dosbox, but differs in WinXP vm:
        Op::Rcl8, Op::Rcr8, Op::Ror8, Op::Shl8, Op::Rol8,

        // SEEMS ALL OK:
        Op::Shr8, Op::Sar8, // OK !
        Op::Div8, Op::Div16, Op::Idiv8, Op::Idiv16, // seems correct. NOTE that winxp crashes with "Divide overflow" on some input
        Op::Bt, Op::Bsf,
        Op::Aaa, Op::Aad, Op::Aam, Op::Aas, Op::Daa, Op::Das,
        
        Op::Cmp8, Op::Cmp16,
        Op::And8, Op::And16,
        Op::Xor8, Op::Xor16,
        Op::Or8, Op::Or16,
        Op::Add8, Op::Add16, Op::Adc8, Op::Adc16,
        Op::Sub8, Op::Sub16, Op::Sbb8, Op::Sbb16,
        Op::Test8, Op::Test16,
        Op::Not8, Op::Not16,
        Op::Neg8, Op::Neg16,
        Op::Xchg8, Op::Xchg16,
        Op::Mul8, Op::Mul16, Op::Imul8, Op::Imul16,
        Op::Lahf, Op::Sahf, Op::Salc,
        Op::Nop,
        Op::Clc, Op::Cld, Op::Cli, Op::Cmc, Op::Stc, Op::Std, Op::Sti,
        Op::Cbw, Op::Cwd16,
        Op::Lea16,
        Op::Inc8, Op::Inc16, Op::Inc32,
        Op::Dec8, Op::Dec16, Op::Dec32,
        */
    );

    let cfg = FuzzConfig{
        mutations_per_op: value_t!(matches, "mutations", usize).unwrap_or(50),
        remote_ip: matches.value_of("ip").unwrap_or("127.0.0.1").to_string(),
    };

    let runner = CodeRunner::SuperSafe;
    //let runner = CodeRunner::DosboxX;

    // seed prng if argument was given
    let mut rng: XorShiftRng;
    let seed_value: u64;
    if let Some(_) = matches.value_of("seed") {
        seed_value = value_t!(matches, "seed", u64).unwrap();
    } else {
        let mut tmp = XorShiftRng::from_entropy();
        seed_value = tmp.gen();
    }

    rng = XorShiftRng::seed_from_u64(seed_value);
    println!("rng seed = {}", seed_value);

    fuzz_ops(&runner, ops_to_fuzz, &cfg, &mut rng);
}
