# Imp-Core v2.0 Specification (Implemented)

## Pipeline

1. Parse `#call` statements into AST calls.
2. Expand compile-time annotations (`@safe` on `core::div`).
3. Compile calls into slot-based IR (`Instr`).
4. Execute IR on VM frames.

## Source Statement

```imp
#call [@anno ...] target key=value key=value ... ;
```

## Atoms

- `null`
- `true` / `false`
- numeric literal (`f64`)
- string literal (`"..."`)
- reference (`namespace::name`)

## Namespaces

- `local::` local frame slots
- `arg::` argument slots
- `return::` return slots
- `err::` error slots
- any other namespace maps to global slots (including `main::`, `mod::`, import aliases)

## Compile-time Behavior

- Function declarations are defined with `core::fn::begin` / `core::fn::end`.
- Targets in `core::*` lower directly to IR instructions.
- Non-`core::*` targets lower to `Instr::Invoke` using a function-valued slot.
- Labels are resolved to concrete program counters at compile time.
- `@safe core::div` lowers to `try`/`jump`/fallback-const sequence.

## Runtime Behavior

- Frame fields: `code`, `pc`, `locals`, `args`, `ret`, `err`, `try_stack`, `meta`.
- Slot accesses are index-based (no runtime ref parsing).
- `Exit` validates return shape according to function metadata.
- `Throw` unwinds to the nearest frame-local try handler, else propagates.

## Current Extensions

- Host print: `core::host::print`
- Object helpers: `core::obj::new`, `core::obj::set`, `core::obj::get`, `core::obj::has`
- String helpers: `core::str::concat`, `core::str::len`
- Module metadata calls: `core::import`, `core::mod::export`

## Standard Library

- Everyday stdlib APIs are implemented as regular `.imp` modules in `stdlib/`.
- Recommended style is namespaced module imports (`math.imp`, `control.imp`, `map.imp`, `string.imp`, `result.imp`).
- `stdlib/prelude.imp` remains available as a flat compatibility layer for existing scripts.
- This keeps the VM core compact while enabling practical high-level coding helpers.

## JIT Backend

- VM includes a runtime JIT tier that compiles IR instructions to a direct-threaded step plan.
- JIT is enabled by default (`VmConfig.enable_jit = true`).
- Supported in JIT tier: data/arithmetic/compare/control/invoke/return/exit/throw/try/object/host-print.
- Disable with `VmConfig.enable_jit = false` or `IMP_NO_JIT=1` for CLI runs.
