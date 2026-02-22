use std::sync::Arc;

pub type FuncId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Local(u32),
    Global(u32),
    Arg(u32),
    Ret(u32),
    Err(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(Arc<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    StoreConst {
        slot: Slot,
        value: ConstValue,
    },
    Move {
        from: Slot,
        to: Slot,
    },

    Add {
        a: Slot,
        b: Slot,
        out: Slot,
    },
    Sub {
        a: Slot,
        b: Slot,
        out: Slot,
    },
    Mul {
        a: Slot,
        b: Slot,
        out: Slot,
    },
    Div {
        a: Slot,
        b: Slot,
        out: Slot,
    },

    Eq {
        a: Slot,
        b: Slot,
        out: Slot,
    },
    Lt {
        a: Slot,
        b: Slot,
        out: Slot,
    },

    Jump {
        target: usize,
    },
    Branch {
        cond: Slot,
        then_pc: usize,
        else_pc: usize,
    },

    Invoke {
        fn_slot: Slot,
        args: Vec<Slot>,
        out: Slot,
    },
    ReturnSet {
        slot_id: u32,
        value: Slot,
    },
    Exit,

    Throw {
        code: String,
        msg: String,
    },
    TryPush {
        handler_pc: usize,
    },
    TryPop,

    ObjNew {
        out: Slot,
    },
    ObjSet {
        obj: Slot,
        key: Slot,
        value: Slot,
        out: Slot,
    },
    ObjGet {
        obj: Slot,
        key: Slot,
        out: Slot,
    },
    ObjHas {
        obj: Slot,
        key: Slot,
        out: Slot,
    },
    StrConcat {
        a: Slot,
        b: Slot,
        out: Slot,
    },
    StrLen {
        value: Slot,
        out: Slot,
    },

    HostPrint {
        slot: Slot,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RetShape {
    Scalar,
    Either(Vec<String>),
    Record(Vec<String>),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnMeta {
    pub name: Arc<str>,
    pub arg_count: u32,
    pub ret_count: u32,
    pub retshape: RetShape,
}

#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub id: FuncId,
    pub code: Arc<[Instr]>,
    pub local_count: u32,
    pub arg_count: u32,
    pub ret_count: u32,
    pub err_count: u32,
    pub meta: FnMeta,
}

#[derive(Debug, Clone)]
pub struct ImportBinding {
    pub path: String,
    pub alias: String,
    pub module: Arc<CompiledModule>,
    pub export_to_global: Vec<(String, u32)>,
}

#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub name: Arc<str>,
    pub init_func: FuncId,
    pub functions: Vec<CompiledFunction>,
    pub function_globals: Vec<(u32, FuncId)>,
    pub exports: Vec<(String, u32)>,
    pub imports: Vec<ImportBinding>,
    pub global_count: u32,
}

impl CompiledModule {
    pub fn function(&self, id: FuncId) -> Option<&CompiledFunction> {
        self.functions.iter().find(|f| f.id == id)
    }
}
