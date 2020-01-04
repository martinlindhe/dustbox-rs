extern crate rand;
extern crate rand_xorshift;

extern crate dustbox;
extern crate dustbox_fuzzer;

use std::io::{self, Write};

use rand::prelude::*;
use rand_xorshift::XorShiftRng;

use dustbox::cpu::{Instruction, Op, Parameter, Segment, R, AMode, Encoder, instructions_to_str};
use dustbox_fuzzer::fuzzer::{fuzz, VmRunner, AffectedFlags};

fn main() {
    let affected_registers = vec!("ax", "dx");

    let ops_to_fuzz = vec!(
        //Op::Cmpsw,
        //Op::Shrd,
        Op::Shld, //  overflow differs from winxp. may be wrong in both
        Op::Aaa, Op::Aas, Op::Aad, Op::Daa, Op::Das,
        Op::Aam, // Aam - P Z S flags differ from winxp & dosbox-x
        //Op::Shl8, Op::Rol8, Op::Ror8, Op::Rcr8, // OVERFLOW flag differ from winxp
        //Op::Rcl8, // register values dont match with dosbox-x, but with bochs & winxp
        Op::Shr8, Op::Sar8,
        Op::Cmp8, Op::And8, Op::Xor8, Op::Or8, Op::Add8, Op::Adc8, Op::Sub8, Op::Sbb8,
        Op::Test8, Op::Not8, Op::Mul8, Op::Imul8, Op::Xchg8,
        Op::Div8, Op::Idiv8, // hard to fuzz due to input that triggers DIV0 exception
        Op::Neg8, // mov ah,0; neg ah =   OVERFLOW flag differs vs winxp
        Op::Lahf,
        Op::Sahf, Op::Salc,
        Op::Nop,
        Op::Clc, Op::Cld, Op::Cli, Op::Cmc, Op::Stc, Op::Std, Op::Sti,
        Op::Cbw, Op::Cwd16,
        Op::Lea16,
        Op::Inc8, Op::Inc16, Op::Inc32,
        Op::Dec8, Op::Dec16, Op::Dec32,
    );

    let iterations_per_op = 50;
    let mut rng = XorShiftRng::from_entropy();
    for op in ops_to_fuzz {
        println!("------");
        println!("fuzzing {} forms of {:?} ...", iterations_per_op, op);
        for _ in 0..iterations_per_op {
            let runner = VmRunner::VmHttp;
            let affected_flags_mask = AffectedFlags::for_op(&op);

            let mut ops = vec!(
                // clear eax,edx
                Instruction::new2(Op::Mov32, Parameter::Reg32(R::EAX), Parameter::Imm32(0)),
                Instruction::new2(Op::Mov32, Parameter::Reg32(R::EDX), Parameter::Imm32(0)),

                // clear flags
                Instruction::new1(Op::Push16, Parameter::Imm16(0)),
                Instruction::new(Op::Popf),
            );

            // mutate parameters
            let snippet = get_mutator_snippet(&op, &mut rng);
            ops.extend(snippet.to_vec());

            io::stdout().flush().expect("Could not flush stdout");
            let encoder = Encoder::new();
            let data = match encoder.encode_vec(&ops) {
                Ok(data) => data,
                Err(why) => panic!("{}", why),
            };

            if !fuzz(&runner, &data, ops.len(), &affected_registers, affected_flags_mask) {
                println!("fuzz failed with this input:");
                println!("{}", instructions_to_str(&snippet));
                println!("------");
            }
        }
    }
}
// returns a snippet used to mutate state for op
fn get_mutator_snippet(op: &Op, rng: &mut XorShiftRng) -> Vec<Instruction> {
    match *op {
        Op::Cmpsw => { vec!(
            // compare word at address DS:(E)SI with byte at address ES:(E)DI;
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::SI), Parameter::Imm16(0x3030)),
            Instruction::new2(Op::Mov16, Parameter::Ptr16Amode(Segment::Default, AMode::SI), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::DI), Parameter::Imm16(0x3040)),
            Instruction::new2(Op::Mov16, Parameter::Ptr16Amode(Segment::Default, AMode::DI), Parameter::Imm16(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Shld | Op::Shrd => { vec!(
            // mutate ax, dx, imm8
            // shld ax, dx, imm8
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::DX), Parameter::Imm16(rng.gen())),
            Instruction::new3(op.clone(), Parameter::Reg16(R::AX), Parameter::Reg16(R::DX), Parameter::Imm8(rng.gen())),
        )}
        Op::Shl8 | Op::Shr8 | Op::Sar8 | Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
        Op::Cmp8 | Op::And8 | Op::Xor8 | Op::Or8 | Op::Add8 | Op::Adc8 | Op::Sub8 | Op::Sbb8 | Op::Test8 => { vec!(
            // test r/m8, imm8
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
        )}
        Op::Mul8 | Op::Imul8 => { vec!(
            // mul r/m8      ax = al * r/m
            // imul r/m8     ax = al * r/m
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::DL)),
        )}
        Op::Div8 | Op::Idiv8 => { vec!(
            // divide AX by r/m8, store in AL, AH
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::DL)),
        )}
        Op::Xchg8 => { vec!(
            // xchg r/m8, r8
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::DL), Parameter::Imm8(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg8(R::DL), Parameter::Reg8(R::BL)),
        )}
        Op::Lahf | Op::Salc | Op::Clc | Op::Cld | Op::Cli | Op::Cmc | Op::Stc | Op::Std | Op::Sti => { vec!(
            // mutate flags
            Instruction::new1(Op::Push16, Parameter::Imm16(rng.gen())),
            Instruction::new(Op::Popf),
            Instruction::new(op.clone()),
        )}
        Op::Aas | Op::Aaa | Op::Daa | Op::Das | Op::Cbw => { vec!(
            // mutate al: no args
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Not8 | Op::Neg8 | Op::Inc8 | Op::Dec8 => { vec!(
            // mutate al: r/m8
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AL), Parameter::Imm8(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::AL)),
        )}
        Op::Sahf => { vec!(
            // mutate ah: no args
            Instruction::new2(Op::Mov8, Parameter::Reg8(R::AH), Parameter::Imm8(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Cwd16 => { vec!(
            // mutate ax: no args
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new(op.clone()),
        )}
        Op::Inc16 | Op::Dec16 => { vec!(
            // mutate ax: r/m16
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg8(R::AX)),
        )}
        Op::Aad | Op::Aam => { vec!(
            // mutate ax: imm8
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::AX), Parameter::Imm16(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Imm8(rng.gen())),
        )}
        Op::Lea16 => { vec!(
            // lea r16, m
            Instruction::new2(Op::Mov16, Parameter::Reg16(R::BX), Parameter::Imm16(rng.gen())),
            Instruction::new2(op.clone(), Parameter::Reg16(R::AX), Parameter::Ptr16Amode(Segment::Default, AMode::BX)),
        )}
        Op::Inc32 | Op::Dec32 => { vec!(
            // mutate eax: r/m16
            Instruction::new2(Op::Mov32, Parameter::Reg32(R::EAX), Parameter::Imm32(rng.gen())),
            Instruction::new1(op.clone(), Parameter::Reg32(R::EAX)),
        )}
        Op::Nop => vec!(Instruction::new(op.clone())),
        _ => panic!("get_mutator_snippet: unhandled op {:?}", op),
    }
}
