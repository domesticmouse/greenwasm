#![allow(non_snake_case, unused_imports)]

// TODO: Open PR in nom for where applicable
macro_rules! verify_ref (
  // Internal parser, do not use directly
  (__impl $i:expr, $submac:ident!( $($args:tt)* ), $submac2:ident!( $($args2:tt)* )) => (
    {
      use nom::lib::std::result::Result::*;
      use nom::{Err,ErrorKind};

      let i_ = $i.clone();
      match $submac!(i_, $($args)*) {
        Err(e)     => Err(e),
        Ok((i, o)) => if $submac2!(&o, $($args2)*) {
          Ok((i, o))
        } else {
          Err(Err::Error(error_position!($i, ErrorKind::Verify)))
        }
      }
    }
  );
  ($i:expr, $submac:ident!( $($args:tt)* ), $g:expr) => (
    verify_ref!(__impl $i, $submac!($($args)*), call!($g));
  );
  ($i:expr, $submac:ident!( $($args:tt)* ), $submac2:ident!( $($args2:tt)* )) => (
    verify_ref!(__impl $i, $submac!($($args)*), $submac2!($($args2)*));
  );
  ($i:expr, $f:expr, $g:expr) => (
    verify_ref!(__impl $i, call!($f), call!($g));
  );
  ($i:expr, $f:expr, $submac:ident!( $($args:tt)* )) => (
    verify_ref!(__impl $i, call!($f), $submac!($($args)*));
  );
);
macro_rules! btag {
    ($i:expr, $b:expr) => (tag!($i, &[$b][..]))
}
macro_rules! btagmap {
    ($i:expr, $b:expr, $r:expr) => (map!($i, btag!($b), |_| $r))
}

use nom::IResult;
use nom::types::CompleteByteSlice;

type Inp<'a> = CompleteByteSlice<'a>;

