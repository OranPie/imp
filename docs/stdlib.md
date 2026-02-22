# Imp stdlib Guide

Imp stdlib is implemented as `.imp` source modules in `stdlib/`.
Goal: reduce boilerplate for real scripts (validation, map access, text assembly, result flow, calculations).

## Recommended import style (namespaced)

```imp
#call core::import alias="std_bool" path="../stdlib/bool.imp";
#call core::import alias="std_math" path="../stdlib/math.imp";
#call core::import alias="std_ctrl" path="../stdlib/control.imp";
#call core::import alias="std_map" path="../stdlib/map.imp";
#call core::import alias="std_str" path="../stdlib/string.imp";
#call core::import alias="std_res" path="../stdlib/result.imp";
#call core::import alias="std_valid" path="../stdlib/validate.imp";
#call core::import alias="std_calc" path="../stdlib/calc.imp";
#call core::import alias="std_sort" path="../stdlib/sort/mod.imp";
#call core::import alias="std_enum" path="../stdlib/enum.imp";
#call core::import alias="std_cobj" path="../stdlib/custom_object.imp";
#call core::import alias="std_col" path="../stdlib/collections.imp";
#call core::import alias="std_iter" path="../stdlib/iter.imp";
#call core::import alias="std_algo" path="../stdlib/algo.imp";
#call core::import alias="std_output" path="../stdlib/output.imp";
```

Use `stdlib/prelude.imp` only when you need flat compatibility.

## Full stdlib surface

- `bool.imp`: logic (`not/and/or/xor`), comparisons (`eq/neq`), multi-input helpers (`all3/any3`), null checks.
- `math.imp`: arithmetic + aggregates + sign/range helpers.
- `control.imp`: if/coalesce/guard/assert helpers.
- `map.imp`: object-map CRUD style helpers including dynamic-key `set/get/has`.
- `string.imp`: text conversion, concat/repeat/surround/join helpers.
- `result.imp`: `ok/err/from_nullable/is_ok/is_err/unwrap_or/unwrap`.
- `validate.imp`: key, numeric range, positivity, and non-empty text requirements.
- `calc.imp`: business calculations (percent, discount, tax, subtotal, weighted score, safe ratio).
- `sort/`: comparator-function-driven sorting helpers (bubble/selection/check) plus range and pass-limit configs.
- `enum.imp`: tagged-value helpers for variants and enum-style branching.
- `custom_object.imp`: configurable object builders (`define/patch/pick`) and wrappers.
- `collections.imp`: indexed-collection helpers (`fromN/push/swap/clone/reverse/at`).
- `iter.imp`: collection iteration helpers (`reduce_sum/any_eq/map_mul_scalar`).
- `algo.imp`: search/stat helpers (`find_index/contains/min_value/max_value`).
- `output.imp`: parameterized output composition for mixed-type parts, keyed values, and key/value pairs.
- `object.imp`: legacy object constructors.
- `io.imp`: print wrapper.

## Core primitives used by advanced stdlib

- `core::obj::get obj=<ref> key=<ref|const> out=<ref>`
- `core::obj::has obj=<ref> key=<ref|const> out=<ref>`
- `core::obj::set obj=<ref> key=<ref|const> value=<ref> out=<ref>`
- `core::str::concat a=<ref|const> b=<ref|const> out=<ref>`
- `core::str::len value=<ref|const> out=<ref>`

## Complex examples

- `examples/complex_billing_pipeline.imp`
- `examples/complex_profile_validation.imp`
- `examples/complex_retry_flow.imp`
- `examples/bubble_sort_demo.imp`
- `examples/sort_custom_comp_demo.imp`
- `examples/sort_config_demo.imp`
- `examples/enum_custom_object_demo.imp`
- `examples/collections_algo_demo.imp`
- `examples/output_collections_demo.imp`

Legacy/simple examples still available:
- `examples/stdlib_namespaced_demo.imp`
- `examples/stdlib_control_demo.imp`
- `examples/stdlib_map_demo.imp`
- `examples/stdlib_result_demo.imp`
- `examples/stdlib_demo.imp`

## See also

- `docs/stdlib_reference.md` - full API list.
- `docs/stdlib_cookbook.md` - practical composition recipes.
