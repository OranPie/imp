use imp_compiler::{FsModuleLoader, compile_module};
use imp_vm::{Vm, VmConfig};
use std::env;
use std::path::Path;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("usage: imp <run|dump-ir> <file.imp>");
        return Ok(());
    }

    let command = args.remove(0);
    let path = args.remove(0);

    let module = compile_module(Path::new(&path), &FsModuleLoader)?;

    match command.as_str() {
        "run" => {
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
            for function in &module.functions {
                println!("fn#{} {}", function.id, function.meta.name);
                for (pc, instr) in function.code.iter().enumerate() {
                    println!("  {:04}: {:?}", pc, instr);
                }
            }
        }
        _ => {
            eprintln!("unknown command '{command}', expected run or dump-ir");
        }
    }

    Ok(())
}
