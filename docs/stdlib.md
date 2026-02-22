# Imp stdlib Guide

Imp stdlib is written in pure `.imp` modules under `stdlib/`.
It is designed to reduce boilerplate in day-to-day scripts and small apps.

## Fast start

Use namespaced imports (recommended):

```imp
#call core::import alias="std_math" path="../stdlib/math.imp";
#call core::import alias="std_ctrl" path="../stdlib/control.imp";
#call core::import alias="std_map" path="../stdlib/map.imp";
#call core::import alias="std_str" path="../stdlib/string.imp";
#call core::import alias="std_res" path="../stdlib/result.imp";
```

Run an example:

```bash
cargo run -p imp-cli -- run examples/stdlib_namespaced_demo.imp
```

## Module overview

- `bool.imp` - boolean logic and null checks.
- `math.imp` - arithmetic, clamping, range checks, sign helpers.
- `control.imp` - conditional selection, assertions, null guards.
- `map.imp` - object-as-map read/write access helpers.
- `string.imp` - concat and length helpers.
- `result.imp` - `{ ok, value/error }` workflow helpers.
- `io.imp` - host output wrappers.
- `object.imp` - legacy object constructors.

## Compatibility mode

`stdlib/prelude.imp` re-exports a flat API for existing scripts.

```imp
#call core::import alias="std" path="../stdlib/prelude.imp";
#call std::clamp args="local::x,local::low,local::high" out=local::y;
```

Prefer namespaced style for new code because it scales better in larger files.

## Core helpers used by stdlib

These are low-level core ops that enable higher-level stdlib modules:

- `core::obj::get obj=<ref> key=<ref|const> out=<ref>`
- `core::obj::has obj=<ref> key=<ref|const> out=<ref>`
- `core::str::concat a=<ref|const> b=<ref|const> out=<ref>`
- `core::str::len value=<ref|const> out=<ref>`

Notes:
- Missing map key returns `null` for `obj::get`.
- `obj::has` returns boolean.
- `str::concat` and `str::len` accept refs or const atoms.

## Typical coding patterns

### 1) Defensive value read

```imp
#call std_map::get_or args="local::cfg,local::k_timeout,local::fallback" out=local::timeout;
```

### 2) Assert and continue

```imp
#call std_ctrl::assert_true args="local::is_valid,local::msg" out=local::ok;
```

### 3) Result pipeline

```imp
#call std_res::ok args="local::value" out=local::r;
#call std_res::unwrap_or args="local::r,local::fallback" out=local::resolved;
```

## Examples in repo

- `examples/stdlib_namespaced_demo.imp`
- `examples/stdlib_control_demo.imp`
- `examples/stdlib_map_demo.imp`
- `examples/stdlib_result_demo.imp`
- `examples/stdlib_demo.imp` (flat prelude compatibility)

## Next docs

- API details: `docs/stdlib_reference.md`
- Practical recipes: `docs/stdlib_cookbook.md`
