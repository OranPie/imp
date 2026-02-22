use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use imp_compiler::{CompileOpts, FsModuleLoader, compile_module, compile_program};
use imp_ir::CompiledModule;
use imp_vm::{Value, Vm, VmConfig};
use std::hint::black_box;
use std::path::PathBuf;

const ARITH_LOOP: &str = r#"
#call core::const out=local::i value=0;
#call core::const out=local::sum value=0;
#call core::const out=local::one value=1;
#call core::const out=local::limit value=1000;
#call core::label name="loop";
#call core::lt a=local::i b=local::limit out=local::cond;
#call core::br cond=local::cond then="body" else="done";
#call core::label name="body";
#call core::add a=local::sum b=local::i out=local::sum;
#call core::add a=local::i b=local::one out=local::i;
#call core::jump target="loop";
#call core::label name="done";
#call core::mov from=local::sum to=return::value;
#call core::exit;
"#;

const INVOKE_LOOP: &str = r#"
#call core::fn::begin name=main::inc args="x" retshape="scalar";
#call core::const out=local::one value=1;
#call core::add a=arg::x b=local::one out=return::value;
#call core::exit;
#call core::fn::end;

#call core::const out=local::x value=0;
#call core::const out=local::i value=0;
#call core::const out=local::one value=1;
#call core::const out=local::limit value=1000;
#call core::label name="loop";
#call core::lt a=local::i b=local::limit out=local::cond;
#call core::br cond=local::cond then="body" else="done";
#call core::label name="body";
#call core::invoke fn=main::inc args="local::x" out=local::x;
#call core::add a=local::i b=local::one out=local::i;
#call core::jump target="loop";
#call core::label name="done";
#call core::mov from=local::x to=return::value;
#call core::exit;
"#;

const SAFE_DIV_LOOP: &str = r#"
#call core::const out=local::n value=0;
#call core::const out=local::one value=1;
#call core::const out=local::zero value=0;
#call core::const out=local::limit value=300;
#call core::label name="loop";
#call core::lt a=local::n b=local::limit out=local::cond;
#call core::br cond=local::cond then="body" else="done";
#call core::label name="body";
#call @safe core::div a=local::one b=local::zero out=local::sink;
#call core::add a=local::n b=local::one out=local::n;
#call core::jump target="loop";
#call core::label name="done";
#call core::mov from=local::n to=return::value;
#call core::exit;
"#;

fn compile_bench_module(src: &str) -> CompiledModule {
    compile_program(
        src,
        CompileOpts {
            module_name: "bench".to_owned(),
        },
    )
    .expect("compile benchmark program")
    .module
}

fn run_module(module: &CompiledModule, enable_jit: bool) -> Value {
    let mut vm = Vm::new(VmConfig {
        enable_host_print: false,
        enable_jit,
    });
    let result = vm
        .run_main(black_box(module))
        .expect("run benchmark program");
    result.returns.first().cloned().unwrap_or(Value::Null)
}

fn bench_program(c: &mut Criterion, name: &str, src: &'static str) {
    let module = compile_bench_module(src);
    bench_compiled_module(c, name, module);
}

fn bench_compiled_module(c: &mut Criterion, name: &str, module: CompiledModule) {
    let mut group = c.benchmark_group(name);
    group.sample_size(30);

    for (label, enable_jit) in [("jit", true), ("interp", false)] {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &enable_jit,
            |b, &jit| {
                b.iter(|| {
                    let value = run_module(&module, jit);
                    black_box(value)
                });
            },
        );
    }

    group.finish();
}

fn compile_import_invoke_chain_module() -> CompiledModule {
    let temp = std::env::temp_dir();
    let provider_path = temp.join("imp_bench_provider.imp");
    let consumer_path = temp.join("imp_bench_consumer.imp");

    let provider_src = r#"
#call core::fn::begin name=main::inc args="x" retshape="scalar";
#call core::const out=local::one value=1;
#call core::add a=arg::x b=local::one out=return::value;
#call core::exit;
#call core::fn::end;
#call core::mod::export name="inc" value=main::inc;
#call core::exit;
"#;
    std::fs::write(&provider_path, provider_src).expect("write bench provider");

    let consumer_src = format!(
        r#"#call core::import alias="p" path="{}";
#call core::const out=local::x value=0;
#call core::const out=local::i value=0;
#call core::const out=local::one value=1;
#call core::const out=local::limit value=1000;
#call core::label name="loop";
#call core::lt a=local::i b=local::limit out=local::cond;
#call core::br cond=local::cond then="body" else="done";
#call core::label name="body";
#call p::inc args="local::x" out=local::x;
#call core::add a=local::i b=local::one out=local::i;
#call core::jump target="loop";
#call core::label name="done";
#call core::mov from=local::x to=return::value;
#call core::exit;
"#,
        provider_path.display()
    );
    std::fs::write(&consumer_path, consumer_src).expect("write bench consumer");

    let path = PathBuf::from(&consumer_path);
    compile_module(&path, &FsModuleLoader).expect("compile bench import invoke module")
}

fn vm_benchmarks(c: &mut Criterion) {
    bench_program(c, "arith_loop", ARITH_LOOP);
    bench_program(c, "invoke_loop", INVOKE_LOOP);
    bench_program(c, "safe_div_loop", SAFE_DIV_LOOP);
    bench_compiled_module(
        c,
        "module_invoke_chain",
        compile_import_invoke_chain_module(),
    );
}

criterion_group!(benches, vm_benchmarks);
criterion_main!(benches);
