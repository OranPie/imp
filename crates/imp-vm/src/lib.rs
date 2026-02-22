use imp_ir::{CompiledFunction, CompiledModule, ConstValue, FnMeta, FuncId, Instr, RetShape, Slot};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(Arc<str>),
    Obj(HashMap<String, Value>),
    Func(FuncId),
    Error { code: Arc<str>, msg: Arc<str> },
}

impl Value {
    fn from_const(value: &ConstValue) -> Self {
        match value {
            ConstValue::Null => Self::Null,
            ConstValue::Bool(flag) => Self::Bool(*flag),
            ConstValue::Num(num) => Self::Num(*num),
            ConstValue::Str(text) => Self::Str(Arc::clone(text)),
        }
    }

    fn as_num(&self) -> Result<f64, VmError> {
        if let Self::Num(num) = self {
            Ok(*num)
        } else {
            Err(VmError::Runtime("expected numeric value".to_owned()))
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(value) => *value,
            Self::Num(value) => *value != 0.0,
            Self::Str(value) => !value.is_empty(),
            Self::Obj(map) => !map.is_empty(),
            Self::Func(_) => true,
            Self::Error { .. } => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub enable_host_print: bool,
    pub enable_jit: bool,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            enable_host_print: true,
            enable_jit: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunResult {
    pub returns: Vec<Value>,
    pub exports: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum VmError {
    Runtime(String),
    Thrown { code: Arc<str>, msg: Arc<str> },
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(message) => write!(f, "runtime error: {message}"),
            Self::Thrown { code, msg } => write!(f, "uncaught throw ({code}): {msg}"),
        }
    }
}

impl std::error::Error for VmError {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct JitKey {
    module_name: String,
    func_id: FuncId,
}

impl JitKey {
    fn new(module: &CompiledModule, function: &CompiledFunction) -> Self {
        Self {
            module_name: module.name.to_string(),
            func_id: function.id,
        }
    }
}

#[derive(Debug, Clone)]
struct JitFunction {
    steps: Arc<[JitStep]>,
}

#[derive(Debug, Clone)]
struct ForeignFunc {
    module: Arc<CompiledModule>,
    func_id: FuncId,
}

impl JitFunction {
    fn compile(function: &CompiledFunction) -> Self {
        let steps = function
            .code
            .iter()
            .map(JitStep::from_instr)
            .collect::<Vec<_>>();
        Self {
            steps: Arc::from(steps),
        }
    }
}

#[derive(Debug, Clone)]
struct JitStep {
    exec: StepExec,
    operands: JitOperands,
}

impl JitStep {
    fn from_instr(instr: &Instr) -> Self {
        match instr {
            Instr::StoreConst { slot, value } => Self {
                exec: step_store_const,
                operands: JitOperands::StoreConst {
                    slot: *slot,
                    value: Value::from_const(value),
                },
            },
            Instr::Move { from, to } => Self {
                exec: step_move,
                operands: JitOperands::Move {
                    from: *from,
                    to: *to,
                },
            },
            Instr::Add { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Add,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Sub { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Sub,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Mul { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Mul,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Div { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Div,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Eq { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Eq,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Lt { a, b, out } => Self {
                exec: step_binary,
                operands: JitOperands::Binary {
                    kind: BinaryOp::Lt,
                    a: *a,
                    b: *b,
                    out: *out,
                },
            },
            Instr::Jump { target } => Self {
                exec: step_jump,
                operands: JitOperands::Jump { target: *target },
            },
            Instr::Branch {
                cond,
                then_pc,
                else_pc,
            } => Self {
                exec: step_branch,
                operands: JitOperands::Branch {
                    cond: *cond,
                    then_pc: *then_pc,
                    else_pc: *else_pc,
                },
            },
            Instr::Invoke { fn_slot, args, out } => Self {
                exec: step_invoke,
                operands: JitOperands::Invoke {
                    fn_slot: *fn_slot,
                    args: args.clone(),
                    out: *out,
                },
            },
            Instr::ReturnSet { slot_id, value } => Self {
                exec: step_return_set,
                operands: JitOperands::ReturnSet {
                    slot_id: *slot_id,
                    value: *value,
                },
            },
            Instr::Exit => Self {
                exec: step_exit,
                operands: JitOperands::None,
            },
            Instr::Throw { code, msg } => Self {
                exec: step_throw,
                operands: JitOperands::Throw {
                    code: Arc::from(code.as_str()),
                    msg: Arc::from(msg.as_str()),
                },
            },
            Instr::TryPush { handler_pc } => Self {
                exec: step_try_push,
                operands: JitOperands::TryPush {
                    handler_pc: *handler_pc,
                },
            },
            Instr::TryPop => Self {
                exec: step_try_pop,
                operands: JitOperands::None,
            },
            Instr::ObjNew { out } => Self {
                exec: step_obj_new,
                operands: JitOperands::UnarySlot { slot: *out },
            },
            Instr::ObjSet {
                obj,
                key,
                value,
                out,
            } => Self {
                exec: step_obj_set,
                operands: JitOperands::ObjSet {
                    obj: *obj,
                    key: Arc::from(key.as_str()),
                    value: *value,
                    out: *out,
                },
            },
            Instr::ObjGet { obj, key, out } => Self {
                exec: step_obj_get,
                operands: JitOperands::ObjLookup {
                    kind: ObjLookupKind::Get,
                    obj: *obj,
                    key: *key,
                    out: *out,
                },
            },
            Instr::ObjHas { obj, key, out } => Self {
                exec: step_obj_get,
                operands: JitOperands::ObjLookup {
                    kind: ObjLookupKind::Has,
                    obj: *obj,
                    key: *key,
                    out: *out,
                },
            },
            Instr::StrConcat { a, b, out } => Self {
                exec: step_str,
                operands: JitOperands::StrOp {
                    kind: StrOpKind::Concat,
                    a: Some(*a),
                    b: Some(*b),
                    out: *out,
                },
            },
            Instr::StrLen { value, out } => Self {
                exec: step_str,
                operands: JitOperands::StrOp {
                    kind: StrOpKind::Len,
                    a: Some(*value),
                    b: None,
                    out: *out,
                },
            },
            Instr::HostPrint { slot } => Self {
                exec: step_host_print,
                operands: JitOperands::UnarySlot { slot: *slot },
            },
        }
    }
}

type StepExec = fn(
    &mut Vm,
    &CompiledModule,
    &mut Frame,
    &mut [Value],
    &JitOperands,
    usize,
) -> Result<StepControl, VmError>;

#[derive(Debug, Clone)]
enum JitOperands {
    None,
    UnarySlot {
        slot: Slot,
    },
    StoreConst {
        slot: Slot,
        value: Value,
    },
    Move {
        from: Slot,
        to: Slot,
    },
    Binary {
        kind: BinaryOp,
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
    Throw {
        code: Arc<str>,
        msg: Arc<str>,
    },
    TryPush {
        handler_pc: usize,
    },
    ObjSet {
        obj: Slot,
        key: Arc<str>,
        value: Slot,
        out: Slot,
    },
    ObjLookup {
        kind: ObjLookupKind,
        obj: Slot,
        key: Slot,
        out: Slot,
    },
    StrOp {
        kind: StrOpKind,
        a: Option<Slot>,
        b: Option<Slot>,
        out: Slot,
    },
}

#[derive(Debug, Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
}

#[derive(Debug, Clone, Copy)]
enum ObjLookupKind {
    Get,
    Has,
}

#[derive(Debug, Clone, Copy)]
enum StrOpKind {
    Concat,
    Len,
}

#[derive(Debug, Clone, Copy)]
enum StepControl {
    Next(usize),
    Exit,
}

#[derive(Debug, Clone)]
pub struct Vm {
    cfg: VmConfig,
    active_module: Option<CompiledModule>,
    jit_cache: HashMap<JitKey, Arc<JitFunction>>,
    foreign_funcs: HashMap<FuncId, ForeignFunc>,
    next_foreign_func_id: FuncId,
}

impl Vm {
    pub fn new(cfg: VmConfig) -> Self {
        Self {
            cfg,
            active_module: None,
            jit_cache: HashMap::new(),
            foreign_funcs: HashMap::new(),
            next_foreign_func_id: 1_000_000,
        }
    }

    pub fn run_main(&mut self, module: &CompiledModule) -> Result<RunResult, VmError> {
        self.active_module = Some(module.clone());
        let mut globals = self.build_module_globals(module)?;

        let returns = self.execute_function(module, module.init_func, &[], &mut globals)?;

        let mut exports = HashMap::new();
        for (name, slot) in &module.exports {
            exports.insert(name.clone(), globals[*slot as usize].clone());
        }

        self.active_module = Some(module.clone());
        Ok(RunResult { returns, exports })
    }

    pub fn invoke(&mut self, func: FuncId, args: &[Value]) -> Result<Vec<Value>, VmError> {
        let module = self
            .active_module
            .as_ref()
            .ok_or_else(|| VmError::Runtime("no active module; call run_main first".to_owned()))?
            .clone();
        let mut globals = self.build_module_globals(&module)?;
        self.execute_function(&module, func, args, &mut globals)
    }

    fn build_module_globals(&mut self, module: &CompiledModule) -> Result<Vec<Value>, VmError> {
        let mut globals = vec![Value::Null; module.global_count as usize];

        for (slot, func_id) in &module.function_globals {
            globals[*slot as usize] = Value::Func(*func_id);
        }

        for import in &module.imports {
            let imported = self.run_main(&import.module)?;
            for (name, destination) in &import.export_to_global {
                if (*destination as usize) >= globals.len() {
                    continue;
                }
                if let Some(value) = imported.exports.get(name) {
                    let linked = self.link_imported_value(value, Arc::clone(&import.module));
                    globals[*destination as usize] = linked;
                }
            }
        }

        Ok(globals)
    }

    fn link_imported_value(&mut self, value: &Value, module: Arc<CompiledModule>) -> Value {
        if let Value::Func(func_id) = value {
            let handle = self.register_foreign_func(module, *func_id);
            Value::Func(handle)
        } else {
            value.clone()
        }
    }

    fn register_foreign_func(&mut self, module: Arc<CompiledModule>, func_id: FuncId) -> FuncId {
        let handle = self.next_foreign_func_id;
        self.next_foreign_func_id = self.next_foreign_func_id.saturating_add(1);
        self.foreign_funcs
            .insert(handle, ForeignFunc { module, func_id });
        handle
    }

    fn execute_function(
        &mut self,
        module: &CompiledModule,
        func_id: FuncId,
        args: &[Value],
        globals: &mut [Value],
    ) -> Result<Vec<Value>, VmError> {
        if module.function(func_id).is_none() {
            if let Some(foreign) = self.foreign_funcs.get(&func_id).cloned() {
                let mut foreign_globals = self.build_module_globals(&foreign.module)?;
                return self.execute_function(
                    &foreign.module,
                    foreign.func_id,
                    args,
                    &mut foreign_globals,
                );
            }
            return Err(VmError::Runtime(format!("unknown function id {func_id}")));
        }
        let function = module
            .function(func_id)
            .ok_or_else(|| VmError::Runtime(format!("unknown function id {func_id}")))?;
        let mut frame = Frame::new(function, args);

        if self.cfg.enable_jit {
            let jit = self.get_or_compile_jit(module, function);
            return self.execute_function_jit(module, &mut frame, globals, &jit);
        }

        self.execute_function_interpreter(module, &mut frame, globals)
    }

    fn get_or_compile_jit(
        &mut self,
        module: &CompiledModule,
        function: &CompiledFunction,
    ) -> Arc<JitFunction> {
        let key = JitKey::new(module, function);
        if let Some(cached) = self.jit_cache.get(&key) {
            return Arc::clone(cached);
        }
        let compiled = Arc::new(JitFunction::compile(function));
        self.jit_cache.insert(key, Arc::clone(&compiled));
        compiled
    }

    fn execute_function_jit(
        &mut self,
        module: &CompiledModule,
        frame: &mut Frame,
        globals: &mut [Value],
        jit: &JitFunction,
    ) -> Result<Vec<Value>, VmError> {
        let mut pc = 0usize;
        loop {
            if pc >= jit.steps.len() {
                return Err(VmError::Runtime(format!(
                    "pc {} out of range for {}",
                    pc, frame.meta.name
                )));
            }

            frame.pc = pc;
            let step = &jit.steps[pc];
            match (step.exec)(self, module, frame, globals, &step.operands, pc)? {
                StepControl::Next(next) => {
                    pc = next;
                }
                StepControl::Exit => {
                    validate_retshape(&frame.meta, &frame.ret)?;
                    return Ok(std::mem::take(&mut frame.ret));
                }
            }
        }
    }

    fn execute_function_interpreter(
        &mut self,
        module: &CompiledModule,
        frame: &mut Frame,
        globals: &mut [Value],
    ) -> Result<Vec<Value>, VmError> {
        loop {
            let Some(instr) = frame.code.get(frame.pc).cloned() else {
                return Err(VmError::Runtime(format!(
                    "pc {} out of range for {}",
                    frame.pc, frame.meta.name
                )));
            };

            match instr {
                Instr::StoreConst { slot, value } => {
                    frame.set(slot, Value::from_const(&value), globals);
                    frame.pc += 1;
                }
                Instr::Move { from, to } => {
                    let value = frame.get(from, globals)?;
                    frame.set(to, value, globals);
                    frame.pc += 1;
                }
                Instr::Add { a, b, out } => {
                    let sum = frame.get(a, globals)?.as_num()? + frame.get(b, globals)?.as_num()?;
                    frame.set(out, Value::Num(sum), globals);
                    frame.pc += 1;
                }
                Instr::Sub { a, b, out } => {
                    let diff =
                        frame.get(a, globals)?.as_num()? - frame.get(b, globals)?.as_num()?;
                    frame.set(out, Value::Num(diff), globals);
                    frame.pc += 1;
                }
                Instr::Mul { a, b, out } => {
                    let product =
                        frame.get(a, globals)?.as_num()? * frame.get(b, globals)?.as_num()?;
                    frame.set(out, Value::Num(product), globals);
                    frame.pc += 1;
                }
                Instr::Div { a, b, out } => {
                    let divisor = frame.get(b, globals)?.as_num()?;
                    if divisor == 0.0 {
                        let handled = frame.handle_throw("div_zero", "division by zero", globals);
                        if handled {
                            continue;
                        }
                        return Err(VmError::Thrown {
                            code: Arc::from("div_zero"),
                            msg: Arc::from("division by zero"),
                        });
                    }
                    let quotient = frame.get(a, globals)?.as_num()? / divisor;
                    frame.set(out, Value::Num(quotient), globals);
                    frame.pc += 1;
                }
                Instr::Eq { a, b, out } => {
                    let result = frame.get(a, globals)? == frame.get(b, globals)?;
                    frame.set(out, Value::Bool(result), globals);
                    frame.pc += 1;
                }
                Instr::Lt { a, b, out } => {
                    let result =
                        frame.get(a, globals)?.as_num()? < frame.get(b, globals)?.as_num()?;
                    frame.set(out, Value::Bool(result), globals);
                    frame.pc += 1;
                }
                Instr::Jump { target } => {
                    frame.pc = target;
                }
                Instr::Branch {
                    cond,
                    then_pc,
                    else_pc,
                } => {
                    let condition = frame.get(cond, globals)?.as_bool();
                    frame.pc = if condition { then_pc } else { else_pc };
                }
                Instr::Invoke { fn_slot, args, out } => {
                    let target = frame.get(fn_slot, globals)?;
                    let mut values = Vec::with_capacity(args.len());
                    for slot in &args {
                        values.push(frame.get(*slot, globals)?);
                    }
                    let Value::Func(target_func) = target else {
                        return Err(VmError::Runtime(
                            "invoke target is not a function".to_owned(),
                        ));
                    };

                    match self.execute_function(module, target_func, &values, globals) {
                        Ok(return_values) => {
                            let value = return_values.into_iter().next().unwrap_or(Value::Null);
                            frame.set(out, value, globals);
                            frame.pc += 1;
                        }
                        Err(VmError::Thrown { code, msg }) => {
                            let handled = frame.handle_throw(&code, &msg, globals);
                            if handled {
                                continue;
                            }
                            return Err(VmError::Thrown { code, msg });
                        }
                        Err(err) => return Err(err),
                    }
                }
                Instr::ReturnSet { slot_id, value } => {
                    let value = frame.get(value, globals)?;
                    frame.set_ret(slot_id as usize, value);
                    frame.pc += 1;
                }
                Instr::Exit => {
                    validate_retshape(&frame.meta, &frame.ret)?;
                    return Ok(std::mem::take(&mut frame.ret));
                }
                Instr::Throw { code, msg } => {
                    let handled = frame.handle_throw(&code, &msg, globals);
                    if handled {
                        continue;
                    }
                    return Err(VmError::Thrown {
                        code: Arc::from(code),
                        msg: Arc::from(msg),
                    });
                }
                Instr::TryPush { handler_pc } => {
                    frame.try_stack.push(handler_pc);
                    frame.pc += 1;
                }
                Instr::TryPop => {
                    frame.try_stack.pop();
                    frame.pc += 1;
                }
                Instr::ObjNew { out } => {
                    frame.set(out, Value::Obj(HashMap::new()), globals);
                    frame.pc += 1;
                }
                Instr::ObjSet {
                    obj,
                    key,
                    value,
                    out,
                } => {
                    let mut object = match frame.get(obj, globals)? {
                        Value::Obj(map) => map,
                        _ => {
                            return Err(VmError::Runtime(
                                "core::obj::set target is not an object".to_owned(),
                            ));
                        }
                    };
                    object.insert(key, frame.get(value, globals)?);
                    frame.set(out, Value::Obj(object), globals);
                    frame.pc += 1;
                }
                Instr::ObjGet { obj, key, out } => {
                    let object = frame.get(obj, globals)?;
                    let key_text = value_to_text(&frame.get(key, globals)?)?;
                    let value = object_lookup(&object, &key_text)?;
                    frame.set(out, value.unwrap_or(Value::Null), globals);
                    frame.pc += 1;
                }
                Instr::ObjHas { obj, key, out } => {
                    let object = frame.get(obj, globals)?;
                    let key_text = value_to_text(&frame.get(key, globals)?)?;
                    let has = object_lookup(&object, &key_text)?.is_some();
                    frame.set(out, Value::Bool(has), globals);
                    frame.pc += 1;
                }
                Instr::StrConcat { a, b, out } => {
                    let av = value_to_text(&frame.get(a, globals)?)?;
                    let bv = value_to_text(&frame.get(b, globals)?)?;
                    frame.set(out, Value::Str(Arc::from(format!("{av}{bv}"))), globals);
                    frame.pc += 1;
                }
                Instr::StrLen { value, out } => {
                    let text = value_to_text(&frame.get(value, globals)?)?;
                    frame.set(out, Value::Num(text.chars().count() as f64), globals);
                    frame.pc += 1;
                }
                Instr::HostPrint { slot } => {
                    if self.cfg.enable_host_print {
                        println!("{:?}", frame.get(slot, globals)?);
                    }
                    frame.pc += 1;
                }
            }
        }
    }
}

fn step_store_const(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::StoreConst { slot, value } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for store_const".to_owned(),
        ));
    };
    frame.set(*slot, value.clone(), globals);
    Ok(StepControl::Next(pc + 1))
}

fn step_move(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Move { from, to } = operands else {
        return Err(VmError::Runtime("jit operand mismatch for move".to_owned()));
    };
    let value = frame.get(*from, globals)?;
    frame.set(*to, value, globals);
    Ok(StepControl::Next(pc + 1))
}

fn step_binary(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Binary { kind, a, b, out } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for binary".to_owned(),
        ));
    };

    match kind {
        BinaryOp::Add => {
            let sum = frame.get(*a, globals)?.as_num()? + frame.get(*b, globals)?.as_num()?;
            frame.set(*out, Value::Num(sum), globals);
            Ok(StepControl::Next(pc + 1))
        }
        BinaryOp::Sub => {
            let diff = frame.get(*a, globals)?.as_num()? - frame.get(*b, globals)?.as_num()?;
            frame.set(*out, Value::Num(diff), globals);
            Ok(StepControl::Next(pc + 1))
        }
        BinaryOp::Mul => {
            let product = frame.get(*a, globals)?.as_num()? * frame.get(*b, globals)?.as_num()?;
            frame.set(*out, Value::Num(product), globals);
            Ok(StepControl::Next(pc + 1))
        }
        BinaryOp::Div => {
            let divisor = frame.get(*b, globals)?.as_num()?;
            if divisor == 0.0 {
                let handled = frame.handle_throw("div_zero", "division by zero", globals);
                if handled {
                    return Ok(StepControl::Next(frame.pc));
                }
                return Err(VmError::Thrown {
                    code: Arc::from("div_zero"),
                    msg: Arc::from("division by zero"),
                });
            }
            let quotient = frame.get(*a, globals)?.as_num()? / divisor;
            frame.set(*out, Value::Num(quotient), globals);
            Ok(StepControl::Next(pc + 1))
        }
        BinaryOp::Eq => {
            let result = frame.get(*a, globals)? == frame.get(*b, globals)?;
            frame.set(*out, Value::Bool(result), globals);
            Ok(StepControl::Next(pc + 1))
        }
        BinaryOp::Lt => {
            let result = frame.get(*a, globals)?.as_num()? < frame.get(*b, globals)?.as_num()?;
            frame.set(*out, Value::Bool(result), globals);
            Ok(StepControl::Next(pc + 1))
        }
    }
}

fn step_jump(
    _vm: &mut Vm,
    _module: &CompiledModule,
    _frame: &mut Frame,
    _globals: &mut [Value],
    operands: &JitOperands,
    _pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Jump { target } = operands else {
        return Err(VmError::Runtime("jit operand mismatch for jump".to_owned()));
    };
    Ok(StepControl::Next(*target))
}

fn step_branch(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    _pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Branch {
        cond,
        then_pc,
        else_pc,
    } = operands
    else {
        return Err(VmError::Runtime(
            "jit operand mismatch for branch".to_owned(),
        ));
    };
    let condition = frame.get(*cond, globals)?.as_bool();
    Ok(StepControl::Next(if condition {
        *then_pc
    } else {
        *else_pc
    }))
}

fn step_invoke(
    vm: &mut Vm,
    module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Invoke { fn_slot, args, out } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for invoke".to_owned(),
        ));
    };

    let target = frame.get(*fn_slot, globals)?;
    let mut values = Vec::with_capacity(args.len());
    for slot in args {
        values.push(frame.get(*slot, globals)?);
    }

    let Value::Func(target_func) = target else {
        return Err(VmError::Runtime(
            "invoke target is not a function".to_owned(),
        ));
    };

    match vm.execute_function(module, target_func, &values, globals) {
        Ok(return_values) => {
            let value = return_values.into_iter().next().unwrap_or(Value::Null);
            frame.set(*out, value, globals);
            Ok(StepControl::Next(pc + 1))
        }
        Err(VmError::Thrown { code, msg }) => {
            let handled = frame.handle_throw(&code, &msg, globals);
            if handled {
                Ok(StepControl::Next(frame.pc))
            } else {
                Err(VmError::Thrown { code, msg })
            }
        }
        Err(err) => Err(err),
    }
}

