use imp_ir::{
    CompiledFunction, CompiledModule, ConstValue, FnMeta, ImportBinding, Instr, RetShape, Slot,
};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

const MAGIC: [u8; 4] = *b"IMPC";
const VERSION: u16 = 1;

#[derive(Debug)]
pub enum BytecodeError {
    Io(io::Error),
    UnexpectedEof,
    InvalidMagic([u8; 4]),
    UnsupportedVersion(u16),
    InvalidUtf8(String),
    InvalidTag { kind: &'static str, tag: u8 },
    Overflow(&'static str),
}

impl fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::UnexpectedEof => write!(f, "bytecode unexpectedly ended"),
            Self::InvalidMagic(magic) => write!(f, "invalid bytecode magic: {magic:?}"),
            Self::UnsupportedVersion(version) => {
                write!(
                    f,
                    "unsupported bytecode version {version} (expected {VERSION})"
                )
            }
            Self::InvalidUtf8(ctx) => write!(f, "invalid utf8 for {ctx}"),
            Self::InvalidTag { kind, tag } => write!(f, "invalid {kind} tag {tag}"),
            Self::Overflow(ctx) => write!(f, "value overflow while encoding/decoding {ctx}"),
        }
    }
}

impl std::error::Error for BytecodeError {}

impl From<io::Error> for BytecodeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn encode_module(module: &CompiledModule) -> Result<Vec<u8>, BytecodeError> {
    let mut w = Writer::default();
    w.write_bytes(&MAGIC);
    w.write_u16(VERSION);
    write_module(&mut w, module)?;
    Ok(w.finish())
}

pub fn decode_module(bytes: &[u8]) -> Result<CompiledModule, BytecodeError> {
    let mut r = Reader::new(bytes);
    let magic = r.read_fixed_4()?;
    if magic != MAGIC {
        return Err(BytecodeError::InvalidMagic(magic));
    }
    let version = r.read_u16()?;
    if version != VERSION {
        return Err(BytecodeError::UnsupportedVersion(version));
    }
    let module = read_module(&mut r)?;
    if !r.is_eof() {
        return Err(BytecodeError::InvalidTag {
            kind: "trailing-bytes",
            tag: 0,
        });
    }
    Ok(module)
}

pub fn encode_to_path(path: &Path, module: &CompiledModule) -> Result<(), BytecodeError> {
    let encoded = encode_module(module)?;
    fs::write(path, encoded)?;
    Ok(())
}

pub fn decode_from_path(path: &Path) -> Result<CompiledModule, BytecodeError> {
    let bytes = fs::read(path)?;
    decode_module(&bytes)
}

fn write_module(w: &mut Writer, module: &CompiledModule) -> Result<(), BytecodeError> {
    w.write_string(module.name.as_ref())?;
    w.write_u32(module.init_func);
    w.write_len(module.functions.len(), "functions length")?;
    for function in &module.functions {
        write_function(w, function)?;
    }
    w.write_len(module.function_globals.len(), "function_globals length")?;
    for (slot, func) in &module.function_globals {
        w.write_u32(*slot);
        w.write_u32(*func);
    }
    w.write_len(module.exports.len(), "exports length")?;
    for (name, slot) in &module.exports {
        w.write_string(name)?;
        w.write_u32(*slot);
    }
    w.write_len(module.imports.len(), "imports length")?;
    for import in &module.imports {
        write_import(w, import)?;
    }
    w.write_u32(module.global_count);
    Ok(())
}

fn read_module(r: &mut Reader<'_>) -> Result<CompiledModule, BytecodeError> {
    let name = Arc::<str>::from(r.read_string("module.name")?.as_str());
    let init_func = r.read_u32()?;
    let function_count = r.read_len("functions length")?;
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        functions.push(read_function(r)?);
    }
    let function_global_count = r.read_len("function_globals length")?;
    let mut function_globals = Vec::with_capacity(function_global_count);
    for _ in 0..function_global_count {
        function_globals.push((r.read_u32()?, r.read_u32()?));
    }
    let export_count = r.read_len("exports length")?;
    let mut exports = Vec::with_capacity(export_count);
    for _ in 0..export_count {
        exports.push((r.read_string("export name")?, r.read_u32()?));
    }
    let import_count = r.read_len("imports length")?;
    let mut imports = Vec::with_capacity(import_count);
    for _ in 0..import_count {
        imports.push(read_import(r)?);
    }
    let global_count = r.read_u32()?;

    Ok(CompiledModule {
        name,
        init_func,
        functions,
        function_globals,
        exports,
        imports,
        global_count,
    })
}

