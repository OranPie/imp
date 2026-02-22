# Stdlib Cookbook

## 1) Dynamic map set/get with fallback

```imp
#call std_map::set args="local::cfg,local::k_timeout,local::timeout" out=local::cfg;
#call std_map::get_or args="local::cfg,local::k_timeout,local::fallback" out=local::timeout;
```

## 2) Validate then calculate

```imp
#call std_valid::require_positive args="local::qty,local::msg" out=local::qty_checked;
#call std_calc::taxed_total args="local::qty_checked,local::unit,local::disc,local::tax" out=local::total;
```

## 3) Safe ratio without crashing

```imp
#call std_calc::ratio_or args="local::part,local::total,local::fallback" out=local::ratio;
```

## 4) Result from nullable

```imp
#call std_res::from_nullable args="local::maybe_user,local::err" out=local::res;
#call std_res::unwrap_or args="local::res,local::anon" out=local::user;
```

## 5) Rich text assembly

```imp
#call std_str::to_text args="local::score" out=local::score_txt;
#call std_str::join_colon args="local::label,local::score_txt" out=local::line;
```

## 6) Branch expression style

```imp
#call std_ctrl::if_else args="local::is_admin,local::admin_path,local::user_path" out=local::path;
```

## 7) Full pipeline sketch

```imp
#call std_map::require args="local::req,local::k_user,local::msg_missing" out=local::user;
#call std_valid::require_non_empty_text args="local::user,local::msg_empty" out=local::user;
#call std_res::ok args="local::user" out=return::value;
#call core::exit;
```

## 8) Complex runnable examples

- `examples/complex_billing_pipeline.imp`
- `examples/complex_profile_validation.imp`
- `examples/complex_retry_flow.imp`