fn step_return_set(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::ReturnSet { slot_id, value } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for return_set".to_owned(),
        ));
    };
    let value = frame.get(*value, globals)?;
    frame.set_ret(*slot_id as usize, value);
    Ok(StepControl::Next(pc + 1))
}

fn step_exit(
    _vm: &mut Vm,
    _module: &CompiledModule,
    _frame: &mut Frame,
    _globals: &mut [Value],
    operands: &JitOperands,
    _pc: usize,
) -> Result<StepControl, VmError> {
    if !matches!(operands, JitOperands::None) {
        return Err(VmError::Runtime("jit operand mismatch for exit".to_owned()));
    }
    Ok(StepControl::Exit)
}

fn step_throw(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    _pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::Throw { code, msg } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for throw".to_owned(),
        ));
    };
    if frame.handle_throw(code, msg, globals) {
        return Ok(StepControl::Next(frame.pc));
    }
    Err(VmError::Thrown {
        code: Arc::clone(code),
        msg: Arc::clone(msg),
    })
}

fn step_try_push(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    _globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::TryPush { handler_pc } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for try_push".to_owned(),
        ));
    };
    frame.try_stack.push(*handler_pc);
    Ok(StepControl::Next(pc + 1))
}

fn step_try_pop(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    _globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    if !matches!(operands, JitOperands::None) {
        return Err(VmError::Runtime(
            "jit operand mismatch for try_pop".to_owned(),
        ));
    }
    frame.try_stack.pop();
    Ok(StepControl::Next(pc + 1))
}