fn write_import(w: &mut Writer, import: &ImportBinding) -> Result<(), BytecodeError> {
    w.write_string(&import.path)?;
    w.write_string(&import.alias)?;
    w.write_len(
        import.export_to_global.len(),
        "import export_to_global length",
    )?;
    for (name, destination) in &import.export_to_global {
        w.write_string(name)?;
        w.write_u32(*destination);
    }
    write_module(w, &import.module)
}

fn read_import(r: &mut Reader<'_>) -> Result<ImportBinding, BytecodeError> {
    let path = r.read_string("import.path")?;
    let alias = r.read_string("import.alias")?;
    let pair_count = r.read_len("import export_to_global length")?;
    let mut export_to_global = Vec::with_capacity(pair_count);
    for _ in 0..pair_count {
        export_to_global.push((r.read_string("import export name")?, r.read_u32()?));
    }
    let module = Arc::new(read_module(r)?);
    Ok(ImportBinding {
        path,
        alias,
        module,
        export_to_global,
    })
}

fn write_function(w: &mut Writer, function: &CompiledFunction) -> Result<(), BytecodeError> {
    w.write_u32(function.id);
    w.write_u32(function.local_count);
    w.write_u32(function.arg_count);
    w.write_u32(function.ret_count);
    w.write_u32(function.err_count);
    write_fn_meta(w, &function.meta)?;
    w.write_len(function.code.len(), "function code length")?;
    for instr in function.code.iter() {
        write_instr(w, instr)?;
    }
    Ok(())
}

fn read_function(r: &mut Reader<'_>) -> Result<CompiledFunction, BytecodeError> {
    let id = r.read_u32()?;
    let local_count = r.read_u32()?;
    let arg_count = r.read_u32()?;
    let ret_count = r.read_u32()?;
    let err_count = r.read_u32()?;
    let meta = read_fn_meta(r)?;
    let code_len = r.read_len("function code length")?;
    let mut code = Vec::with_capacity(code_len);
    for _ in 0..code_len {
        code.push(read_instr(r)?);
    }
    Ok(CompiledFunction {
        id,
        code: Arc::from(code),
        local_count,
        arg_count,
        ret_count,
        err_count,
        meta,
    })
}

fn write_fn_meta(w: &mut Writer, meta: &FnMeta) -> Result<(), BytecodeError> {
    w.write_string(meta.name.as_ref())?;
    w.write_u32(meta.arg_count);
    w.write_u32(meta.ret_count);
    write_retshape(w, &meta.retshape)
}

fn read_fn_meta(r: &mut Reader<'_>) -> Result<FnMeta, BytecodeError> {
    let name = Arc::<str>::from(r.read_string("fn meta name")?.as_str());
    let arg_count = r.read_u32()?;
    let ret_count = r.read_u32()?;
    let retshape = read_retshape(r)?;
    Ok(FnMeta {
        name,
        arg_count,
        ret_count,
        retshape,
    })
}

fn write_retshape(w: &mut Writer, retshape: &RetShape) -> Result<(), BytecodeError> {
    match retshape {
        RetShape::Scalar => w.write_u8(0),
        RetShape::Either(values) => {
            w.write_u8(1);
            w.write_len(values.len(), "retshape either length")?;
            for value in values {
                w.write_string(value)?;
            }
        }
        RetShape::Record(values) => {
            w.write_u8(2);
            w.write_len(values.len(), "retshape record length")?;
            for value in values {
                w.write_string(value)?;
            }
        }
        RetShape::Any => w.write_u8(3),
    }
    Ok(())
}

