use structure::types::*;
use structure::instructions::*;
use structure::modules::*;
use crate::structure_references::*;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

// TODO: util module
#[derive(Clone, Debug, PartialEq)]
pub struct TypedIndexVec<T, IndexT> {
    data: Vec<T>,
    _marker: PhantomData<IndexT>,
}

impl<T, IndexT> TypedIndexVec<T, IndexT>
    where IndexT: Into<usize> + From<usize>
{
    pub fn get(&self, addr: IndexT) -> Option<&T> {
        self.data.get(addr.into())
    }
    pub fn get_mut(&mut self, addr: IndexT) -> Option<&mut T> {
        self.data.get_mut(addr.into())
    }
    pub fn next_addr(&self) -> IndexT {
        self.data.len().into()
    }
    pub fn push(&mut self, inst: T) -> IndexT {
        let addr = self.next_addr();
        self.data.push(inst);
        addr
    }

    /// This should only be called for temporary data
    pub fn pop_aux(&mut self) {
        self.data.pop();
    }
}
impl<T, IndexT> From<Vec<T>> for TypedIndexVec<T, IndexT> {
    fn from(v: Vec<T>) -> Self {
        Self {
            data: v,
            _marker: PhantomData,
        }
    }
}
impl<T, IndexT> Index<IndexT> for TypedIndexVec<T, IndexT>
    where IndexT: Into<usize> + From<usize>
{
    type Output = T;
    fn index(&self, addr: IndexT) -> &T {
        &self.data[addr.into()]
    }
}
impl<T, IndexT> IndexMut<IndexT> for TypedIndexVec<T, IndexT>
    where IndexT: Into<usize> + From<usize>
{
    fn index_mut(&mut self, addr: IndexT) -> &mut T {
        &mut self.data[addr.into()]
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Val {
    I32(I32),
    I64(I64),
    F32(F32),
    F64(F64),
}
impl Val {
    pub fn ty(&self) -> ValType {
        match *self {
            Val::I32(_) => ValType::I32,
            Val::I64(_) => ValType::I64,
            Val::F32(_) => ValType::F32,
            Val::F64(_) => ValType::F64,
        }
    }
}

pub enum Result {
    Vals(Vec<Val>),
    Trap,
}

pub struct Store<Refs>
    where Refs: StructureReference
{
    pub funcs: TypedIndexVec<FuncInst<Refs>, FuncAddr>,
    pub tables: TypedIndexVec<TableInst, TableAddr>,
    pub mems: TypedIndexVec<MemInst, MemAddr>,
    pub globals: TypedIndexVec<GlobalInst, GlobalAddr>,

    /// All instanced modules in the `Store`
    ///
    /// This is a modification of the spec to make it easier to resolve cycles
    /// between data structures.
    pub modules: TypedIndexVec<ModuleInst<Refs>, ModuleAddr>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct FuncAddr(pub usize);
impl Into<usize> for FuncAddr { fn into(self) -> usize { self.0 } }
impl From<usize> for FuncAddr { fn from(a: usize) -> Self { FuncAddr(a) } }

#[derive(Clone, Copy, PartialEq)]
pub struct TableAddr(pub usize);
impl Into<usize> for TableAddr { fn into(self) -> usize { self.0 } }
impl From<usize> for TableAddr { fn from(a: usize) -> Self { TableAddr(a) } }

#[derive(Clone, Copy, PartialEq)]
pub struct MemAddr(pub usize);
impl Into<usize> for MemAddr { fn into(self) -> usize { self.0 } }
impl From<usize> for MemAddr { fn from(a: usize) -> Self { MemAddr(a) } }

#[derive(Clone, Copy, PartialEq)]
pub struct GlobalAddr(pub usize);
impl Into<usize> for GlobalAddr { fn into(self) -> usize { self.0 } }
impl From<usize> for GlobalAddr { fn from(a: usize) -> Self { GlobalAddr(a) } }

/// Address of a `ModuleInst` in the `Store`
///
/// This is a modification of the spec to make it easier to resolve cycles
/// between data structures.
#[derive(Clone, Copy, PartialEq)]
pub struct ModuleAddr(pub usize);
impl Into<usize> for ModuleAddr { fn into(self) -> usize { self.0 } }
impl From<usize> for ModuleAddr { fn from(a: usize) -> Self { ModuleAddr(a) } }

#[derive(Clone)]
pub struct ModuleInst<Refs>
    where Refs: StructureReference
{
    pub types: Refs::FuncTypesRef,
    pub funcaddrs: TypedIndexVec<FuncAddr, FuncIdx>,
    pub tableaddrs: TypedIndexVec<TableAddr, TableIdx>,
    pub memaddrs: TypedIndexVec<MemAddr, MemIdx>,
    pub globaladdrs: TypedIndexVec<GlobalAddr, GlobalIdx>,
    pub exports: Vec<ExportInst<Refs>>,
}

#[derive(Clone)]
pub enum FuncInst<Refs>
    where Refs: StructureReference
{
    Internal {
        type_: Refs::FuncTypeRef,
        module: ModuleAddr,
        code: Refs::FuncRef,
    },
    Host {
        type_: Refs::FuncTypeRef,
        hostcode: HostFunc
    },
}
impl<Refs> FuncInst<Refs> where Refs: StructureReference {
    pub fn type_(&self) -> &Refs::FuncTypeRef {
        match self {
            | FuncInst::Internal { type_, .. }
            | FuncInst::Host { type_, .. }
            => {
                type_
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct HostFunc {
    pub id: u32
}

pub struct TableInst {
    pub elem: Wec<FuncElem>,
    pub max: Option<u32>
}

#[derive(Clone, Copy)]
pub struct FuncElem(pub Option<FuncAddr>);

pub struct MemInst {
    pub data: Wec<Byte>,
    pub max: Option<u32>,
}

pub struct GlobalInst {
    pub value: Val,
    pub mutability: Mut,
}

#[derive(Clone)]
pub struct ExportInst<Refs>
    where Refs: StructureReference
{
    pub name: Refs::NameRef,
    pub value: ExternVal,
}

#[derive(Clone)]
pub enum ExternVal {
    Func(FuncAddr),
    Table(TableAddr),
    Mem(MemAddr),
    Global(GlobalAddr),
}

#[derive(Default)]
pub struct Stack<'instr> {
    data: Vec<StackElem<'instr>>,
    frame_indices: Vec<usize>,
    label_indices: Vec<usize>,
}

impl Stack<'instr> {
    pub fn new() -> Self { Self::default() }

    pub fn push_val(&mut self, val: Val) {
        self.data.push(StackElem::Val(val));
    }
    pub fn push_label(&mut self, n: usize, branch_target: &'instr [Instr], next_instr: &'instr [Instr]) {
        self.label_indices.push(self.data.len());
        self.data.push(StackElem::Label(Label {
            n,
            branch_target,
            next_instr,
        }));
    }
    pub fn push_frame(&mut self, n: usize, frame: Frame, next_instr: &'instr [Instr]) {
        self.frame_indices.push(self.data.len());
        self.data.push(StackElem::Activation(Activation {
            n,
            frame,
            next_instr,
        }));
    }

    pub fn top(&self) -> Option<&StackElem> {
        self.data.last()
    }

    pub fn pop_frame(&mut self) -> Activation {
        self.frame_indices.pop();
        if let Some(StackElem::Activation(a)) = self.data.pop() {
            a
        } else {
            panic!("No Frame at top of stack")
        }
    }

    pub fn pop_val(&mut self) -> Val {
        if let Some(StackElem::Val(val)) = self.data.pop() {
            val
        } else {
            panic!("No Val at top of stack")
        }
    }

    pub fn pop_label(&mut self) -> Label {
        self.label_indices.pop();
        if let Some(StackElem::Label(l)) = self.data.pop() {
            l
        } else {
            panic!("No Label at top of stack")
        }
    }

    pub fn lth_label(&self, l: LabelIdx) -> Label {
        let len = self.label_indices.len();
        let pos = self.label_indices[len - 1 - (l.0 as usize)];
        if let StackElem::Label(l) = self.data[pos] {
            l
        } else {
            panic!("No Label l at position pos of stack")
            // TODO: more useful debug messages
        }
    }

    pub fn peek_val(&mut self) -> Val {
        if let Some(&StackElem::Val(val)) = self.data.last() {
            val
        } else {
            panic!("No Val at top of stack")
        }
    }

    pub fn current_activation(&mut self) -> &mut Activation {
        let cfi = *self.frame_indices.last().expect("No Frame at top of stack");
        if let StackElem::Activation (ref mut a) = self.data[cfi] {
            a
        } else {
            panic!("No Frame at top of stack")
        }
    }

    pub fn current_label(&self) -> &Label {
        let cli = *self.label_indices.last().expect("No Label at top of stack");
        if let StackElem::Label(ref l) = self.data[cli] {
            l
        } else {
            panic!("No Label at top of stack")
        }
    }

    pub fn current_frame(&mut self) -> &mut Frame {
        &mut self.current_activation().frame
    }

    pub fn current_frame_arity(&mut self) -> usize {
        self.current_activation().n
    }

    pub fn label_count(&self) -> usize {
        self.label_indices.len()
    }

    pub fn top_ctrl_entry(&self) -> TopCtrlEntry {
        let fi = self.frame_indices.last();
        let li = self.label_indices.last();
        match (fi, li) {
            (Some(fi), Some(li)) if fi > li => TopCtrlEntry::Activation,
            (Some(fi), Some(li)) if fi < li => TopCtrlEntry::Label,
            (Some(_), None) => TopCtrlEntry::Activation,
            (None, Some(_)) => TopCtrlEntry::Label,
            (None, None) => TopCtrlEntry::None,
            (Some(_), Some(_)) => unreachable!(),
        }
    }
}

pub enum TopCtrlEntry {
    Label,
    Activation,
    None,
}

#[derive(PartialEq)]
pub struct Label<'instr> {
    pub n: usize, // NB. Can only be 0 or 1
    pub branch_target: &'instr [Instr],

    // NB: Added to make instr execution less error prone
    pub next_instr: &'instr [Instr],
}

#[derive(PartialEq)]
pub struct Activation<'instr> {
    pub n: usize, // NB. Can only be 0 or 1
    pub frame: Frame,

    // NB: Added to make instr execution less error prone
    pub next_instr: &'instr [Instr],
}

#[derive(PartialEq)]
pub enum StackElem<'instr> {
    Val(Val),
    Label(Label<'instr>),
    Activation(Activation<'instr>),
}

#[derive(PartialEq)]
pub struct Frame {
    pub locals: TypedIndexVec<Val, LocalIdx>,
    pub module: ModuleAddr,
}