fn step_obj_new(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::UnarySlot { slot } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for obj_new".to_owned(),
        ));
    };
    frame.set(*slot, Value::Obj(HashMap::new()), globals);
    Ok(StepControl::Next(pc + 1))
}

fn step_obj_set(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::ObjSet {
        obj,
        key,
        value,
        out,
    } = operands
    else {
        return Err(VmError::Runtime(
            "jit operand mismatch for obj_set".to_owned(),
        ));
    };

    let mut object = match frame.get(*obj, globals)? {
        Value::Obj(map) => map,
        _ => {
            return Err(VmError::Runtime(
                "core::obj::set target is not an object".to_owned(),
            ));
        }
    };
    object.insert(key.to_string(), frame.get(*value, globals)?);
    frame.set(*out, Value::Obj(object), globals);
    Ok(StepControl::Next(pc + 1))
}

fn step_obj_get(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::ObjLookup {
        kind,
        obj,
        key,
        out,
    } = operands
    else {
        return Err(VmError::Runtime(
            "jit operand mismatch for obj_lookup".to_owned(),
        ));
    };

    let object = frame.get(*obj, globals)?;
    let key_text = value_to_text(&frame.get(*key, globals)?)?;
    let value = object_lookup(&object, &key_text)?;
    match kind {
        ObjLookupKind::Get => frame.set(*out, value.unwrap_or(Value::Null), globals),
        ObjLookupKind::Has => frame.set(*out, Value::Bool(value.is_some()), globals),
    }
    Ok(StepControl::Next(pc + 1))
}

