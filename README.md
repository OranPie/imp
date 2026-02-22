# imp

Imp-Core v2.0 reference implementation in Rust.

## What is included

- Parser, compiler, IR, VM, JIT execution path
- Import/module system
- Expanded stdlib as `.imp` source modules
- CLI runner and IR dump command

## Run

```bash
cargo run -p imp-cli -- run examples/stdlib_namespaced_demo.imp
```

## Test

```bash
cargo test
```

## Bench

```bash
cargo bench -p imp-vm --bench vm_bench
```

## Docs

- `docs/spec-v2.md` - language/runtime specification
- `docs/stdlib.md` - stdlib guide and usage
- `docs/stdlib_reference.md` - full API list
- `docs/stdlib_cookbook.md` - practical patterns
- `docs/benchmarks.md` - benchmark commands
