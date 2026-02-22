# Benchmark Guide

Run VM benchmark suite (Criterion):

```bash
cargo bench -p imp-vm --bench vm_bench
```

The suite compares `jit` vs `interp` for:

- `arith_loop`
- `invoke_loop`
- `safe_div_loop`
- `module_invoke_chain`

To quickly check compile only:

```bash
cargo bench -p imp-vm --no-run
```

To run CLI without JIT for comparison:

```bash
IMP_NO_JIT=1 cargo run -p imp-cli -- run path/to/file.imp
```

To benchmark AOT-loading startup manually:

```bash
cargo run -p imp-cli -- build examples/complex_billing_pipeline.imp -o /tmp/billing.impc
time cargo run -p imp-cli -- run examples/complex_billing_pipeline.imp
time cargo run -p imp-cli -- run /tmp/billing.impc
```