fn step_str(
    _vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::StrOp { kind, a, b, out } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for str op".to_owned(),
        ));
    };

    match kind {
        StrOpKind::Concat => {
            let a_slot = a.ok_or_else(|| VmError::Runtime("str concat missing a".to_owned()))?;
            let b_slot = b.ok_or_else(|| VmError::Runtime("str concat missing b".to_owned()))?;
            let av = value_to_text(&frame.get(a_slot, globals)?)?;
            let bv = value_to_text(&frame.get(b_slot, globals)?)?;
            frame.set(*out, Value::Str(Arc::from(format!("{av}{bv}"))), globals);
        }
        StrOpKind::Len => {
            let value_slot =
                a.ok_or_else(|| VmError::Runtime("str len missing value".to_owned()))?;
            let text = value_to_text(&frame.get(value_slot, globals)?)?;
            frame.set(*out, Value::Num(text.chars().count() as f64), globals);
        }
    }

    Ok(StepControl::Next(pc + 1))
}

fn step_host_print(
    vm: &mut Vm,
    _module: &CompiledModule,
    frame: &mut Frame,
    globals: &mut [Value],
    operands: &JitOperands,
    pc: usize,
) -> Result<StepControl, VmError> {
    let JitOperands::UnarySlot { slot } = operands else {
        return Err(VmError::Runtime(
            "jit operand mismatch for host_print".to_owned(),
        ));
    };
    if vm.cfg.enable_host_print {
        println!("{:?}", frame.get(*slot, globals)?);
    }
    Ok(StepControl::Next(pc + 1))
}

