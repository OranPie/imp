# imp

Imp-Core v2.0 reference implementation in Rust.

## Features

- Parse/compile/VM execution pipeline
- Runtime JIT path with interpreter fallback
- Module import/export system
- Expanded stdlib written in `.imp`
- Complex example programs

## Quick run

```bash
cargo run -p imp-cli -- run examples/complex_billing_pipeline.imp
cargo run -p imp-cli -- run examples/bubble_sort_demo.imp
cargo run -p imp-cli -- run examples/output_collections_demo.imp
```

## Tests

```bash
cargo test
```

## Benchmarks

```bash
cargo bench -p imp-vm --bench vm_bench
```

## Docs

- `docs/spec-v2.md`
- `docs/stdlib.md`
- `docs/stdlib_reference.md`
- `docs/stdlib_cookbook.md`
- `docs/benchmarks.md`
