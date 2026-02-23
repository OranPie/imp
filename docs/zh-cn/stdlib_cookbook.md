# 标准库 Cookbook

## 1) 动态 key 读写 + 默认值

```imp
#call std_map::set args="local::cfg,local::k_timeout,local::timeout" out=local::cfg;
#call std_map::get_or args="local::cfg,local::k_timeout,local::fallback" out=local::timeout;
```

## 2) 先校验再计算

```imp
#call std_valid::require_positive args="local::qty,local::msg" out=local::qty_checked;
#call std_calc::taxed_total args="local::qty_checked,local::unit,local::disc,local::tax" out=local::total;
```

## 3) 安全比值（不会崩溃）

```imp
#call std_calc::ratio_or args="local::part,local::total,local::fallback" out=local::ratio;
```

## 4) nullable 转 Result

```imp
#call std_res::from_nullable args="local::maybe_user,local::err" out=local::res;
#call std_res::unwrap_or args="local::res,local::anon" out=local::user;
```

## 5) 文本拼接

```imp
#call std_str::to_text args="local::score" out=local::score_txt;
#call std_str::join_colon args="local::label,local::score_txt" out=local::line;
```

## 6) 表达式风格分支

```imp
#call std_ctrl::if_else args="local::is_admin,local::admin_path,local::user_path" out=local::path;
```

## 7) 完整流水线草图

```imp
#call std_map::require args="local::req,local::k_user,local::msg_missing" out=local::user;
#call std_valid::require_non_empty_text args="local::user,local::msg_empty" out=local::user;
#call std_res::ok args="local::user" out=return::value;
#call core::exit;
```

## 8) 可直接运行的复杂示例

- `examples/complex_billing_pipeline.imp`
- `examples/complex_profile_validation.imp`
- `examples/complex_retry_flow.imp`
- `examples/bubble_sort_demo.imp`
- `examples/sort_custom_comp_demo.imp`
- `examples/sort_config_demo.imp`
- `examples/enum_custom_object_demo.imp`
- `examples/collections_algo_demo.imp`
- `examples/output_collections_demo.imp`

## 9) 数字索引 map 的冒泡排序

```imp
#call std_sort::bubble_asc args="local::arr,local::n" out=local::arr;
```

## 10) 自定义比较器排序

```imp
#call core::fn::begin name=main::my_comp args="a,b" retshape="scalar";
#call core::lt a=arg::b b=arg::a out=return::value;
#call core::exit;
#call core::fn::end;
#call std_sort::bubble_by args="local::arr,local::n,main::my_comp" out=local::arr;
```

## 11) 排序配置（区间 + 限制轮数）

```imp
#call std_sort::bubble_partial_by args="local::arr,local::n,local::passes,std_sort::comp_asc" out=local::arr;
#call std_sort::bubble_range_by args="local::arr,local::start,local::end,std_sort::comp_asc" out=local::arr;
#call std_sort::is_sorted_asc args="local::arr,local::n" out=local::ok;
```

## 12) 混合集合的参数化输出

```imp
#call std_output::join_parts args="local::parts,local::n,local::sep,local::prefix,local::suffix" out=local::txt_a;
#call std_output::join_values args="local::obj,local::keys,local::n,local::sep,local::prefix,local::suffix" out=local::txt_b;
#call std_output::join_pairs args="local::obj,local::keys,local::n,local::kv_sep,local::part_sep,local::prefix,local::suffix" out=local::txt_c;
```

## 13) enum + 自定义对象组合

```imp
#call std_cobj::define args="local::keys,local::values,local::n" out=local::payload;
#call std_enum::variant args="local::ok_tag,local::payload" out=local::res;
#call std_enum::expect_payload args="local::res,local::ok_tag,local::msg" out=local::obj;
```

## 14) 集合与算法辅助

```imp
#call std_col::from4 args="local::a,local::b,local::c,local::d" out=local::items;
#call std_algo::find_index args="local::items,local::n,local::target" out=local::idx;
#call std_iter::reduce_sum args="local::items,local::n" out=local::sum;
```
