# Stdlib API Reference

This document lists exported stdlib functions and expected behavior.

## bool.imp

- `not(value) -> bool`
- `and(a, b) -> bool`
- `or(a, b) -> bool`
- `xor(a, b) -> bool`
- `select(cond, when_true, when_false) -> any`
- `is_null(value) -> bool`
- `is_not_null(value) -> bool`

## math.imp

- `add(a, b) -> num`
- `sub(a, b) -> num`
- `mul(a, b) -> num`
- `div(a, b) -> num`
- `inc(x) -> num`
- `dec(x) -> num`
- `sum3(a, b, c) -> num`
- `avg2(a, b) -> num`
- `abs(x) -> num`
- `sign(x) -> num` (`-1`, `0`, `1`)
- `is_positive(x) -> bool`
- `is_negative(x) -> bool`
- `min(a, b) -> num`
- `max(a, b) -> num`
- `clamp(value, low, high) -> num`
- `between(value, low, high) -> bool`

## control.imp

- `if_else(cond, when_true, when_false) -> any`
- `coalesce(value, fallback) -> any`
- `require_not_null(value, msg) -> value | throw`
- `assert_true(cond, msg) -> true | throw`
- `assert_eq(a, b, msg) -> true | throw`

Throw behavior:
- prints `msg` via host output before throwing.
- throw code is stable (`null_error`, `assert_true`, `assert_eq`).

## map.imp

- `new() -> obj`
- `get(obj, key) -> any|null`
- `has(obj, key) -> bool`
- `get_or(obj, key, fallback) -> any`
- `require(obj, key, msg) -> any | throw`

`require` prints `msg` and throws with code `missing_key` when absent.

## string.imp

- `concat(a, b) -> str`
- `concat3(a, b, c) -> str`
- `len(value) -> num`
- `is_empty(value) -> bool`
- `prefix(label, value) -> str`

## result.imp

Shape convention:
- success: `{ ok: true, value: ... }`
- error: `{ ok: false, error: ... }`

Functions:
- `ok(value) -> result`
- `err(message) -> result`
- `is_ok(result) -> bool`
- `is_err(result) -> bool`
- `unwrap_or(result, fallback) -> any`
- `expect(result, msg_prefix) -> any | throw`

`expect` prints `msg_prefix + error` then throws with code `expect_failed`.

## io.imp

- `print(value) -> value`

## object.imp (legacy helpers)

- `point2(x, y) -> obj`
- `pair(first, second) -> obj`
- `result_ok(value) -> obj`
- `result_err(message) -> obj`

## prelude.imp (flat compatibility)

`prelude.imp` re-exports common helpers with short names for existing scripts.
Prefer namespaced imports for new code.