fn read_retshape(r: &mut Reader<'_>) -> Result<RetShape, BytecodeError> {
    let tag = r.read_u8()?;
    match tag {
        0 => Ok(RetShape::Scalar),
        1 => {
            let len = r.read_len("retshape either length")?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(r.read_string("retshape either value")?);
            }
            Ok(RetShape::Either(values))
        }
        2 => {
            let len = r.read_len("retshape record length")?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(r.read_string("retshape record value")?);
            }
            Ok(RetShape::Record(values))
        }
        3 => Ok(RetShape::Any),
        _ => Err(BytecodeError::InvalidTag {
            kind: "retshape",
            tag,
        }),
    }
}

fn write_slot(w: &mut Writer, slot: Slot) {
    match slot {
        Slot::Local(v) => {
            w.write_u8(0);
            w.write_u32(v);
        }
        Slot::Global(v) => {
            w.write_u8(1);
            w.write_u32(v);
        }
        Slot::Arg(v) => {
            w.write_u8(2);
            w.write_u32(v);
        }
        Slot::Ret(v) => {
            w.write_u8(3);
            w.write_u32(v);
        }
        Slot::Err(v) => {
            w.write_u8(4);
            w.write_u32(v);
        }
    }
}

fn read_slot(r: &mut Reader<'_>) -> Result<Slot, BytecodeError> {
    let tag = r.read_u8()?;
    let value = r.read_u32()?;
    match tag {
        0 => Ok(Slot::Local(value)),
        1 => Ok(Slot::Global(value)),
        2 => Ok(Slot::Arg(value)),
        3 => Ok(Slot::Ret(value)),
        4 => Ok(Slot::Err(value)),
        _ => Err(BytecodeError::InvalidTag { kind: "slot", tag }),
    }
}

fn write_const(w: &mut Writer, value: &ConstValue) -> Result<(), BytecodeError> {
    match value {
        ConstValue::Null => w.write_u8(0),
        ConstValue::Bool(v) => {
            w.write_u8(1);
            w.write_u8(if *v { 1 } else { 0 });
        }
        ConstValue::Num(v) => {
            w.write_u8(2);
            w.write_f64(*v);
        }
        ConstValue::Str(v) => {
            w.write_u8(3);
            w.write_string(v)?;
        }
    }
    Ok(())
}

fn read_const(r: &mut Reader<'_>) -> Result<ConstValue, BytecodeError> {
    let tag = r.read_u8()?;
    match tag {
        0 => Ok(ConstValue::Null),
        1 => Ok(ConstValue::Bool(r.read_u8()? != 0)),
        2 => Ok(ConstValue::Num(r.read_f64()?)),
        3 => Ok(ConstValue::Str(Arc::<str>::from(
            r.read_string("const string")?.as_str(),
        ))),
        _ => Err(BytecodeError::InvalidTag { kind: "const", tag }),
    }
}

