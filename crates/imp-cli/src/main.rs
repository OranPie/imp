use imp_bytecode::{decode_from_path, encode_to_path};
use imp_compiler::{FsModuleLoader, compile_module};
use imp_ir::CompiledModule;
use imp_vm::{Vm, VmConfig};
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("usage: imp <run|dump-ir|build> <file.(imp|impc)> [options]");
        return Ok(());
    }

    let command = args.remove(0);
    match command.as_str() {
        "run" => {
            let path = args.remove(0);
            let strict = parse_strict_flag(&args)?;
            let module = load_module(Path::new(&path), strict)?;
            let mut cfg = VmConfig::default();
            if std::env::var("IMP_NO_JIT").is_ok() {
                cfg.enable_jit = false;
            }
            let mut vm = Vm::new(cfg);
            let result = vm.run_main(&module)?;
            println!("returns: {:?}", result.returns);
            if !result.exports.is_empty() {
                println!("exports: {:?}", result.exports);
            }
        }
        "dump-ir" => {
            let path = args.remove(0);
            let strict = parse_strict_flag(&args)?;
            let module = load_module(Path::new(&path), strict)?;
            for function in &module.functions {
                println!("fn#{} {}", function.id, function.meta.name);
                for (pc, instr) in function.code.iter().enumerate() {
                    println!("  {:04}: {:?}", pc, instr);
                }
            }
        }
        "build" => {
            let input = args.remove(0);
            let (out_path, strict) = parse_build_flags(Path::new(&input), &args)?;
            if strict {
                eprintln!("warning: --strict-bytecode has no effect for build");
            }
            if has_impc_extension(Path::new(&input)) {
                return Err("build expects a .imp source input".into());
            }
            let module = compile_module(Path::new(&input), &FsModuleLoader)?;
            encode_to_path(&out_path, &module)?;
            println!("wrote {}", out_path.display());
        }
        _ => {
            eprintln!("unknown command '{command}', expected run, dump-ir, or build");
        }
    }

    Ok(())
}

fn load_module(
    path: &Path,
    strict_bytecode: bool,
) -> Result<CompiledModule, Box<dyn std::error::Error>> {
    if has_impc_extension(path) {
        return Ok(decode_from_path(path)?);
    }
    if strict_bytecode {
        return Err("strict bytecode mode requires .impc input".into());
    }
    Ok(compile_module(path, &FsModuleLoader)?)
}

fn parse_strict_flag(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let mut strict = false;
    for arg in args {
        match arg.as_str() {
            "--strict-bytecode" => strict = true,
            other => {
                return Err(format!("unknown option '{other}'").into());
            }
        }
    }
    Ok(strict)
}

fn parse_build_flags(
    input: &Path,
    args: &[String],
) -> Result<(PathBuf, bool), Box<dyn std::error::Error>> {
    let mut strict = false;
    let mut out: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--strict-bytecode" => {
                strict = true;
                i += 1;
            }
            "-o" | "--out" => {
                let Some(next) = args.get(i + 1) else {
                    return Err("missing output path after -o/--out".into());
                };
                out = Some(PathBuf::from(next));
                i += 2;
            }
            other => return Err(format!("unknown option '{other}'").into()),
        }
    }

    let out = if let Some(out) = out {
        out
    } else {
        default_impc_path(input)
    };
    Ok((out, strict))
}

fn default_impc_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();
    output.set_extension("impc");
    output
}

fn has_impc_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(|s| s.to_str()), Some("impc"))
}