// 5.1.3. Vectors
fn parse_vec<'a, F, B>(input: Inp<'a>, mut parse_b: F) -> IResult<Inp<'a>, Vec<B>>
    where F: FnMut(Inp<'a>) -> IResult<Inp<'a>, B>
{
    do_parse!(input,
        n: apply!(parse_uN, 32)
        >> res: many_m_n!(n as usize, n as usize, parse_b)
        >> (res)
    )
}

// 5.2.1. Bytes
named!(parse_byte <Inp, u8>, map!(take!(1), |n| n[0]));

// 5.2.2. Integers
named_args!(parse_uN(N: u32) <Inp, u64>, alt!(
    do_parse!(
        n: verify!(parse_byte, |n| {
            // n < 2^7 ∧ n < 2^N
            let n = n as u128;
            let v27 = 1 << 7;
            let v2N = 1 << N;
            n < v27 && n < v2N
        })
        >> (n as u64)
    )
    | do_parse!(
        n: verify!(parse_byte, |n| {
            // n ≥ 2^7 ∧ N > 7
            let n = n as u128;
            let v27 = 1 << 7;
            n >= v27 && N > 7
        })
        >> m: apply!(parse_uN, N - 7)
        >> ((1 << 7) * m + ((n as u64) - (1 << 7)))
    )
));

named!(pub parse_u32 <Inp, u32>, map!(apply!(parse_uN, 32), |x| x as u32));
named!(pub parse_u64 <Inp, u64>, apply!(parse_uN, 64));

named_args!(parse_sN(N: u32) <Inp, i64>, alt!(
    do_parse!(
        n: verify!(parse_byte, |n| {
            // n < 2^6 ∧ n < 2^(N−1)
            let n = n as i128;
            let v26 = 1 << 6;
            let v2N1 = 1 << (N - 1);
            n < v26 && n < v2N1
        })
        >> (n as i64)
    )
    | do_parse!(
        n: verify!(parse_byte, |n| {
            // 2^6 ≤ n < 2^7 ∧ n ≥ 2^7 − 2^(N − 1)
            let n = n as i128;
            let v26 = 1 << 6;
            let v27 = 1 << 7;
            let v2N1 = 1 << (N - 1);
            v26 <= n && n < v27 && n >= (v27 - v2N1)
        })
        >> (n as i64 - (1 << 7))
    )
    | do_parse!(
        n: verify!(parse_byte, |n| {
            // n ≥ 2^7 ∧ N > 7
            let n = n as i128;
            let v27 = 1 << 7;
            n >= v27 && N > 7
        })
        >> m: apply!(parse_sN, N - 7)
        >> ((1 << 7) * m + ((n as i64) - (1 << 7)))
    )
));

named!(parse_s32 <Inp, i32>, map!(apply!(parse_sN, 32), |x| x as i32));
named!(parse_s64 <Inp, i64>, apply!(parse_sN, 64));

named!(parse_i32 <Inp, u32>, map!(parse_s32, |x| x as u32));
named!(parse_i64 <Inp, u64>, map!(parse_s64, |x| x as u64));

// 5.2.3. Floating-Point
named!(parse_f32 <Inp, f32>, do_parse!(
    bs: map!(take!(4), |s| { let mut b = [0; 4]; b.copy_from_slice(&**s); b })
    >> (f32::from_bits(u32::from_le(u32::from_bytes(bs))))
));
named!(parse_f64 <Inp, f64>, do_parse!(
    bs: map!(take!(8), |s| { let mut b = [0; 8]; b.copy_from_slice(&**s); b })
    >> (f64::from_bits(u64::from_le(u64::from_bytes(bs))))
));

// 5.2.4. Names
named!(parse_name <Inp, String>, do_parse!(
    bs: map!(
        verify_ref!(
            map!(
                call!(parse_vec, parse_byte),
                String::from_utf8
            ),
            |res: &Result<_, _>| res.is_ok()
        ),
        |res| res.unwrap()
    )
    >> (bs)
));

// 5.3.1 Value Types
use structure::types::ValType;
named!(parse_valtype <Inp, ValType>, alt!(
    btagmap!(0x7f, ValType::I32)
    | btagmap!(0x7e, ValType::I64)
    | btagmap!(0x7d, ValType::F32)
    | btagmap!(0x7c, ValType::F64)
));

// 5.3.2 Result Types
use structure::types::ResultType;
named!(parse_blocktype <Inp, ResultType>, alt!(
    btagmap!(0x40, None)
    | map!(parse_valtype, |v| Some(v))
));

// 5.3.3 Function Types
use structure::types::FuncType;
named!(parse_functype <Inp, FuncType>, do_parse!(
    btag!(0x60)
    >> t1s: call!(parse_vec, parse_valtype)
    >> t2s: call!(parse_vec, parse_valtype)
    >> (FuncType {
        args: t1s,
        results: t2s,
    })
));

// 5.3.4 Limits
use structure::types::Limits;
named!(parse_limits <Inp, Limits>, alt!(
    do_parse!(
        btag!(0x00)
        >> n: parse_u32
        >> (Limits { min: n, max: None })
    )
    |do_parse!(
        btag!(0x01)
        >> n: parse_u32
        >> m: parse_u32
        >> (Limits { min: n, max: Some(m) })
    )
));

// 5.3.5 Memory Types
use structure::types::MemType;
named!(parse_memtype <Inp, MemType>, map!(parse_limits, |limits| MemType { limits }));

// 5.3.6. Table Types
use structure::types::TableType;
use structure::types::ElemType;
named!(parse_tabletype <Inp, TableType>, do_parse!(
    et: parse_elemtype
    >> lim: parse_limits
    >> (TableType{ limits: lim, elemtype: et })
));
named!(parse_elemtype <Inp, ElemType>, btagmap!(0x70, ElemType::AnyFunc));

// 5.3.7. Global Types
use structure::types::GlobalType;
use structure::types::Mut;
named!(parse_globaltype <Inp, GlobalType>, do_parse!(
    t: parse_valtype
    >> m: parse_mut
    >> (GlobalType{ mutability: m, valtype: t })
));
named!(parse_mut <Inp, Mut>, alt!(
    btagmap!(0x00, Mut::Const)
    | btagmap!(0x01, Mut::Var)
));

// 5.4. Instructions
macro_rules! ins {
    ($i:expr, $b:expr, $r:expr; $($t:tt)*) => (
        do_parse!($i, btag!($b) >> $($t)* >> ($r))
    );
    ($i:expr, $b:expr, $r:expr) => (
        do_parse!($i, btag!($b) >> ($r))
    )
}
use structure::instructions::Instr;
named!(parse_instr <Inp, Instr>, alt!(
    // 5.4.1. Control Instructions
    ins!(0x00, Instr::Unreachable)
    | ins!(0x01, Instr::Nop)
    | ins!(0x02, Instr::Block(rt, ins);
        rt: parse_blocktype
        >> ins: many0!(parse_instr)
        >> btag!(0x0b)
    )
    | ins!(0x03, Instr::Loop(rt, ins);
        rt: parse_blocktype
        >> ins: many0!(parse_instr)
        >> btag!(0x0b)
    )
    | ins!(0x04, Instr::IfElse(rt, ins1, ins2);
        rt: parse_blocktype
        >> ins1: many0!(parse_instr)
        >> ins2: map!(opt!(do_parse!(
            btag!(0x05)
            >> ins2: many0!(parse_instr)
            >> (ins2)
        )), |x| x.unwrap_or_default())
        >> btag!(0x0b)
    )
    | ins!(0x0c, Instr::Br(l);
        l: parse_labelidx
    )
    | ins!(0x0d, Instr::BrIf(l);
        l: parse_labelidx
    )
    | ins!(0x0e, Instr::BrTable(ls, lN);
        ls: call!(parse_vec, parse_labelidx)
        >> lN: parse_labelidx
    )
    | ins!(0x0f, Instr::Return)
    | ins!(0x10, Instr::Call(x);
        x: parse_funcidx
    )
    | ins!(0x11, Instr::CallIndirect(x);
        x: parse_typeidx
        >> btag!(0x00)
    )

    // 5.4.2. Parametric Instructions
    | ins!(0x1A, Instr::Drop)
    | ins!(0x1B, Instr::Select)

    // 5.4.3. Variable Instructions
    | ins!(0x20, Instr::GetLocal(x); x: parse_localidx)
    | ins!(0x21, Instr::SetLocal(x); x: parse_localidx)
    | ins!(0x22, Instr::TeeLocal(x); x: parse_localidx)
    | ins!(0x23, Instr::GetGlobal(x); x: parse_globalidx)
    | ins!(0x24, Instr::SetGlobal(x); x: parse_globalidx)

    // 5.4.4. Memory Instructions
    | ins!(0x28, Instr::I32Load(m); m: parse_memarg)
    | ins!(0x29, Instr::I64Load(m); m: parse_memarg)
    | ins!(0x2A, Instr::F32Load(m); m: parse_memarg)
    | ins!(0x2B, Instr::F64Load(m); m: parse_memarg)

    | ins!(0x2C, Instr::I32Load8S(m); m: parse_memarg)
    | ins!(0x2D, Instr::I32Load8U(m); m: parse_memarg)
    | ins!(0x2E, Instr::I32Load16S(m); m: parse_memarg)
    | ins!(0x2F, Instr::I32Load16U(m); m: parse_memarg)

    | ins!(0x30, Instr::I64Load8S(m); m: parse_memarg)
    | ins!(0x31, Instr::I64Load8U(m); m: parse_memarg)
    | ins!(0x32, Instr::I64Load16S(m); m: parse_memarg)
    | ins!(0x33, Instr::I64Load16U(m); m: parse_memarg)
    | ins!(0x34, Instr::I64Load32S(m); m: parse_memarg)
    | ins!(0x35, Instr::I64Load32U(m); m: parse_memarg)

    | ins!(0x36, Instr::I32Store(m); m: parse_memarg)
    | ins!(0x37, Instr::I64Store(m); m: parse_memarg)
    | ins!(0x38, Instr::F32Store(m); m: parse_memarg)
    | ins!(0x39, Instr::F64Store(m); m: parse_memarg)

    | ins!(0x3A, Instr::I32Store8(m); m: parse_memarg)
    | ins!(0x3B, Instr::I32Store16(m); m: parse_memarg)

    | ins!(0x3C, Instr::I64Store8(m); m: parse_memarg)
    | ins!(0x3D, Instr::I64Store16(m); m: parse_memarg)
    | ins!(0x3E, Instr::I64Store32(m); m: parse_memarg)

    | ins!(0x3F, Instr::CurrentMemory)
    | ins!(0x40, Instr::GrowMemory)

    // 5.4.5. Numeric Instructions
    | ins!(0x41, Instr::I32Const(n); n: parse_i32)
    | ins!(0x42, Instr::I64Const(n); n: parse_i64)
    | ins!(0x43, Instr::F32Const(z); z: parse_f32)
    | ins!(0x44, Instr::F64Const(z); z: parse_f64)

    | ins!(0x45, Instr::I32EqZ)
    | ins!(0x46, Instr::I32Eq)
    | ins!(0x47, Instr::I32Ne)
    | ins!(0x48, Instr::I32LtS)
    | ins!(0x49, Instr::I32LtU)
    | ins!(0x4A, Instr::I32GtS)
    | ins!(0x4B, Instr::I32GtU)
    | ins!(0x4C, Instr::I32LeS)
    | ins!(0x4D, Instr::I32LeU)
    | ins!(0x4E, Instr::I32GeS)
    | ins!(0x4F, Instr::I32GeU)

    | ins!(0x50, Instr::I64EqZ)
    | ins!(0x51, Instr::I64Eq)
    | ins!(0x52, Instr::I64Ne)
    | ins!(0x53, Instr::I64LtS)
    | ins!(0x54, Instr::I64LtU)
    | ins!(0x55, Instr::I64GtS)
    | ins!(0x56, Instr::I64GtU)
    | ins!(0x57, Instr::I64LeS)
    | ins!(0x58, Instr::I64LeU)
    | ins!(0x59, Instr::I64GeS)
    | ins!(0x5A, Instr::I64GeU)

    | ins!(0x5B, Instr::F32Eq)
    | ins!(0x5C, Instr::F32Ne)
    | ins!(0x5D, Instr::F32Lt)
    | ins!(0x5E, Instr::F32Gt)
    | ins!(0x5F, Instr::F32Le)
    | ins!(0x60, Instr::F32Ge)

    | ins!(0x61, Instr::F64Eq)
    | ins!(0x62, Instr::F64Ne)
    | ins!(0x63, Instr::F64Lt)
    | ins!(0x64, Instr::F64Gt)
    | ins!(0x65, Instr::F64Le)
    | ins!(0x66, Instr::F64Ge)

    | ins!(0x67, Instr::I32Clz)
    | ins!(0x68, Instr::I32Ctz)
    | ins!(0x69, Instr::I32Popcnt)
    | ins!(0x6A, Instr::I32Add)
    | ins!(0x6B, Instr::I32Sub)
    | ins!(0x6C, Instr::I32Mul)
    | ins!(0x6D, Instr::I32DivS)
    | ins!(0x6E, Instr::I32DivU)
    | ins!(0x6F, Instr::I32RemS)
    | ins!(0x70, Instr::I32RemU)
    | ins!(0x71, Instr::I32And)
    | ins!(0x72, Instr::I32Or)
    | ins!(0x73, Instr::I32Xor)
    | ins!(0x74, Instr::I32Shl)
    | ins!(0x75, Instr::I32ShrS)
    | ins!(0x76, Instr::I32ShrU)
    | ins!(0x77, Instr::I32Rotl)
    | ins!(0x78, Instr::I32Rotr)

    | ins!(0x79, Instr::I64Clz)
    | ins!(0x7A, Instr::I64Ctz)
    | ins!(0x7B, Instr::I64Popcnt)
    | ins!(0x7C, Instr::I64Add)
    | ins!(0x7D, Instr::I64Sub)
    | ins!(0x7E, Instr::I64Mul)
    | ins!(0x7F, Instr::I64DivS)
    | ins!(0x80, Instr::I64DivU)
    | ins!(0x81, Instr::I64RemS)
    | ins!(0x82, Instr::I64RemU)
    | ins!(0x83, Instr::I64And)
    | ins!(0x84, Instr::I64Or)
    | ins!(0x85, Instr::I64Xor)
    | ins!(0x86, Instr::I64Shl)
    | ins!(0x87, Instr::I64ShrS)
    | ins!(0x88, Instr::I64ShrU)
    | ins!(0x89, Instr::I64Rotl)
    | ins!(0x8A, Instr::I64Rotr)

    | ins!(0x8B, Instr::F32Abs)
    | ins!(0x8C, Instr::F32Neg)
    | ins!(0x8D, Instr::F32Ceil)
    | ins!(0x8E, Instr::F32Floor)
    | ins!(0x8F, Instr::F32Trunc)
    | ins!(0x90, Instr::F32Nearest)
    | ins!(0x91, Instr::F32Sqrt)
    | ins!(0x92, Instr::F32Add)
    | ins!(0x93, Instr::F32Sub)
    | ins!(0x94, Instr::F32Mul)
    | ins!(0x95, Instr::F32Div)
    | ins!(0x96, Instr::F32Min)
    | ins!(0x97, Instr::F32Max)
    | ins!(0x98, Instr::F32CopySign)

    | ins!(0x99, Instr::F64Abs)
    | ins!(0x9A, Instr::F64Neg)
    | ins!(0x9B, Instr::F64Ceil)
    | ins!(0x9C, Instr::F64Floor)
    | ins!(0x9D, Instr::F64Trunc)
    | ins!(0x9E, Instr::F64Nearest)
    | ins!(0x9F, Instr::F64Sqrt)
    | ins!(0xA0, Instr::F64Add)
    | ins!(0xA1, Instr::F64Sub)
    | ins!(0xA2, Instr::F64Mul)
    | ins!(0xA3, Instr::F64Div)
    | ins!(0xA4, Instr::F64Min)
    | ins!(0xA5, Instr::F64Max)
    | ins!(0xA6, Instr::F64CopySign)

    | ins!(0xA7, Instr::I32WrapI64)
    | ins!(0xA8, Instr::I32TruncSF32)
    | ins!(0xA9, Instr::I32TruncUF32)
    | ins!(0xAA, Instr::I32TruncSF64)
    | ins!(0xAB, Instr::I32TruncUF64)

    | ins!(0xAC, Instr::I64ExtendSI32)
    | ins!(0xAD, Instr::I64ExtendUI32)
    | ins!(0xAE, Instr::I64TruncSF32)
    | ins!(0xAF, Instr::I64TruncUF32)
    | ins!(0xB0, Instr::I64TruncSF64)
    | ins!(0xB1, Instr::I64TruncUF64)

    | ins!(0xB2, Instr::F32ConvertSI32)
    | ins!(0xB3, Instr::F32ConvertUI32)
    | ins!(0xB4, Instr::F32ConvertSI64)
    | ins!(0xB5, Instr::F32ConvertUI64)
    | ins!(0xB6, Instr::F32DemoteF64)

    | ins!(0xB7, Instr::F64ConvertSI32)
    | ins!(0xB8, Instr::F64ConvertUI32)
    | ins!(0xB9, Instr::F64ConvertSI64)
    | ins!(0xBA, Instr::F64ConvertUI64)
    | ins!(0xBB, Instr::F64PromoteF32)

    | ins!(0xBC, Instr::I32ReinterpretF32)
    | ins!(0xBD, Instr::I64ReinterpretF64)
    | ins!(0xBE, Instr::F32ReinterpretI32)
    | ins!(0xBF, Instr::F64ReinterpretI64)
));
use structure::instructions::Memarg;
named!(parse_memarg <Inp, Memarg>, do_parse!(
    a: parse_u32
    >> o: parse_u32
    >> (Memarg { offset: o, align: a })
));

// 5.4.6. Expressions
use structure::instructions::Expr;
named!(parse_expr <Inp, Expr>, do_parse!(
    ins: many0!(parse_instr)
    >> btag!(0x0B)
    >> (Expr { body: ins })
));

// 5.5.1. Indices
use structure::modules::TypeIdx;
use structure::modules::FuncIdx;
use structure::modules::TableIdx;
use structure::modules::MemIdx;
use structure::modules::GlobalIdx;
use structure::modules::LocalIdx;
use structure::modules::LabelIdx;
named!(parse_typeidx <Inp, TypeIdx>, call!(parse_u32));
named!(parse_funcidx <Inp, FuncIdx>, call!(parse_u32));
named!(parse_tableidx <Inp, TableIdx>, call!(parse_u32));
named!(parse_memidx <Inp, MemIdx>, call!(parse_u32));
named!(parse_globalidx <Inp, GlobalIdx>, call!(parse_u32));
named!(parse_localidx <Inp, LocalIdx>, call!(parse_u32));
named!(parse_labelidx <Inp, LabelIdx>, call!(parse_u32));

// 5.5.1. Sections
fn parse_section<'a, F, B>(input: Inp<'a>, N: u8, parse_B: F) -> IResult<Inp<'a>, B>
    where F: Fn(Inp<'a>) -> IResult<Inp<'a>, B>
{
    do_parse!(input,
        btag!(N)
        >> cont: length_value!(
            parse_u32,
            parse_B
        )
        >> (cont)
    )
}

// 5.5.3. Custom Section
use structure::modules::Custom;
named!(parse_custom <Inp, Custom>, do_parse!(
    name: parse_name
    >> bytes: many0!(parse_byte)
    >> (Custom { name, bytes })
));
named!(parse_customsec <Inp, Custom>,
    call!(parse_section, 0, parse_custom)
);
named!(parse_customsecs <Inp, Vec<Custom>>,
    many0!(parse_customsec)
);



#[cfg(test)]
#[path="tests_binary_format.rs"]
mod tests;