fn write_instr(w: &mut Writer, instr: &Instr) -> Result<(), BytecodeError> {
    match instr {
        Instr::StoreConst { slot, value } => {
            w.write_u8(0);
            write_slot(w, *slot);
            write_const(w, value)?;
        }
        Instr::Move { from, to } => {
            w.write_u8(1);
            write_slot(w, *from);
            write_slot(w, *to);
        }
        Instr::Add { a, b, out } => {
            w.write_u8(2);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Sub { a, b, out } => {
            w.write_u8(3);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Mul { a, b, out } => {
            w.write_u8(4);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Div { a, b, out } => {
            w.write_u8(5);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Eq { a, b, out } => {
            w.write_u8(6);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Lt { a, b, out } => {
            w.write_u8(7);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::Jump { target } => {
            w.write_u8(8);
            w.write_usize_as_u32(*target, "jump target")?;
        }
        Instr::Branch {
            cond,
            then_pc,
            else_pc,
        } => {
            w.write_u8(9);
            write_slot(w, *cond);
            w.write_usize_as_u32(*then_pc, "branch then_pc")?;
            w.write_usize_as_u32(*else_pc, "branch else_pc")?;
        }
        Instr::Invoke { fn_slot, args, out } => {
            w.write_u8(10);
            write_slot(w, *fn_slot);
            w.write_len(args.len(), "invoke args length")?;
            for slot in args {
                write_slot(w, *slot);
            }
            write_slot(w, *out);
        }
        Instr::ReturnSet { slot_id, value } => {
            w.write_u8(11);
            w.write_u32(*slot_id);
            write_slot(w, *value);
        }
        Instr::Exit => w.write_u8(12),
        Instr::Throw { code, msg } => {
            w.write_u8(13);
            w.write_string(code)?;
            w.write_string(msg)?;
        }
        Instr::TryPush { handler_pc } => {
            w.write_u8(14);
            w.write_usize_as_u32(*handler_pc, "try handler pc")?;
        }
        Instr::TryPop => w.write_u8(15),
        Instr::ObjNew { out } => {
            w.write_u8(16);
            write_slot(w, *out);
        }
        Instr::ObjSet {
            obj,
            key,
            value,
            out,
        } => {
            w.write_u8(17);
            write_slot(w, *obj);
            write_slot(w, *key);
            write_slot(w, *value);
            write_slot(w, *out);
        }
        Instr::ObjGet { obj, key, out } => {
            w.write_u8(18);
            write_slot(w, *obj);
            write_slot(w, *key);
            write_slot(w, *out);
        }
        Instr::ObjHas { obj, key, out } => {
            w.write_u8(19);
            write_slot(w, *obj);
            write_slot(w, *key);
            write_slot(w, *out);
        }
        Instr::StrConcat { a, b, out } => {
            w.write_u8(20);
            write_slot(w, *a);
            write_slot(w, *b);
            write_slot(w, *out);
        }
        Instr::StrLen { value, out } => {
            w.write_u8(21);
            write_slot(w, *value);
            write_slot(w, *out);
        }
        Instr::HostPrint { slot } => {
            w.write_u8(22);
            write_slot(w, *slot);
        }
    }
    Ok(())
}

fn read_instr(r: &mut Reader<'_>) -> Result<Instr, BytecodeError> {
    let tag = r.read_u8()?;
    match tag {
        0 => Ok(Instr::StoreConst {
            slot: read_slot(r)?,
            value: read_const(r)?,
        }),
        1 => Ok(Instr::Move {
            from: read_slot(r)?,
            to: read_slot(r)?,
        }),
        2 => Ok(Instr::Add {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        3 => Ok(Instr::Sub {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        4 => Ok(Instr::Mul {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        5 => Ok(Instr::Div {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        6 => Ok(Instr::Eq {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        7 => Ok(Instr::Lt {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        8 => Ok(Instr::Jump {
            target: usize::try_from(r.read_u32()?).map_err(|_| BytecodeError::Overflow("jump"))?,
        }),
        9 => Ok(Instr::Branch {
            cond: read_slot(r)?,
            then_pc: usize::try_from(r.read_u32()?)
                .map_err(|_| BytecodeError::Overflow("branch then_pc"))?,
            else_pc: usize::try_from(r.read_u32()?)
                .map_err(|_| BytecodeError::Overflow("branch else_pc"))?,
        }),
        10 => {
            let fn_slot = read_slot(r)?;
            let arg_count = r.read_len("invoke args length")?;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(read_slot(r)?);
            }
            Ok(Instr::Invoke {
                fn_slot,
                args,
                out: read_slot(r)?,
            })
        }
        11 => Ok(Instr::ReturnSet {
            slot_id: r.read_u32()?,
            value: read_slot(r)?,
        }),
        12 => Ok(Instr::Exit),
        13 => Ok(Instr::Throw {
            code: r.read_string("throw.code")?,
            msg: r.read_string("throw.msg")?,
        }),
        14 => Ok(Instr::TryPush {
            handler_pc: usize::try_from(r.read_u32()?)
                .map_err(|_| BytecodeError::Overflow("try handler pc"))?,
        }),
        15 => Ok(Instr::TryPop),
        16 => Ok(Instr::ObjNew { out: read_slot(r)? }),
        17 => Ok(Instr::ObjSet {
            obj: read_slot(r)?,
            key: read_slot(r)?,
            value: read_slot(r)?,
            out: read_slot(r)?,
        }),
        18 => Ok(Instr::ObjGet {
            obj: read_slot(r)?,
            key: read_slot(r)?,
            out: read_slot(r)?,
        }),
        19 => Ok(Instr::ObjHas {
            obj: read_slot(r)?,
            key: read_slot(r)?,
            out: read_slot(r)?,
        }),
        20 => Ok(Instr::StrConcat {
            a: read_slot(r)?,
            b: read_slot(r)?,
            out: read_slot(r)?,
        }),
        21 => Ok(Instr::StrLen {
            value: read_slot(r)?,
            out: read_slot(r)?,
        }),
        22 => Ok(Instr::HostPrint {
            slot: read_slot(r)?,
        }),
        _ => Err(BytecodeError::InvalidTag { kind: "instr", tag }),
    }
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_len(&mut self, value: usize, ctx: &'static str) -> Result<(), BytecodeError> {
        let value = u32::try_from(value).map_err(|_| BytecodeError::Overflow(ctx))?;
        self.write_u32(value);
        Ok(())
    }

    fn write_usize_as_u32(&mut self, value: usize, ctx: &'static str) -> Result<(), BytecodeError> {
        let value = u32::try_from(value).map_err(|_| BytecodeError::Overflow(ctx))?;
        self.write_u32(value);
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), BytecodeError> {
        self.write_len(value.len(), "string length")?;
        self.write_bytes(value.as_bytes());
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos == self.bytes.len()
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], BytecodeError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(BytecodeError::Overflow("reader position"))?;
        if end > self.bytes.len() {
            return Err(BytecodeError::UnexpectedEof);
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, BytecodeError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, BytecodeError> {
        let raw = self.read_exact(2)?;
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(raw);
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, BytecodeError> {
        let raw = self.read_exact(4)?;
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(raw);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_f64(&mut self) -> Result<f64, BytecodeError> {
        let raw = self.read_exact(8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(raw);
        Ok(f64::from_le_bytes(bytes))
    }

    fn read_fixed_4(&mut self) -> Result<[u8; 4], BytecodeError> {
        let raw = self.read_exact(4)?;
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(raw);
        Ok(bytes)
    }

    fn read_len(&mut self, ctx: &'static str) -> Result<usize, BytecodeError> {
        let raw = self.read_u32()?;
        usize::try_from(raw).map_err(|_| BytecodeError::Overflow(ctx))
    }

    fn read_string(&mut self, ctx: &'static str) -> Result<String, BytecodeError> {
        let len = self.read_len("string length")?;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| BytecodeError::InvalidUtf8(ctx.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_compiler::{FsModuleLoader, compile_module};
    use imp_vm::{Value, Vm, VmConfig};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn roundtrip_complex_example_module() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join("enum_custom_object_demo.imp")
            .canonicalize()
            .expect("canonicalize example");
        let module = compile_module(&path, &FsModuleLoader).expect("compile module");
        let encoded = encode_module(&module).expect("encode");
        let decoded = decode_module(&encoded).expect("decode");

        assert_eq!(decoded.name, module.name);
        assert_eq!(decoded.global_count, module.global_count);
        assert_eq!(decoded.functions.len(), module.functions.len());
        assert_eq!(decoded.exports, module.exports);
        assert_eq!(decoded.imports.len(), module.imports.len());
    }

    #[test]
    fn decoded_module_runs_with_vm() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join("enum_custom_object_demo.imp")
            .canonicalize()
            .expect("canonicalize example");
        let module = compile_module(&path, &FsModuleLoader).expect("compile module");
        let encoded = encode_module(&module).expect("encode");
        let decoded = decode_module(&encoded).expect("decode");

        let mut vm = Vm::new(VmConfig {
            enable_host_print: false,
            enable_jit: true,
        });
        let result = vm.run_main(&decoded).expect("run decoded");
        assert_eq!(
            result.returns,
            vec![Value::Str(Arc::from("ok=true name=Ada"))]
        );
    }
}
