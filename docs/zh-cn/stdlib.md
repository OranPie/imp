# Imp 标准库导读

Imp 标准库使用 `.imp` 模块实现，位于 `stdlib/`。
目标是提升日常编码速度，同时保持 VM 内核精简。

## 推荐导入方式（命名空间）

```imp
#call core::import alias="std_math" path="../stdlib/math.imp";
#call core::import alias="std_ctrl" path="../stdlib/control.imp";
#call core::import alias="std_map" path="../stdlib/map.imp";
#call core::import alias="std_str" path="../stdlib/string.imp";
#call core::import alias="std_res" path="../stdlib/result.imp";
#call core::import alias="std_valid" path="../stdlib/validate.imp";
#call core::import alias="std_calc" path="../stdlib/calc.imp";
#call core::import alias="std_sort" path="../stdlib/sort/mod.imp";
#call core::import alias="std_col" path="../stdlib/collections.imp";
#call core::import alias="std_iter" path="../stdlib/iter.imp";
#call core::import alias="std_algo" path="../stdlib/algo.imp";
#call core::import alias="std_output" path="../stdlib/output.imp";
```

仅在兼容旧脚本时使用 `stdlib/prelude.imp`。

## 模块总览

- `bool.imp`：逻辑运算、相等判断、null 判断
- `math.imp`：算术、聚合、范围判断
- `control.imp`：if/coalesce/assert/guard
- `map.imp`：对象映射（支持动态 key）
- `string.imp`：文本拼接、长度、格式拼接
- `result.imp`：`ok/err/unwrap` 流
- `validate.imp`：输入校验（范围、正数、非空、必需键）
- `calc.imp`：业务计算（税费、折扣、百分比等）
- `sort/mod.imp`：排序与 comparator 框架
- `enum.imp`：标签化值（variant/unit/tag）
- `custom_object.imp`：可配置对象构建
- `collections.imp`：数字索引集合工具
- `iter.imp`：常见遍历/聚合
- `algo.imp`：搜索与统计
- `output.imp`：参数化字符串输出
- `io.imp` / `object.imp`：兼容与基础工具

## 复杂示例

- `examples/complex_billing_pipeline.imp`
- `examples/complex_profile_validation.imp`
- `examples/complex_retry_flow.imp`
- `examples/bubble_sort_demo.imp`
- `examples/sort_custom_comp_demo.imp`
- `examples/collections_algo_demo.imp`
- `examples/output_collections_demo.imp`

## 进一步阅读

- `docs/zh-cn/stdlib_reference.md`（完整 API）
- `docs/zh-cn/stdlib_cookbook.md`（组合式用法）
- `docs/zh-cn/programming.md`（完整编程流程）
