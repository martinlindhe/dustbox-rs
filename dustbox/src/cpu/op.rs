use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// ASCII Adjust After Addition
    Aaa,

    /// ASCII Adjust AX Before Division
    Aad,

    /// ASCII Adjust AX After Multiply
    Aam,

    /// ASCII Adjust AL After Subtraction
    Aas,

    Adc8, Adc16, Adc32,
    Add8, Add16, Add32,
    And8, And16, And32,

    /// Adjust RPL Field of Segment Selector
    Arpl,

    /// Check Array Index Against Bounds
    Bound,

    /// Bit Scan Forward
    Bsf,

    /// Bit Test
    Bt,

    Bts,
    CallNear, CallFar,

    /// Convert Byte to Word
    Cbw,

    /// Clear Carry Flag
    Clc,

    /// Clear Direction Flag
    Cld,

    /// Clear Interrupt Flag
    Cli,

    /// Complement Carry Flag
    Cmc,

    Cmp8, Cmp16, Cmp32,

    /// Compare String Operands
    Cmpsb,

    /// 16 bit opsize mode, 16 bit address size
    Cmpsw16,

    /// 16 bit opsize mode, 32 bit address size
    Cmpsw32,

    /// 32 bit opsize mode, 16 bit address size
    Cmpsd16,

    /// 32 bit opsize mode, 32 bit address size
    Cmpsd32,

    /// Convert Word to Doubleword
    Cwd16, Cwde32,

    /// Decimal Adjust AL after Addition
    Daa,

    /// Decimal Adjust AL after Subtraction
    Das,

    Dec8, Dec16, Dec32,
    Div8, Div16,

    /// Unsigned divide EDX:EAX by r/m32, with result stored in EAX ← Quotient, EDX ← Remainder
    Div32,

    Enter,
    Hlt,

    Idiv8, Idiv16, Idiv32,
    Imul8, Imul16, Imul32,

    /// Input from Port
    In8, In16,

    Inc8, Inc16, Inc32,

    /// Input from Port to String
    Insb, Insw,

    Int,
    Into,
    Iret,

    /// Jump if above (CF=0 and ZF=0).    (alias: jnbe)
    Ja,

    /// Jump if carry (CF=1).    (alias: jb, jnae)
    Jc,

    /// Jump if CX register is 0.
    Jcxz,

    /// Jump if ECX register is 0.
    Jecxz,

    /// Jump if greater (ZF=0 and SF=OF).    (alias: jnle)
    Jg,

    /// Jump if less (SF ≠ OF).    (alias: jnge)
    Jl,

    JmpShort, JmpNear, JmpFar,

    /// Jump if not above (CF=1 or ZF=1).    (alias: jbe)
    Jna,

    /// Jump if not carry (CF=0).    (alias: jae, jnb)
    Jnc,

    /// Jump if not greater (ZF=1 or SF ≠ OF).    (alias: jle)
    Jng,

    /// Jump if not less (SF=OF).    (alias: jge)
    Jnl,

    /// Jump if not overflow (OF=0).
    Jno,

    /// Jump if not sign (SF=0).
    Jns,

    /// Jump if not zero (ZF=0).    (alias: jne)
    Jnz,

    /// Jump if overflow (OF=1).
    Jo,

    /// Jump short if parity even (PF=1)
    Jpe,

    /// Jump short if parity odd (PF=0).
    Jpo,

    /// Jump if sign (SF=1).
    Js,

    /// Jump if zero (ZF ← 1).    (alias: je)
    Jz,

    /// Load Status Flags into AH Register
    Lahf,

    /// Load Access Rights Byte
    Lar16,

    /// Load DS:r16 with far pointer from memory.
    Lds,

    /// Load Effective Address
    /// Computes the effective address of the source operand and stores it in the destination operand.
    Lea16,

    Leave,

    /// Load ES:r16 with far pointer from memory.
    Les,

    /// Load byte at address DS:(E)SI into AL.
    Lodsb,

    /// Load word at address DS:(E)SI into AX.
    Lodsw,

    /// Load dword at address DS:(E)SI into EAX.
    Lodsd,

    /// Decrement count (cx or ecx); jump short if count ≠ 0.
    Loop16,
    Loop32,

    /// Decrement count (cx or ecx); jump short if count ≠ 0 and ZF = 1.
    Loop16e,
    Loop32e,

    /// Decrement count (cx or ecx); jump short if count ≠ 0 and ZF = 0.
    Loop16ne,
    Loop32ne,

    Mov8, Mov16, Mov32,
    Movsb, Movsw, Movsd,

    /// Move with Sign-Extension
    Movsx16, Movsx32,

    /// Move with Zero-Extend
    Movzx16, Movzx32,

    Mul8, Mul16, Mul32,
    Neg8, Neg16, Neg32,
    Nop,
    Not8, Not16, Not32,
    Or8, Or16, Or32,
    Out8, Out16,
    Outsb, Outsw,
    Pop16, Pop32,

    /// Pop DI, SI, BP, BX, DX, CX, and AX.
    Popa16,

    /// Pop EDI, ESI, EBP, EBX, EDX, ECX, and EAX.
    Popad32,

    /// Pop top of stack into lower 16 bits of EFLAGS.
    Popf,

    Push16, Push32,

    /// Push AX, CX, DX, BX, original SP, BP, SI, and DI.
    Pusha16,

    /// Push EAX, ECX, EDX, EBX, original ESP, EBP, ESI, and EDI.
    Pushad32,

    /// push 16 bit FLAGS register onto stack
    Pushf,

    Rcl8, Rcl16, Rcl32,

    /// Rotate 9 bits (CF, r/m8) right
    Rcr8,

    /// Rotate 17 bits (CF, r/m16) right
    Rcr16,

    /// Rotate 33 bits (CF, r/m32) right
    Rcr32,

    Retn, Retf, RetImm16,

    Rol8, Rol16, Rol32,
    Ror8, Ror16, Ror32,

    /// Store AH into Flags
    Sahf,

    /// "salc", or "setalc" is a undocumented Intel instruction
    /// http://ref.x86asm.net/coder32.html#gen_note_u_SALC_D6
    /// http://www.rcollins.org/secrets/opcodes/SALC.html
    /// used by dos-software-decoding/demo-256/luminous/luminous.com
    Salc,

    Sar8, Sar16, Sar32,

    /// Integer Subtraction with Borrow
    Sbb8, Sbb16, Sbb32,

    Scasb, Scasw, Scasd,

    /// setc: Set byte if carry (CF=1).
    /// alias setb: Set byte if below (CF=1).
    Setc,

    /// setg: Set byte if greater (ZF=0 and SF=OF).
    /// alias setnle: Set byte if not less or equal (ZF=0 and SF=OF).
    Setg,

    /// setnz: Set byte if not zero (ZF=0).
    /// alias setne: Set byte if not equal (ZF=0).
    Setnz,

    /// Multiply `dst` by 2, `src` times (alias sal)
    Shl8,
    /// Multiply `dst` by 2, `src` times (alias sal)
    Shl16,
    /// Multiply `dst` by 2, `src` times (alias sal)
    Shl32,

    /// Double Precision Shift Left
    Shld,

    Shr8, Shr16, Shr32,

    /// Double Precision Shift Right
    Shrd,

    Sldt,

    // Set Carry Flag
    Stc,

    /// Set Direction Flag
    Std,

    /// Set Interrupt Flag
    Sti,

    Stosb, Stosw, Stosd,
    Sub8, Sub16, Sub32,
    Test8, Test16, Test32,

    /// Exchange Register/Memory with Register
    Xchg8, Xchg16, Xchg32,

    Xlatb,

    Xor8, Xor16, Xor32,

    /// (FPU) Absolute Value
    Fabs,

    /// (FPU) Add
    Fadd, Faddp,

    /// (FPU) Change Sign
    Fchs,

    /// (FPU) Compare Floating Point Values
    Fcom, Fcomp,

    /// (FPU) Cosine
    Fcos,

    /// (FPU) Divide
    Fdiv, Fdivp, Fidiv,

    /// (FPU) Reverse Divide
    Fdivr,

    /// (FPU) Free Floating-Point Register
    Ffree,

    /// (FPU) Compare Integer
    Ficom, Ficomp,

    /// (FPU) Load Integer
    Fild,

    /// (FPU) Initialize Floating-Point Unit
    Finit,

    /// (FPU) Store Integer
    Fist, Fistp,

    /// (FPU) Store Integer with Truncation
    Fisttp,

    /// (FPU) Load Floating Point Value
    Fld,

    /// (FPU) Load Constant +1.0
    Fld1,

    /// (FPU) Load Constant log₂10
    Fldl2t,

    /// (FPU) Load Constant log₂e
    Fldl2e,

    /// (FPU) Load Constant +0.0
    Fldz,

    /// (FPU) Load Constant π
    Fldpi,

    /// (FPU) Load x87 FPU Control Word
    Fldcw,

    /// (FPU) Multiply
    Fmul, Fimul,

    /// (FPU) Partial Arctangent
    Fpatan,

    /// (FPU) Round to Integer
    Frndint,

    /// (FPU) Sine
    Fsin,

    /// (FPU) Sine and Cosine
    Fsincos,

    /// (FPU)
    Fsqrt,

    /// (FPU) Store Floating Point Value
    Fst, Fstp,

    /// (FPU) Store x87 FPU Status Word
    Fstsw,

    /// (FPU) Store x87 FPU Control Word
    Fnstcw,

    /// (FPU) Subtract
    Fsub, Fsubp,

    /// (FPU) Reverse Subtract
    Fsubr, Fsubrp,

    /// (FPU) Test
    Ftst,

    /// (FPU) Wait
    Fwait,

    /// (FPU) Exchange Register Contents
    Fxch,

    /// Initial state
    Uninitialized,

    /// Invalid encoding. XXX also used for unhandled encodings atm
    Invalid(Vec<u8>, Invalid),
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Invalid(bytes, _) => {
                let mut x = Vec::new();
                for b in bytes {
                    x.push(format!("{:02X}", b));
                }
                write!(f, "INVALID {}", x.join(", "))
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl Op {
    pub fn is_valid(&self) -> bool {
        match *self {
            Op::Uninitialized | Op::Invalid(_, _) => false,
            _ => true,
        }
    }
}

/// the class of instruction decode error that occured
#[derive(Clone, Debug, PartialEq)]
pub enum Invalid {
    /// a reg value was unhandled / invalid
    Reg(u8),

    /// unimplemented / invalid CPU instr
    Op,

    /// unimplemented / invalid FPU instr
    FPUOp,
}
