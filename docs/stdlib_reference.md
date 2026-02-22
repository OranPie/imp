# Stdlib API Reference

## bool.imp

- `not(value) -> bool`
- `and(a, b) -> bool`
- `or(a, b) -> bool`
- `xor(a, b) -> bool`
- `eq(a, b) -> bool`
- `neq(a, b) -> bool`
- `all3(a, b, c) -> bool`
- `any3(a, b, c) -> bool`
- `select(cond, when_true, when_false) -> any`
- `is_null(value) -> bool`
- `is_not_null(value) -> bool`

## math.imp

- `add/sub/mul/div(a, b) -> num`
- `inc/dec(x) -> num`
- `sum3(a,b,c)`, `sum4(a,b,c,d)`
- `avg2(a,b)`, `avg3(a,b,c)`
- `abs(x)`, `sign(x)`
- `is_positive(x)`, `is_negative(x)`
- `min/max(a,b)`
- `clamp(value, low, high)`
- `between(value, low, high)`

## control.imp

- `if_else(cond, when_true, when_false) -> any`
- `coalesce(value, fallback) -> any`
- `guard_or(cond, value, msg) -> value | throw`
- `require_not_null(value, msg) -> value | throw`
- `assert_true(cond, msg) -> true | throw`
- `assert_eq(a, b, msg) -> true | throw`

## map.imp

- `new() -> obj`
- `set(obj, key, value) -> obj`
- `get(obj, key) -> any|null`
- `has(obj, key) -> bool`
- `get_or(obj, key, fallback) -> any`
- `require(obj, key, msg) -> any | throw`
- `upsert_default(obj, key, default_value) -> obj`

## string.imp

- `to_text(value) -> str`
- `concat(a, b) -> str`
- `concat3(a, b, c) -> str`
- `repeat2(value) -> str`
- `repeat3(value) -> str`
- `len(value) -> num`
- `is_empty(value) -> bool`
- `prefix(label, value) -> str`
- `surround(left, value, right) -> str`
- `join_space(a, b) -> str`
- `join_colon(a, b) -> str`

## result.imp

Result shape:
- success: `{ ok: true, value: ... }`
- error: `{ ok: false, error: ... }`

Functions:
- `ok(value)`
- `err(message)`
- `from_nullable(value, error_message)`
- `is_ok(result)`
- `is_err(result)`
- `unwrap_or(result, fallback)`
- `unwrap(result, msg_prefix)`

## validate.imp

- `require_between(value, low, high, msg)`
- `require_positive(value, msg)`
- `require_non_empty_text(value, msg)`
- `require_key(obj, key, msg)`

## calc.imp

- `percent_of(value, pct)`
- `discount_amount(subtotal, discount_pct)`
- `tax_amount(base, tax_pct)`
- `subtotal(qty, unit_price)`
- `taxed_total(qty, unit_price, discount_pct, tax_pct)`
- `ratio_or(part, total, fallback)`
- `weighted_score(base, bonus, multiplier)`

## sort/ (directory module)

Comparator contract (`comp(a,b) -> bool`):
- return `true` when the left element should move right (swap)
- return `false` when the pair order is already correct
- custom comparator functions are supported (including user-defined functions)

Built-in comparators:
- `comp_asc(a,b)` numeric/text ascending comparator
- `comp_desc(a,b)` numeric/text descending comparator

Bubble methods:
- `bubble_cfg(obj, start, end, max_passes, comp)`
- `bubble_by(obj, n, comp)`
- `bubble_asc(obj, n)`
- `bubble_desc(obj, n)`
- `bubble_range_by(obj, start, end, comp)`
- `bubble_partial_by(obj, n, max_passes, comp)`

Selection methods:
- `selection_by(obj, n, comp)`
- `selection_asc(obj, n)`
- `selection_desc(obj, n)`

Check methods:
- `is_sorted_by(obj, n, comp) -> bool`
- `is_sorted_asc(obj, n) -> bool`
- `is_sorted_desc(obj, n) -> bool`

Compatibility:
- `stdlib/sort.imp` remains as a shim that re-exports from `stdlib/sort/mod.imp`

## enum.imp

Core constructors/accessors:
- `variant(tag, payload) -> obj`
- `unit(tag) -> obj`
- `tag(value) -> scalar|null`
- `payload(value) -> scalar|null`
- `payload_or(value, fallback) -> scalar`

Tag checks/control:
- `is_tag(value, tag) -> bool`
- `expect_payload(value, tag, msg) -> payload | throw`
- `match_tag(value, tag, when_true, when_false) -> scalar`

## custom_object.imp

Basic object ops:
- `new() -> obj`
- `set(obj, key, value) -> obj`
- `get(obj, key) -> scalar|null`
- `has(obj, key) -> bool`

Configurable builders:
- `define(keys, values, n) -> obj`
- `patch(obj, keys, values, n) -> obj`
- `pick(obj, keys, n) -> obj`
- `with2(k1, v1, k2, v2) -> obj`
- `with3(k1, v1, k2, v2, k3, v3) -> obj`

## output.imp

- `join_parts(parts, n, sep, prefix, suffix)` for numeric-indexed mixed-type collections
- `join_values(obj, keys, n, sep, prefix, suffix)` for selected keyed values
- `join_pairs(obj, keys, n, kv_sep, part_sep, prefix, suffix)` for selected `key<kv_sep>value` segments

## Legacy/utility modules

- `io.imp`: `print(value) -> value`
- `object.imp`: `point2`, `pair`, `result_ok`, `result_err`
- `prelude.imp`: flat compatibility exports
