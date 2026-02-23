# 标准库 API 参考

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
- `sum3(a,b,c)`、`sum4(a,b,c,d)`
- `avg2(a,b)`、`avg3(a,b,c)`
- `abs(x)`、`sign(x)`
- `is_positive(x)`、`is_negative(x)`
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

结果结构：
- 成功：`{ ok: true, value: ... }`
- 失败：`{ ok: false, error: ... }`

函数：
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

## sort/（目录模块）

比较器约定（`comp(a,b) -> bool`）：
- 返回 `true`：左元素应向右移动（交换）
- 返回 `false`：当前顺序已正确
- 支持自定义比较函数（包括用户定义函数）

内置比较器：
- `comp_asc(a,b)`：数字/文本升序
- `comp_desc(a,b)`：数字/文本降序

冒泡排序方法：
- `bubble_cfg(obj, start, end, max_passes, comp)`
- `bubble_by(obj, n, comp)`
- `bubble_asc(obj, n)`
- `bubble_desc(obj, n)`
- `bubble_range_by(obj, start, end, comp)`
- `bubble_partial_by(obj, n, max_passes, comp)`

选择排序方法：
- `selection_by(obj, n, comp)`
- `selection_asc(obj, n)`
- `selection_desc(obj, n)`

有序性检查：
- `is_sorted_by(obj, n, comp) -> bool`
- `is_sorted_asc(obj, n) -> bool`
- `is_sorted_desc(obj, n) -> bool`

兼容说明：
- `stdlib/sort.imp` 仍保留为 shim，重导出 `stdlib/sort/mod.imp`

## enum.imp

构造与访问：
- `variant(tag, payload) -> obj`
- `unit(tag) -> obj`
- `tag(value) -> scalar|null`
- `payload(value) -> scalar|null`
- `payload_or(value, fallback) -> scalar`

标签判断与控制：
- `is_tag(value, tag) -> bool`
- `expect_payload(value, tag, msg) -> payload | throw`
- `match_tag(value, tag, when_true, when_false) -> scalar`

## custom_object.imp

基础对象操作：
- `new() -> obj`
- `set(obj, key, value) -> obj`
- `get(obj, key) -> scalar|null`
- `has(obj, key) -> bool`

可配置构建：
- `define(keys, values, n) -> obj`
- `patch(obj, keys, values, n) -> obj`
- `pick(obj, keys, n) -> obj`
- `with2(k1, v1, k2, v2) -> obj`
- `with3(k1, v1, k2, v2, k3, v3) -> obj`

## collections.imp

- `new() -> obj`：创建数字索引 map 集合
- `push(obj, n, value) -> obj`：在索引 `n` 写入 `value`
- `swap(obj, i, j) -> obj`：交换两个索引
- `at(obj, index, fallback) -> scalar`：缺失/空值时返回 fallback
- `clone(obj, n) -> obj`：克隆区间 `[0, n)`
- `reverse(obj, n) -> obj`：双指针原地反转
- `from2(a, b) -> obj`
- `from3(a, b, c) -> obj`
- `from4(a, b, c, d) -> obj`

## iter.imp

- `reduce_sum(obj, n) -> num`：对 `[0, n)` 做求和 fold
- `any_eq(obj, n, target) -> bool`：任一元素等于 target 时返回 true
- `map_mul_scalar(obj, n, factor) -> obj`：映射拷贝并乘以因子

## algo.imp

- `find_index(obj, n, target) -> num`：返回首个索引，否则 `-1`
- `contains(obj, n, target) -> bool`
- `min_value(obj, n, fallback) -> scalar`
- `max_value(obj, n, fallback) -> scalar`

## output.imp

- `join_parts(parts, n, sep, prefix, suffix)`：面向数字索引混合类型集合
- `join_values(obj, keys, n, sep, prefix, suffix)`：按 key 列表拼接值
- `join_pairs(obj, keys, n, kv_sep, part_sep, prefix, suffix)`：按 `key<kv_sep>value` 形式拼接

## 兼容/工具模块

- `io.imp`：`print(value) -> value`
- `object.imp`：`point2`、`pair`、`result_ok`、`result_err`
- `prelude.imp`：平铺兼容导出
