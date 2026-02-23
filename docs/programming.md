# Imp Programming Guide

This guide focuses on writing real `.imp` programs efficiently.

## 1) Mental model

Imp-Core v2 programs follow one pipeline:

1. Parse `#call` statements to AST.
2. Expand compile-time annotations (for example `@safe`).
3. Compile to slot-based IR instructions.
4. Execute on VM (JIT enabled by default, interpreter fallback available).

As a script author, you mainly care about `#call` composition and module APIs.

## 2) Single statement form

Imp has one source-level statement:

```imp
#call [@anno ...] target key=value key=value ... ;
```

- `target` can be `core::...` or `alias::function`.
- `key=value` pairs are named arguments.
- Every line ends with `;`.

## 3) Atoms and references

Values in arguments are atoms:

- `null`
- `true` / `false`
- numbers (`f64`)
- strings (`"text"`)
- refs (`namespace::name`)

Reference namespaces:

- `local::` locals in current frame
- `arg::` function arguments
- `return::` return slots
- `err::` error slots
- `main::` and other namespaces map to globals/module exports

## 4) Function definitions

Define functions with `core::fn::begin` / `core::fn::end`:

```imp
#call core::fn::begin name=main::sum2 args="a,b" retshape="scalar";
#call core::add a=arg::a b=arg::b out=return::value;
#call core::exit;
#call core::fn::end;
```

Rules:

- `name` is a global function slot (usually `main::...`).
- `args` is a CSV list bound to `arg::...`.
- `retshape` controls return validation on `core::exit`.
- Call `core::exit` to finish a function path.

## 5) Calling functions

- `core::...` targets compile directly to IR instructions.
- non-`core` targets compile to invoke (`Instr::Invoke`).

Call style:

```imp
#call std_math::sum3 args="local::x,local::y,local::z" out=local::total;
```

The `args` field is CSV refs for positional call arguments.

## 6) Control flow

Core flow ops:

- `core::label name="L"`
- `core::jump target="L"`
- `core::br cond=<ref> then="L1" else="L2"`

Typical loop shape:

```imp
#call core::label name="loop";
#call core::lt a=local::i b=local::n out=local::keep;
#call core::br cond=local::keep then="body" else="done";
#call core::label name="body";
#call core::add a=local::i b=local::one out=local::i;
#call core::jump target="loop";
#call core::label name="done";
```

## 7) Errors and safety

Throw error:

```imp
#call core::throw code="bad_input" msg="value invalid";
```

Manual handlers:

```imp
#call core::try::push handler="on_err";
#call core::div a=local::a b=local::b out=local::q;
#call core::try::pop;
#call core::jump target="ok";
#call core::label name="on_err";
#call core::const out=local::q value=null;
#call core::label name="ok";
```

Shortcut safety annotation:

```imp
#call @safe core::div a=local::a b=local::b out=local::q;
```

`@safe` is expanded at compile time.

## 8) Module organization

Import module file:

```imp
#call core::import alias="std_map" path="../stdlib/map.imp";
```

Export from a module:

```imp
#call core::mod::export name="set" value=main::set;
#call core::exit;
```

Recommendations:

- keep one concern per module (validation, calculation, formatting)
- export small reusable functions
- prefer namespaced imports over flat prelude

## 9) Stdlib-first coding style

For everyday scripting, compose from stdlib modules:

- `math.imp`, `bool.imp`, `control.imp`
- `map.imp`, `string.imp`, `result.imp`
- `validate.imp`, `calc.imp`
- `sort/mod.imp`, `collections.imp`, `iter.imp`, `algo.imp`, `output.imp`

This keeps core VM minimal while giving high-level APIs in `.imp`.

## 10) Common patterns

### 10.1 Dynamic-key object map

```imp
#call std_map::set args="local::obj,local::key,local::value" out=local::obj;
#call std_map::get_or args="local::obj,local::key,local::fallback" out=local::v;
```

### 10.2 Validate then compute

```imp
#call std_valid::require_positive args="local::qty,local::msg" out=local::qty_checked;
#call std_calc::taxed_total args="local::qty_checked,local::unit,local::disc,local::tax" out=local::total;
```

### 10.3 Result flow

```imp
#call std_res::from_nullable args="local::maybe_user,local::err" out=local::res;
#call std_res::unwrap_or args="local::res,local::fallback" out=return::value;
#call core::exit;
```

## 11) Debugging tips

- use `core::host::print` (or `std_io::print`) on intermediate values
- run `imp dump-ir file.imp` to inspect lowered instructions
- run with `IMP_NO_JIT=1` to compare interpreter behavior
- keep labels unique and explicit (`loop`, `done`, `on_err`)

## 12) Performance tips

- cache reused constants in locals instead of re-creating each step
- prefer module helpers over repeated low-level call sequences
- keep hot loops numeric and branch-light when possible
- use `.impc` build/run for deployment startup improvements

## 13) Next reading

- `docs/spec-v2.md`
- `docs/stdlib.md`
- `docs/stdlib_reference.md`
- `docs/stdlib_cookbook.md`
- `examples/complex_billing_pipeline.imp`
- `examples/sort_custom_comp_demo.imp`
