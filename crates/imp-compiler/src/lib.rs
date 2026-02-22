use imp_ast::{Atom, Call, Program, RefPath, parse_program};
use imp_ir::{
    CompiledFunction, CompiledModule, ConstValue, FnMeta, FuncId, ImportBinding, Instr, RetShape,
    Slot,
};
use imp_std::{ANNO_SAFE, is_core_target, parse_csv};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CompileOpts {
    pub module_name: String,
}

impl Default for CompileOpts {
    fn default() -> Self {
        Self {
            module_name: "main".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub line: usize,
    pub message: String,
}

impl CompileError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for CompileError {}

pub trait ModuleLoader {
    fn load(&self, path: &Path) -> Result<String, CompileError>;
    fn normalize(&self, path: &Path) -> Result<PathBuf, CompileError>;
}

pub struct FsModuleLoader;

impl ModuleLoader for FsModuleLoader {
    fn load(&self, path: &Path) -> Result<String, CompileError> {
        fs::read_to_string(path).map_err(|err| {
            CompileError::new(1, format!("failed to read {}: {err}", path.display()))
        })
    }

    fn normalize(&self, path: &Path) -> Result<PathBuf, CompileError> {
        path.canonicalize().map_err(|err| {
            CompileError::new(
                1,
                format!("failed to canonicalize {}: {err}", path.display()),
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub module: CompiledModule,
}

pub fn compile_program(src: &str, opts: CompileOpts) -> Result<CompiledProgram, CompileError> {
    let program = parse_program(src).map_err(|err| CompileError::new(err.line, err.message))?;
    let mut cache = HashMap::new();
    let mut visiting = HashSet::new();
    let module = compile_source_internal(
        &program,
        opts.module_name,
        None,
        &NoopLoader,
        &mut cache,
        &mut visiting,
    )?;
    Ok(CompiledProgram { module })
}

pub fn compile_module(
    path: &Path,
    loader: &dyn ModuleLoader,
) -> Result<CompiledModule, CompileError> {
    let mut cache = HashMap::new();
    let mut visiting = HashSet::new();
    compile_module_internal(path, loader, &mut cache, &mut visiting)
}

fn compile_module_internal(
    path: &Path,
    loader: &dyn ModuleLoader,
    cache: &mut HashMap<PathBuf, CompiledModule>,
    visiting: &mut HashSet<PathBuf>,
) -> Result<CompiledModule, CompileError> {
    let canonical = loader.normalize(path)?;
    if let Some(module) = cache.get(&canonical) {
        return Ok(module.clone());
    }
    if visiting.contains(&canonical) {
        return Err(CompileError::new(
            1,
            format!("cyclic import detected at {}", canonical.display()),
        ));
    }

    visiting.insert(canonical.clone());
    let src = loader.load(&canonical)?;
    let program = parse_program(&src).map_err(|err| CompileError::new(err.line, err.message))?;
    let module_name = canonical
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_owned();

    let module = compile_source_internal(
        &program,
        module_name,
        Some(canonical.as_path()),
        loader,
        cache,
        visiting,
    )?;

    visiting.remove(&canonical);
    cache.insert(canonical, module.clone());
    Ok(module)
}

struct NoopLoader;

impl ModuleLoader for NoopLoader {
    fn load(&self, _path: &Path) -> Result<String, CompileError> {
        Err(CompileError::new(
            1,
            "import is unavailable in compile_program(); use compile_module()",
        ))
    }

    fn normalize(&self, path: &Path) -> Result<PathBuf, CompileError> {
        Ok(path.to_path_buf())
    }
}

#[derive(Debug, Clone)]
struct FunctionAst {
    name: RefPath,
    args: Vec<String>,
    retshape: RetShape,
    ret_count: u32,
    body: Vec<Call>,
    line: usize,
}

fn compile_source_internal(
    program: &Program,
    module_name: String,
    module_path: Option<&Path>,
    loader: &dyn ModuleLoader,
    cache: &mut HashMap<PathBuf, CompiledModule>,
    visiting: &mut HashSet<PathBuf>,
) -> Result<CompiledModule, CompileError> {
    let expanded = expand_macros(&program.calls)?;
    let (top_level, functions) = split_functions(&expanded)?;

    let mut builder = ModuleBuilder::new(module_name);

    let mut compiled_functions = Vec::new();
    let mut function_globals = Vec::new();

    // Reserve function IDs by compile order; init function is always id 0.
    let mut next_func_id: FuncId = 1;
    for function_ast in &functions {
        let global_slot =
            builder.resolve_global(&function_ast.name.namespace, &function_ast.name.name);
        let func_id = next_func_id;
        next_func_id += 1;
        function_globals.push((global_slot, func_id));
        compiled_functions.push(compile_function(function_ast, func_id, &mut builder)?);
    }

    let imports = compile_imports(
        &top_level,
        module_path,
        loader,
        cache,
        visiting,
        &mut builder,
    )?;
    let exports = collect_exports(&top_level, &mut builder)?;
    let init_body = filter_meta_calls(&top_level);

    let init_func = compile_raw_function(
        &init_body,
        0,
        "<init>",
        Vec::new(),
        RetShape::Any,
        0,
        &mut builder,
        1,
    )?;

    let mut functions_all = Vec::with_capacity(compiled_functions.len() + 1);
    functions_all.push(init_func);
    functions_all.extend(compiled_functions);

    Ok(CompiledModule {
        name: Arc::from(builder.module_name.as_str()),
        init_func: 0,
        functions: functions_all,
        function_globals,
        exports,
        imports,
        global_count: builder.next_global,
    })
}

fn filter_meta_calls(calls: &[Call]) -> Vec<Call> {
    calls
        .iter()
        .filter(|call| {
            call.target != "core::import"
                && call.target != "core::mod::export"
                && call.target != "core::fn::begin"
                && call.target != "core::fn::end"
        })
        .cloned()
        .collect()
}

fn compile_imports(
    calls: &[Call],
    module_path: Option<&Path>,
    loader: &dyn ModuleLoader,
    cache: &mut HashMap<PathBuf, CompiledModule>,
    visiting: &mut HashSet<PathBuf>,
    builder: &mut ModuleBuilder,
) -> Result<Vec<ImportBinding>, CompileError> {
    let mut imports = Vec::new();

    for call in calls {
        if call.target != "core::import" {
            continue;
        }

        let alias = get_string_arg(call, "alias")?;
        let path_raw = get_string_arg(call, "path")?;
        let import_path = resolve_import_path(module_path, Path::new(&path_raw));
        let imported_module = compile_module_internal(&import_path, loader, cache, visiting)?;

        let mut export_to_global = Vec::new();
        for (name, _) in &imported_module.exports {
            let destination = builder.resolve_global(&alias, name);
            export_to_global.push((name.clone(), destination));
        }

        imports.push(ImportBinding {
            path: import_path.to_string_lossy().to_string(),
            alias,
            module: Arc::new(imported_module),
            export_to_global,
        });
    }

    Ok(imports)
}

fn resolve_import_path(module_path: Option<&Path>, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(module_path) = module_path
        && let Some(parent) = module_path.parent()
    {
        return parent.join(path);
    }

    path.to_path_buf()
}

fn collect_exports(
    calls: &[Call],
    builder: &mut ModuleBuilder,
) -> Result<Vec<(String, u32)>, CompileError> {
    let mut exports = Vec::new();
    for call in calls {
        if call.target != "core::mod::export" {
            continue;
        }
        let name = get_string_arg(call, "name")?;
        let value_ref = get_ref_arg(call, "value")?;
        let slot = builder.resolve_global(&value_ref.namespace, &value_ref.name);
        exports.push((name, slot));
    }
    Ok(exports)
}

fn split_functions(calls: &[Call]) -> Result<(Vec<Call>, Vec<FunctionAst>), CompileError> {
    let mut top_level = Vec::new();
    let mut functions = Vec::new();

    let mut in_function = false;
    let mut current: Option<FunctionAst> = None;

    for call in calls {
        match call.target.as_str() {
            "core::fn::begin" => {
                if in_function {
                    return Err(CompileError::new(
                        call.line,
                        "nested functions are not allowed",
                    ));
                }
                in_function = true;
                current = Some(FunctionAst {
                    name: get_ref_arg(call, "name")?,
                    args: parse_csv(&get_string_arg(call, "args").unwrap_or_default()),
                    retshape: parse_retshape(
                        call.arg("retshape").and_then(atom_as_str).unwrap_or("any"),
                    ),
                    ret_count: call
                        .arg("retcount")
                        .and_then(atom_as_number)
                        .map(|v| v as u32)
                        .unwrap_or(1),
                    body: Vec::new(),
                    line: call.line,
                });
            }
            "core::fn::end" => {
                if !in_function {
                    return Err(CompileError::new(
                        call.line,
                        "core::fn::end without core::fn::begin",
                    ));
                }
                in_function = false;
                if let Some(function) = current.take() {
                    functions.push(function);
                }
            }
            _ => {
                if in_function {
                    if let Some(function) = current.as_mut() {
                        function.body.push(call.clone());
                    }
                } else {
                    top_level.push(call.clone());
                }
            }
        }
    }

    if in_function {
        let line = current.as_ref().map_or(1, |f| f.line);
        return Err(CompileError::new(line, "unclosed core::fn::begin block"));
    }

    Ok((top_level, functions))
}

fn compile_function(
    function_ast: &FunctionAst,
    func_id: FuncId,
    builder: &mut ModuleBuilder,
) -> Result<CompiledFunction, CompileError> {
    compile_raw_function(
        &function_ast.body,
        func_id,
        &format!(
            "{}::{}",
            function_ast.name.namespace, function_ast.name.name
        ),
        function_ast.args.clone(),
        function_ast.retshape.clone(),
        function_ast.ret_count,
        builder,
        function_ast.line,
    )
}

fn compile_raw_function(
    calls: &[Call],
    func_id: FuncId,
    name: &str,
    args: Vec<String>,
    retshape: RetShape,
    ret_count: u32,
    builder: &mut ModuleBuilder,
    default_line: usize,
) -> Result<CompiledFunction, CompileError> {
    let mut env = SlotEnv::new(args, ret_count);
    let mut code = Vec::new();
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut pending_jumps = Vec::new();
    let mut pending_branches = Vec::new();
    let mut pending_try = Vec::new();

    for call in calls {
        lower_call(
            call,
            &mut env,
            builder,
            &mut code,
            &mut labels,
            &mut pending_jumps,
            &mut pending_branches,
            &mut pending_try,
        )?;
    }

    if !matches!(code.last(), Some(Instr::Exit)) {
        code.push(Instr::Exit);
    }

    for (pc, label) in pending_jumps {
        let target = labels
            .get(&label)
            .copied()
            .ok_or_else(|| CompileError::new(default_line, format!("unknown label '{label}'")))?;
        if let Some(Instr::Jump {
            target: jump_target,
        }) = code.get_mut(pc)
        {
            *jump_target = target;
        }
    }

    for (pc, then_label, else_label) in pending_branches {
        let then_pc = labels.get(&then_label).copied().ok_or_else(|| {
            CompileError::new(default_line, format!("unknown label '{then_label}'"))
        })?;
        let else_pc = labels.get(&else_label).copied().ok_or_else(|| {
            CompileError::new(default_line, format!("unknown label '{else_label}'"))
        })?;
        if let Some(Instr::Branch {
            then_pc: branch_then,
            else_pc: branch_else,
            ..
        }) = code.get_mut(pc)
        {
            *branch_then = then_pc;
            *branch_else = else_pc;
        }
    }

    for (pc, label) in pending_try {
        let handler_pc = labels
            .get(&label)
            .copied()
            .ok_or_else(|| CompileError::new(default_line, format!("unknown label '{label}'")))?;
        if let Some(Instr::TryPush { handler_pc: target }) = code.get_mut(pc) {
            *target = handler_pc;
        }
    }

    Ok(CompiledFunction {
        id: func_id,
        code: code.into(),
        local_count: env.next_local,
        arg_count: env.args.len() as u32,
        ret_count,
        err_count: env.next_err,
        meta: FnMeta {
            name: Arc::from(name),
            arg_count: env.args.len() as u32,
            ret_count,
            retshape,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_call(
    call: &Call,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
    code: &mut Vec<Instr>,
    labels: &mut HashMap<String, usize>,
    pending_jumps: &mut Vec<(usize, String)>,
    pending_branches: &mut Vec<(usize, String, String)>,
    pending_try: &mut Vec<(usize, String)>,
) -> Result<(), CompileError> {
    if !is_core_target(&call.target) {
        let fn_slot = resolve_target_ref(call, env, builder)?;
        let mut args = collect_invoke_args(call, env, builder)?;
        let out = call
            .arg("out")
            .map(|atom| resolve_ref_atom(atom, env, builder, call.line))
            .transpose()?
            .unwrap_or_else(|| env.resolve_local("_invoke_out"));
        code.push(Instr::Invoke {
            fn_slot,
            args: std::mem::take(&mut args),
            out,
        });
        return Ok(());
    }

    match call.target.as_str() {
        "core::const" => {
            let out = resolve_ref_atom(
                call.arg("out")
                    .ok_or_else(|| CompileError::new(call.line, "core::const missing out"))?,
                env,
                builder,
                call.line,
            )?;
            let value = lower_const(
                call.arg("value")
                    .ok_or_else(|| CompileError::new(call.line, "core::const missing value"))?,
                call.line,
            )?;
            code.push(Instr::StoreConst { slot: out, value });
        }
        "core::mov" => {
            let from = resolve_named_ref(call, "from", env, builder)?;
            let to = resolve_named_ref(call, "to", env, builder)?;
            code.push(Instr::Move { from, to });
        }
        "core::add" | "core::sub" | "core::mul" | "core::div" | "core::eq" | "core::lt" => {
            let a = resolve_named_ref(call, "a", env, builder)?;
            let b = resolve_named_ref(call, "b", env, builder)?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            let instr = match call.target.as_str() {
                "core::add" => Instr::Add { a, b, out },
                "core::sub" => Instr::Sub { a, b, out },
                "core::mul" => Instr::Mul { a, b, out },
                "core::div" => Instr::Div { a, b, out },
                "core::eq" => Instr::Eq { a, b, out },
                _ => Instr::Lt { a, b, out },
            };
            code.push(instr);
        }
        "core::label" => {
            let name = get_string_arg(call, "name")?;
            labels.insert(name, code.len());
        }
        "core::jump" => {
            let target_label = get_string_arg(call, "target")?;
            let pc = code.len();
            code.push(Instr::Jump { target: 0 });
            pending_jumps.push((pc, target_label));
        }
        "core::br" => {
            let cond = resolve_named_ref(call, "cond", env, builder)?;
            let then_label = get_string_arg(call, "then")?;
            let else_label = get_string_arg(call, "else")?;
            let pc = code.len();
            code.push(Instr::Branch {
                cond,
                then_pc: 0,
                else_pc: 0,
            });
            pending_branches.push((pc, then_label, else_label));
        }
        "core::invoke" => {
            let fn_slot = resolve_named_ref(call, "fn", env, builder)?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            let args = collect_invoke_args(call, env, builder)?;
            code.push(Instr::Invoke { fn_slot, args, out });
        }
        "core::ret::set" => {
            let slot_id = call.arg("slot").and_then(atom_as_number).ok_or_else(|| {
                CompileError::new(call.line, "core::ret::set requires numeric slot")
            })? as u32;
            let value = resolve_named_ref(call, "value", env, builder)?;
            code.push(Instr::ReturnSet { slot_id, value });
        }
        "core::exit" => {
            code.push(Instr::Exit);
        }
        "core::throw" => {
            let code_text = get_string_arg(call, "code")?;
            let msg = get_string_arg(call, "msg")?;
            code.push(Instr::Throw {
                code: code_text,
                msg,
            });
        }
        "core::try::push" => {
            let handler_label = get_string_arg(call, "handler")?;
            let pc = code.len();
            code.push(Instr::TryPush { handler_pc: 0 });
            pending_try.push((pc, handler_label));
        }
        "core::try::pop" => {
            code.push(Instr::TryPop);
        }
        "core::obj::new" => {
            let out = resolve_named_ref(call, "out", env, builder)?;
            code.push(Instr::ObjNew { out });
        }
        "core::obj::set" => {
            let obj = resolve_named_ref(call, "obj", env, builder)?;
            let key = get_string_arg(call, "key")?;
            let value = resolve_named_ref(call, "value", env, builder)?;
            let out = call
                .arg("out")
                .map(|atom| resolve_ref_atom(atom, env, builder, call.line))
                .transpose()?
                .unwrap_or(obj);
            code.push(Instr::ObjSet {
                obj,
                key,
                value,
                out,
            });
        }
        "core::obj::get" => {
            let obj = resolve_named_ref(call, "obj", env, builder)?;
            let key = resolve_atom_to_slot(
                call.arg("key")
                    .ok_or_else(|| CompileError::new(call.line, "core::obj::get missing key"))?,
                env,
                builder,
                code,
                call.line,
            )?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            code.push(Instr::ObjGet { obj, key, out });
        }
        "core::obj::has" => {
            let obj = resolve_named_ref(call, "obj", env, builder)?;
            let key = resolve_atom_to_slot(
                call.arg("key")
                    .ok_or_else(|| CompileError::new(call.line, "core::obj::has missing key"))?,
                env,
                builder,
                code,
                call.line,
            )?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            code.push(Instr::ObjHas { obj, key, out });
        }
        "core::str::concat" => {
            let a = resolve_atom_to_slot(
                call.arg("a")
                    .ok_or_else(|| CompileError::new(call.line, "core::str::concat missing a"))?,
                env,
                builder,
                code,
                call.line,
            )?;
            let b = resolve_atom_to_slot(
                call.arg("b")
                    .ok_or_else(|| CompileError::new(call.line, "core::str::concat missing b"))?,
                env,
                builder,
                code,
                call.line,
            )?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            code.push(Instr::StrConcat { a, b, out });
        }
        "core::str::len" => {
            let value = resolve_atom_to_slot(
                call.arg("value")
                    .ok_or_else(|| CompileError::new(call.line, "core::str::len missing value"))?,
                env,
                builder,
                code,
                call.line,
            )?;
            let out = resolve_named_ref(call, "out", env, builder)?;
            code.push(Instr::StrLen { value, out });
        }
        "core::host::print" => {
            let slot = call
                .arg("slot")
                .or_else(|| call.arg("value"))
                .ok_or_else(|| {
                    CompileError::new(call.line, "core::host::print missing slot/value")
                })?;
            let slot = resolve_ref_atom(slot, env, builder, call.line)?;
            code.push(Instr::HostPrint { slot });
        }
        "core::import" | "core::mod::export" => {
            // Handled in metadata pass.
        }
        "core::fn::begin" | "core::fn::end" => {
            return Err(CompileError::new(
                call.line,
                "function declarations are not valid inside lowered body",
            ));
        }
        other => {
            return Err(CompileError::new(
                call.line,
                format!("unsupported core target '{other}'"),
            ));
        }
    }

    Ok(())
}

fn lower_const(atom: &Atom, line: usize) -> Result<ConstValue, CompileError> {
    match atom {
        Atom::Null => Ok(ConstValue::Null),
        Atom::Bool(value) => Ok(ConstValue::Bool(*value)),
        Atom::Num(value) => Ok(ConstValue::Num(*value)),
        Atom::Str(value) => Ok(ConstValue::Str(Arc::from(value.as_str()))),
        Atom::Ref(_) => Err(CompileError::new(
            line,
            "core::const value cannot be a ref; use core::mov",
        )),
    }
}

fn resolve_target_ref(
    call: &Call,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
) -> Result<Slot, CompileError> {
    let target = RefPath::parse(&call.target).ok_or_else(|| {
        CompileError::new(
            call.line,
            format!("non-core target '{}' must be namespace::name", call.target),
        )
    })?;
    Ok(env.resolve_ref(&target, builder))
}

fn resolve_named_ref(
    call: &Call,
    key: &str,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
) -> Result<Slot, CompileError> {
    let atom = call
        .arg(key)
        .ok_or_else(|| CompileError::new(call.line, format!("{} missing {key}", call.target)))?;
    resolve_ref_atom(atom, env, builder, call.line)
}

fn resolve_ref_atom(
    atom: &Atom,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
    line: usize,
) -> Result<Slot, CompileError> {
    if let Atom::Ref(path) = atom {
        Ok(env.resolve_ref(path, builder))
    } else {
        Err(CompileError::new(line, "expected ref atom"))
    }
}

fn resolve_atom_to_slot(
    atom: &Atom,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
    code: &mut Vec<Instr>,
    line: usize,
) -> Result<Slot, CompileError> {
    match atom {
        Atom::Ref(path) => Ok(env.resolve_ref(path, builder)),
        Atom::Null | Atom::Bool(_) | Atom::Num(_) | Atom::Str(_) => {
            let slot = env.resolve_temp_local("const");
            let value = lower_const(atom, line)?;
            code.push(Instr::StoreConst { slot, value });
            Ok(slot)
        }
    }
}

fn collect_invoke_args(
    call: &Call,
    env: &mut SlotEnv,
    builder: &mut ModuleBuilder,
) -> Result<Vec<Slot>, CompileError> {
    if let Some(args_csv) = call.arg("args").and_then(atom_as_str) {
        let mut out = Vec::new();
        for item in parse_csv(args_csv) {
            let path = RefPath::parse(&item).ok_or_else(|| {
                CompileError::new(call.line, format!("invalid invoke arg ref '{item}'"))
            })?;
            out.push(env.resolve_ref(&path, builder));
        }
        return Ok(out);
    }

    let mut arg_pairs = call
        .args
        .iter()
        .filter(|arg| arg.key.starts_with("arg"))
        .collect::<Vec<_>>();
    arg_pairs.sort_by(|a, b| a.key.cmp(&b.key));

    let mut out = Vec::new();
    for arg in arg_pairs {
        out.push(resolve_ref_atom(&arg.value, env, builder, call.line)?);
    }
    Ok(out)
}

fn parse_retshape(raw: &str) -> RetShape {
    if raw.eq_ignore_ascii_case("scalar") {
        return RetShape::Scalar;
    }
    if raw.eq_ignore_ascii_case("any") {
        return RetShape::Any;
    }
    if let Some(inner) = raw
        .strip_prefix("either(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return RetShape::Either(parse_csv(inner));
    }
    if let Some(inner) = raw
        .strip_prefix("record(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return RetShape::Record(parse_csv(inner));
    }
    RetShape::Any
}

fn get_string_arg(call: &Call, key: &str) -> Result<String, CompileError> {
    call.arg(key)
        .and_then(atom_as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            CompileError::new(
                call.line,
                format!("{} missing string arg {key}", call.target),
            )
        })
}

fn get_ref_arg(call: &Call, key: &str) -> Result<RefPath, CompileError> {
    match call.arg(key) {
        Some(Atom::Ref(path)) => Ok(path.clone()),
        _ => Err(CompileError::new(
            call.line,
            format!("{} missing ref arg {key}", call.target),
        )),
    }
}

fn atom_as_str(atom: &Atom) -> Option<&str> {
    if let Atom::Str(value) = atom {
        Some(value.as_str())
    } else {
        None
    }
}

fn atom_as_number(atom: &Atom) -> Option<f64> {
    if let Atom::Num(value) = atom {
        Some(*value)
    } else {
        None
    }
}

fn expand_macros(calls: &[Call]) -> Result<Vec<Call>, CompileError> {
    let mut output = Vec::new();
    let mut safe_counter = 0usize;

    for call in calls {
        if !call.annos.iter().any(|anno| anno == ANNO_SAFE) {
            output.push(call.clone());
            continue;
        }

        if call.target != "core::div" {
            let mut cloned = call.clone();
            cloned.annos.clear();
            output.push(cloned);
            continue;
        }

        let out_ref = call
            .arg("out")
            .and_then(|atom| {
                if let Atom::Ref(path) = atom {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| CompileError::new(call.line, "@safe core::div requires out=<ref>"))?;

        let handler = format!("__safe_handler_{safe_counter}");
        let end = format!("__safe_end_{safe_counter}");
        safe_counter += 1;

        output.push(Call {
            annos: Vec::new(),
            target: "core::try::push".to_owned(),
            args: vec![imp_ast::Arg {
                key: "handler".to_owned(),
                value: Atom::Str(handler.clone()),
            }],
            line: call.line,
        });

        let mut div = call.clone();
        div.annos.clear();
        output.push(div);

        output.push(Call {
            annos: Vec::new(),
            target: "core::jump".to_owned(),
            args: vec![imp_ast::Arg {
                key: "target".to_owned(),
                value: Atom::Str(end.clone()),
            }],
            line: call.line,
        });

        output.push(Call {
            annos: Vec::new(),
            target: "core::label".to_owned(),
            args: vec![imp_ast::Arg {
                key: "name".to_owned(),
                value: Atom::Str(handler),
            }],
            line: call.line,
        });

        output.push(Call {
            annos: Vec::new(),
            target: "core::const".to_owned(),
            args: vec![
                imp_ast::Arg {
                    key: "out".to_owned(),
                    value: Atom::Ref(out_ref),
                },
                imp_ast::Arg {
                    key: "value".to_owned(),
                    value: Atom::Null,
                },
            ],
            line: call.line,
        });

        output.push(Call {
            annos: Vec::new(),
            target: "core::label".to_owned(),
            args: vec![imp_ast::Arg {
                key: "name".to_owned(),
                value: Atom::Str(end),
            }],
            line: call.line,
        });

        output.push(Call {
            annos: Vec::new(),
            target: "core::try::pop".to_owned(),
            args: Vec::new(),
            line: call.line,
        });
    }

    Ok(output)
}

struct ModuleBuilder {
    module_name: String,
    globals: HashMap<String, u32>,
    next_global: u32,
}

impl ModuleBuilder {
    fn new(module_name: String) -> Self {
        Self {
            module_name,
            globals: HashMap::new(),
            next_global: 0,
        }
    }

    fn resolve_global(&mut self, namespace: &str, name: &str) -> u32 {
        let key = format!("{namespace}::{name}");
        if let Some(existing) = self.globals.get(&key) {
            return *existing;
        }
        let slot = self.next_global;
        self.next_global += 1;
        self.globals.insert(key, slot);
        slot
    }
}

struct SlotEnv {
    locals: HashMap<String, u32>,
    args: HashMap<String, u32>,
    returns: HashMap<String, u32>,
    errors: HashMap<String, u32>,
    next_local: u32,
    next_err: u32,
    temp_counter: u32,
}

impl SlotEnv {
    fn new(args: Vec<String>, ret_count: u32) -> Self {
        let mut args_map = HashMap::new();
        for (index, name) in args.into_iter().enumerate() {
            args_map.insert(name, index as u32);
        }

        let mut returns = HashMap::new();
        for i in 0..ret_count {
            returns.insert(format!("{i}"), i);
            returns.insert("value".to_owned(), 0);
        }

        Self {
            locals: HashMap::new(),
            args: args_map,
            returns,
            errors: HashMap::new(),
            next_local: 0,
            next_err: 0,
            temp_counter: 0,
        }
    }

    fn resolve_local(&mut self, name: &str) -> Slot {
        if let Some(slot) = self.locals.get(name) {
            return Slot::Local(*slot);
        }
        let slot = self.next_local;
        self.next_local += 1;
        self.locals.insert(name.to_owned(), slot);
        Slot::Local(slot)
    }

    fn resolve_temp_local(&mut self, prefix: &str) -> Slot {
        let name = format!("__tmp_{prefix}_{}", self.temp_counter);
        self.temp_counter += 1;
        self.resolve_local(&name)
    }

    fn resolve_ref(&mut self, path: &RefPath, builder: &mut ModuleBuilder) -> Slot {
        match path.namespace.as_str() {
            "local" => self.resolve_local(&path.name),
            "arg" => {
                let slot = if let Some(slot) = self.args.get(&path.name) {
                    *slot
                } else {
                    let index = self.args.len() as u32;
                    self.args.insert(path.name.clone(), index);
                    index
                };
                Slot::Arg(slot)
            }
            "return" => {
                let slot = if let Some(slot) = self.returns.get(&path.name) {
                    *slot
                } else {
                    let index = self.returns.len() as u32;
                    self.returns.insert(path.name.clone(), index);
                    index
                };
                Slot::Ret(slot)
            }
            "err" => {
                let slot = *self.errors.entry(path.name.clone()).or_insert_with(|| {
                    let index = self.next_err;
                    self.next_err += 1;
                    index
                });
                Slot::Err(slot)
            }
            namespace => Slot::Global(builder.resolve_global(namespace, &path.name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_basic_add_program() {
        let src = r#"
#call core::const out=local::x value=2;
#call core::const out=local::y value=3;
#call core::add a=local::x b=local::y out=return::value;
#call core::exit;
"#;
        let compiled = compile_program(src, CompileOpts::default()).expect("compile");
        let init = compiled.module.function(0).expect("init");
        assert!(!init.code.is_empty());
    }

    #[test]
    fn safe_anno_expands() {
        let src = r#"
#call @safe core::div a=local::a b=local::b out=local::c;
#call core::exit;
"#;
        let compiled = compile_program(src, CompileOpts::default()).expect("compile");
        let init = compiled.module.function(0).expect("init");
        assert!(
            init.code
                .iter()
                .any(|instr| matches!(instr, Instr::TryPush { .. }))
        );
    }

    #[test]
    fn labels_are_patched_to_pc() {
        let src = r#"
#call core::const out=local::flag value=true;
#call core::br cond=local::flag then="yes" else="no";
#call core::label name="yes";
#call core::const out=return::value value=1;
#call core::jump target="end";
#call core::label name="no";
#call core::const out=return::value value=2;
#call core::label name="end";
#call core::exit;
"#;

        let compiled = compile_program(src, CompileOpts::default()).expect("compile");
        let init = compiled.module.function(0).expect("init");

        assert!(init.code.iter().any(|instr| {
            matches!(
                instr,
                Instr::Branch {
                    then_pc,
                    else_pc,
                    ..
                } if *then_pc > 0 && *else_pc > 0
            )
        }));
    }

    #[test]
    fn compiles_module_imports() {
        let root = std::env::temp_dir().join("imp_compiler_import_test");
        let _ = std::fs::create_dir_all(&root);
        let dep = root.join("dep.imp");
        let main = root.join("main.imp");

        std::fs::write(
            &dep,
            r#"#call core::const out=main::x value=5;
#call core::mod::export name="x" value=main::x;
#call core::exit;
"#,
        )
        .expect("write dep");

        std::fs::write(
            &main,
            format!(
                r#"#call core::import alias="dep" path="{}";
#call core::mov from=dep::x to=return::value;
#call core::exit;
"#,
                dep.display()
            ),
        )
        .expect("write main");

        let module = compile_module(&main, &FsModuleLoader).expect("compile module");
        assert!(!module.imports.is_empty());
    }

    #[test]
    fn lowers_new_stdlib_enabler_targets() {
        let src = r#"
#call core::obj::new out=local::m;
#call core::const out=local::k value="name";
#call core::obj::has obj=local::m key=local::k out=local::has_name;
#call core::obj::get obj=local::m key=local::k out=local::value;
#call core::str::concat a="hi " b="there" out=local::joined;
#call core::str::len value=local::joined out=return::value;
#call core::exit;
"#;

        let compiled = compile_program(src, CompileOpts::default()).expect("compile");
        let init = compiled.module.function(0).expect("init");

        assert!(
            init.code
                .iter()
                .any(|instr| matches!(instr, Instr::ObjHas { .. }))
        );
        assert!(
            init.code
                .iter()
                .any(|instr| matches!(instr, Instr::ObjGet { .. }))
        );
        assert!(
            init.code
                .iter()
                .any(|instr| matches!(instr, Instr::StrConcat { .. }))
        );
        assert!(
            init.code
                .iter()
                .any(|instr| matches!(instr, Instr::StrLen { .. }))
        );
    }
}
