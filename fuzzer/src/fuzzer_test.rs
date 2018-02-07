use dustbox::cpu::instruction::Instruction;
use dustbox::cpu::op::Op;
use dustbox::cpu::parameter::Parameter;
use dustbox::cpu::register::{R8, R16};

use fuzzer::{fuzz, AffectedFlags};

#[test] #[ignore] // expensive test
fn fuzz_instruction() {
    let affected_registers = vec!("ax");
    let affected_flags_mask = AffectedFlags{c:1, o:1, s:1, z:1, p:1, a:1}.mask();

    // XXX verified with winXP: Shr8, Rol8
    // XXX differs from winXP: Shl8 (OF), Sar8 (wrong result some times)
    //          - Ror8 (CF)

    for i in 1..65535 as usize {
        let n1 = ((i + 1) & 0xFF) ^ 0xFF;
        let n2 = i & 0xFF;
        let ops = vec!(
            // clear flags
            Instruction::new1(Op::Push16, Parameter::Imm16(0)),
            Instruction::new(Op::Popf),
            // clear ax,bx,cx,dx
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::CX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::DX), Parameter::Imm16(0)),
            // mutate parameters
            Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AH), Parameter::Imm8(n1 as u8)),
            Instruction::new2(Op::Ror8, Parameter::Reg8(R8::AH), Parameter::Imm8(n2 as u8)),
        );

        fuzz(&ops, &affected_registers, affected_flags_mask);
    }
}
