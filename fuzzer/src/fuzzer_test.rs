use std::io::{self, Write};

use rand::{Rng, XorShiftRng};

use dustbox::cpu::instruction::Instruction;
use dustbox::cpu::op::Op;
use dustbox::cpu::parameter::Parameter;
use dustbox::cpu::segment::Segment;
use dustbox::cpu::register::{R8, R16, AMode};
use dustbox::cpu::decoder::instructions_to_str;
use dustbox::cpu::encoder::Encoder;

use fuzzer::{fuzz, VmRunner, AffectedFlags};

#[test] #[ignore] // expensive test
fn fuzz_instruction() {
    let affected_registers = vec!("ax", "dx");

    let ops_to_fuzz = vec!(
        Op::Aaa, Op::Aas, Op::Aad, Op::Daa, Op::Das,
        Op::Aam, // Aam - P Z S flags differ from winxp & dosbox-x
        Op::Shld, //  overflow differs from winxp. may be wrong in both
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
        Op::Cbw, Op::Cwd,
        Op::Lea16,
    );

    let mut rng = XorShiftRng::new_unseeded();
    for op in ops_to_fuzz {
        println!("------");
        println!("fuzzing {:?} ...", op);
        for it in 0..3 {
            let runner = VmRunner::VmHttp;
            let affected_flags_mask = AffectedFlags::for_op(&op);

            let mut ops = vec!(
                // clear ax,dx
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(0)),
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::DX), Parameter::Imm16(0)),

                // clear flags
                Instruction::new1(Op::Push16, Parameter::Imm16(0)),
                Instruction::new(Op::Popf),
            );

            // mutate parameters
            let snippet = get_mutator_snippet(&op, &mut rng);
            if it == 0 {
                println!("{}", instructions_to_str(&snippet));
            }
            ops.extend(snippet.to_vec());

            io::stdout().flush().ok().expect("Could not flush stdout");
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
        Op::Shld => {
            // shld r/m16, r16, imm8
            vec!(
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(rng.gen())),
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::DX), Parameter::Imm16(rng.gen())),
                Instruction::new3(op.clone(), Parameter::Reg16(R16::AX), Parameter::Reg16(R16::DX), Parameter::Imm8(rng.gen())),
            )
        }
        Op::Shl8 | Op::Shr8 | Op::Sar8 | Op::Rol8 | Op::Ror8 | Op::Rcl8 | Op::Rcr8 |
        Op::Cmp8 | Op::And8 | Op::Xor8 | Op::Or8 | Op::Add8 | Op::Adc8 | Op::Sub8 | Op::Sbb8 | Op::Test8 => {
            // test r/m8, imm8
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
                Instruction::new2(op.clone(), Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
            )
        }
        Op::Mul8 | Op::Imul8 => {
            // mul r/m8      ax = al * r/m
            // imul r/m8     ax = al * r/m
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::DL), Parameter::Imm8(rng.gen())),
                Instruction::new1(op.clone(), Parameter::Reg8(R8::DL)),
            )
        }
        Op::Div8 | Op::Idiv8 => {
            // divide AX by r/m8, store in AL, AH
            vec!(
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(rng.gen())),
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::DL), Parameter::Imm8(rng.gen())),
                Instruction::new1(op.clone(), Parameter::Reg8(R8::DL)),
            )
        }
        Op::Xchg8 => {
            // xchg r/m8, r8
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::DL), Parameter::Imm8(rng.gen())),
                Instruction::new2(op.clone(), Parameter::Reg8(R8::DL), Parameter::Reg8(R8::BL)),
            )
        }
        Op::Lahf | Op::Salc | Op::Clc | Op::Cld | Op::Cli | Op::Cmc | Op::Stc | Op::Std | Op::Sti => {
            // mutate flags
            vec!(
                Instruction::new1(Op::Push16, Parameter::Imm16(rng.gen())),
                Instruction::new(Op::Popf),
                Instruction::new(op.clone()),
            )
        }
        Op::Aas | Op::Aaa | Op::Daa | Op::Das | Op::Cbw => {
            // mutate al: no args
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
                Instruction::new(op.clone()),
            )
        }
        Op::Not8 | Op::Neg8 => {
            // mutate al: r/m8
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AL), Parameter::Imm8(rng.gen())),
                Instruction::new1(op.clone(), Parameter::Reg8(R8::AL)),
            )
        }
        Op::Sahf => {
            // mutate ah: no args
            vec!(
                Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AH), Parameter::Imm8(rng.gen())),
                Instruction::new(op.clone()),
            )
        }
        Op::Cwd => {
            // mutate ax: no args
            vec!(
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(rng.gen())),
                Instruction::new(op.clone()),
            )
        }
        Op::Aad | Op::Aam => {
            // mutate ax: imm8
            vec!(
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(rng.gen())),
                Instruction::new1(op.clone(), Parameter::Imm8(rng.gen())),
            )
        }
        Op::Lea16 => {
            // lea r16, m
            vec!(
                Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(rng.gen())),
                Instruction::new2(op.clone(), Parameter::Reg16(R16::AX), Parameter::Ptr16Amode(Segment::Default, AMode::BX)),
            )
        }
        Op::Nop => vec!(Instruction::new(op.clone())),
        _ => panic!("get_mutator_snippet: unhandled op {:?}", op),
    }
}