fn object_lookup(object: &Value, key: &str) -> Result<Option<Value>, VmError> {
    match object {
        Value::Obj(map) => Ok(map.get(key).cloned()),
        _ => Err(VmError::Runtime(
            "object lookup target is not an object".to_owned(),
        )),
    }
}

fn value_to_text(value: &Value) -> Result<String, VmError> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(v) => Ok(v.to_string()),
        Value::Num(v) => Ok(v.to_string()),
        Value::Str(v) => Ok(v.to_string()),
        Value::Error { code, msg } => Ok(format!("error({code}): {msg}")),
        Value::Obj(_) | Value::Func(_) => Err(VmError::Runtime(
            "cannot convert complex value to string".to_owned(),
        )),
    }
}

fn validate_retshape(meta: &FnMeta, values: &[Value]) -> Result<(), VmError> {
    match &meta.retshape {
        RetShape::Scalar => {
            if values.len() != 1 {
                return Err(VmError::Runtime(format!(
                    "{} expects scalar return with 1 slot, got {}",
                    meta.name,
                    values.len()
                )));
            }
        }
        RetShape::Either(allowed) => {
            if values.len() != 1 {
                return Err(VmError::Runtime(format!(
                    "{} expects single either slot",
                    meta.name
                )));
            }
            if let Value::Str(value) = &values[0]
                && allowed.iter().any(|item| item == value.as_ref())
            {
                return Ok(());
            }
            return Err(VmError::Runtime(format!(
                "{} return is not in either(...) set",
                meta.name
            )));
        }
        RetShape::Record(fields) => {
            if values.len() != 1 {
                return Err(VmError::Runtime(format!(
                    "{} expects single record slot",
                    meta.name
                )));
            }
            let Value::Obj(map) = &values[0] else {
                return Err(VmError::Runtime(format!(
                    "{} return is not an object for record shape",
                    meta.name
                )));
            };
            for field in fields {
                if !map.contains_key(field) {
                    return Err(VmError::Runtime(format!(
                        "{} missing record field '{field}'",
                        meta.name
                    )));
                }
            }
        }
        RetShape::Any => {}
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct Frame {
    code: Arc<[Instr]>,
    pc: usize,
    locals: Vec<Value>,
    args: Vec<Value>,
    ret: Vec<Value>,
    err: Vec<Value>,
    try_stack: Vec<usize>,
    meta: FnMeta,
}

impl Frame {
    fn new(function: &CompiledFunction, args: &[Value]) -> Self {
        let mut frame_args = vec![Value::Null; function.arg_count as usize];
        for (index, value) in args.iter().enumerate() {
            if index >= frame_args.len() {
                break;
            }
            frame_args[index] = value.clone();
        }

        Self {
            code: Arc::clone(&function.code),
            pc: 0,
            locals: vec![Value::Null; function.local_count as usize],
            args: frame_args,
            ret: vec![Value::Null; function.ret_count as usize],
            err: vec![Value::Null; function.err_count.max(1) as usize],
            try_stack: Vec::new(),
            meta: function.meta.clone(),
        }
    }

    fn get(&self, slot: Slot, globals: &[Value]) -> Result<Value, VmError> {
        match slot {
            Slot::Local(index) => self
                .locals
                .get(index as usize)
                .cloned()
                .ok_or_else(|| VmError::Runtime(format!("local slot {index} out of range"))),
            Slot::Global(index) => globals
                .get(index as usize)
                .cloned()
                .ok_or_else(|| VmError::Runtime(format!("global slot {index} out of range"))),
            Slot::Arg(index) => self
                .args
                .get(index as usize)
                .cloned()
                .ok_or_else(|| VmError::Runtime(format!("arg slot {index} out of range"))),
            Slot::Ret(index) => self
                .ret
                .get(index as usize)
                .cloned()
                .ok_or_else(|| VmError::Runtime(format!("ret slot {index} out of range"))),
            Slot::Err(index) => self
                .err
                .get(index as usize)
                .cloned()
                .ok_or_else(|| VmError::Runtime(format!("err slot {index} out of range"))),
        }
    }

    fn set(&mut self, slot: Slot, value: Value, globals: &mut [Value]) {
        match slot {
            Slot::Local(index) => set_vec_slot(&mut self.locals, index as usize, value),
            Slot::Global(index) => {
                if (index as usize) < globals.len() {
                    globals[index as usize] = value;
                }
            }
            Slot::Arg(index) => set_vec_slot(&mut self.args, index as usize, value),
            Slot::Ret(index) => set_vec_slot(&mut self.ret, index as usize, value),
            Slot::Err(index) => set_vec_slot(&mut self.err, index as usize, value),
        }
    }

    fn set_ret(&mut self, index: usize, value: Value) {
        set_vec_slot(&mut self.ret, index, value);
    }

    fn handle_throw(&mut self, code: &str, msg: &str, globals: &mut [Value]) -> bool {
        if let Some(handler_pc) = self.try_stack.pop() {
            self.set(
                Slot::Err(0),
                Value::Error {
                    code: Arc::from(code),
                    msg: Arc::from(msg),
                },
                globals,
            );
            self.pc = handler_pc;
            return true;
        }
        false
    }
}

fn set_vec_slot(vec: &mut Vec<Value>, index: usize, value: Value) {
    if index >= vec.len() {
        vec.resize(index + 1, Value::Null);
    }
    vec[index] = value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_compiler::{FsModuleLoader, compile_module};
    use imp_ir::{CompiledFunction, CompiledModule, ConstValue, FnMeta, Instr, RetShape, Slot};
    use std::fs;

    fn scalar_meta(name: &str) -> FnMeta {
        FnMeta {
            name: Arc::from(name),
            arg_count: 0,
            ret_count: 1,
            retshape: RetShape::Scalar,
        }
    }

    #[test]
    fn executes_add_and_return_jit() {
        let function = CompiledFunction {
            id: 0,
            code: Arc::from([
                Instr::StoreConst {
                    slot: Slot::Local(0),
                    value: ConstValue::Num(2.0),
                },
                Instr::StoreConst {
                    slot: Slot::Local(1),
                    value: ConstValue::Num(3.0),
                },
                Instr::Add {
                    a: Slot::Local(0),
                    b: Slot::Local(1),
                    out: Slot::Ret(0),
                },
                Instr::Exit,
            ]),
            local_count: 2,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main"),
        };

        let module = CompiledModule {
            name: Arc::from("main"),
            init_func: 0,
            functions: vec![function],
            function_globals: vec![],
            exports: vec![],
            imports: vec![],
            global_count: 0,
        };

        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Num(5.0)]);
    }

    #[test]
    fn catches_divide_by_zero_with_try_handler_jit() {
        let function = CompiledFunction {
            id: 0,
            code: Arc::from([
                Instr::StoreConst {
                    slot: Slot::Local(0),
                    value: ConstValue::Num(1.0),
                },
                Instr::StoreConst {
                    slot: Slot::Local(1),
                    value: ConstValue::Num(0.0),
                },
                Instr::TryPush { handler_pc: 5 },
                Instr::Div {
                    a: Slot::Local(0),
                    b: Slot::Local(1),
                    out: Slot::Ret(0),
                },
                Instr::Jump { target: 7 },
                Instr::StoreConst {
                    slot: Slot::Ret(0),
                    value: ConstValue::Num(99.0),
                },
                Instr::TryPop,
                Instr::Exit,
            ]),
            local_count: 2,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main"),
        };

        let module = CompiledModule {
            name: Arc::from("main"),
            init_func: 0,
            functions: vec![function],
            function_globals: vec![],
            exports: vec![],
            imports: vec![],
            global_count: 0,
        };

        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Num(99.0)]);
    }

    #[test]
    fn invoke_uses_function_global_slot_jit() {
        let init = CompiledFunction {
            id: 0,
            code: Arc::from([
                Instr::Invoke {
                    fn_slot: Slot::Global(0),
                    args: vec![],
                    out: Slot::Ret(0),
                },
                Instr::Exit,
            ]),
            local_count: 0,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main"),
        };

        let callee = CompiledFunction {
            id: 1,
            code: Arc::from([
                Instr::StoreConst {
                    slot: Slot::Ret(0),
                    value: ConstValue::Num(7.0),
                },
                Instr::Exit,
            ]),
            local_count: 0,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main::f"),
        };

        let module = CompiledModule {
            name: Arc::from("main"),
            init_func: 0,
            functions: vec![init, callee],
            function_globals: vec![(0, 1)],
            exports: vec![],
            imports: vec![],
            global_count: 1,
        };

        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Num(7.0)]);
    }

    #[test]
    fn interpreter_fallback_matches_behavior() {
        let function = CompiledFunction {
            id: 0,
            code: Arc::from([
                Instr::StoreConst {
                    slot: Slot::Local(0),
                    value: ConstValue::Num(10.0),
                },
                Instr::StoreConst {
                    slot: Slot::Local(1),
                    value: ConstValue::Num(4.0),
                },
                Instr::Sub {
                    a: Slot::Local(0),
                    b: Slot::Local(1),
                    out: Slot::Ret(0),
                },
                Instr::Exit,
            ]),
            local_count: 2,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main"),
        };

        let module = CompiledModule {
            name: Arc::from("main"),
            init_func: 0,
            functions: vec![function],
            function_globals: vec![],
            exports: vec![],
            imports: vec![],
            global_count: 0,
        };

        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: false,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Num(6.0)]);
    }

    #[test]
    fn new_core_ops_match_between_jit_and_interpreter() {
        let function = CompiledFunction {
            id: 0,
            code: Arc::from([
                Instr::ObjNew {
                    out: Slot::Local(0),
                },
                Instr::StoreConst {
                    slot: Slot::Local(1),
                    value: ConstValue::Str(Arc::from("neo")),
                },
                Instr::ObjSet {
                    obj: Slot::Local(0),
                    key: "name".to_owned(),
                    value: Slot::Local(1),
                    out: Slot::Local(0),
                },
                Instr::StoreConst {
                    slot: Slot::Local(2),
                    value: ConstValue::Str(Arc::from("name")),
                },
                Instr::ObjHas {
                    obj: Slot::Local(0),
                    key: Slot::Local(2),
                    out: Slot::Local(3),
                },
                Instr::ObjGet {
                    obj: Slot::Local(0),
                    key: Slot::Local(2),
                    out: Slot::Local(4),
                },
                Instr::StoreConst {
                    slot: Slot::Local(5),
                    value: ConstValue::Str(Arc::from("!")),
                },
                Instr::StrConcat {
                    a: Slot::Local(4),
                    b: Slot::Local(5),
                    out: Slot::Local(6),
                },
                Instr::StrLen {
                    value: Slot::Local(6),
                    out: Slot::Ret(0),
                },
                Instr::Exit,
            ]),
            local_count: 7,
            arg_count: 0,
            ret_count: 1,
            err_count: 1,
            meta: scalar_meta("main"),
        };

        let module = CompiledModule {
            name: Arc::from("main"),
            init_func: 0,
            functions: vec![function],
            function_globals: vec![],
            exports: vec![],
            imports: vec![],
            global_count: 0,
        };

        for enable_jit in [true, false] {
            let mut vm = Vm::new(VmConfig {
                enable_host_print: false,
                enable_jit,
            });
            let result = vm.run_main(&module).expect("run");
            assert_eq!(result.returns, vec![Value::Num(4.0)]);
        }
    }

    #[test]
    fn stdlib_prelude_module_runs() {
        let prelude = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../stdlib/prelude.imp")
            .canonicalize()
            .expect("canonicalize prelude path");

        let program = format!(
            r#"#call core::import alias="std" path="{}";
#call core::const out=local::x value=-3;
#call std::abs args="local::x" out=local::absx;
#call core::const out=local::low value=0;
#call core::const out=local::high value=2;
#call std::clamp args="local::absx,local::low,local::high" out=local::clamped;
#call core::mov from=local::clamped to=return::value;
#call core::exit;
"#,
            prelude.display()
        );

        let main_path = std::env::temp_dir().join("imp_stdlib_prelude_test.imp");
        fs::write(&main_path, program).expect("write main");

        let module = compile_module(&main_path, &FsModuleLoader).expect("compile module");
        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Num(2.0)]);
    }

    #[test]
    fn namespaced_stdlib_modules_run_together() {
        let stdlib_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../stdlib")
            .canonicalize()
            .expect("canonicalize stdlib root");
        let map = stdlib_root.join("map.imp");
        let string = stdlib_root.join("string.imp");
        let result_mod = stdlib_root.join("result.imp");

        let program = format!(
            r#"#call core::import alias="std_map" path="{}";
#call core::import alias="std_str" path="{}";
#call core::import alias="std_res" path="{}";

#call std_map::new out=local::m;
#call core::const out=local::name value="imp";
#call core::obj::set obj=local::m key="name" value=local::name out=local::m;
#call core::const out=local::key value="name";
#call core::const out=local::msg value="missing name";
#call std_map::require args="local::m,local::key,local::msg" out=local::got;
#call core::const out=local::suffix value="!";
#call std_str::concat args="local::got,local::suffix" out=local::text;
#call std_res::ok args="local::text" out=local::r;
#call core::const out=local::fallback value="fallback";
#call std_res::unwrap_or args="local::r,local::fallback" out=return::value;
#call core::exit;
"#,
            map.display(),
            string.display(),
            result_mod.display()
        );

        let main_path = std::env::temp_dir().join("imp_stdlib_namespaced_test.imp");
        fs::write(&main_path, program).expect("write main");

        let module = compile_module(&main_path, &FsModuleLoader).expect("compile module");
        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&module).expect("run");
        assert_eq!(result.returns, vec![Value::Str(Arc::from("imp!"))]);
    }
}
