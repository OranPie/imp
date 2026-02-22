# Stdlib Cookbook

Small ready-to-copy patterns for everyday coding.

## 1) Pick fallback when missing

```imp
#call std_map::get_or args="local::settings,local::k_theme,local::default_theme" out=local::theme;
```

## 2) Guard input and crash early

```imp
#call std_ctrl::require_not_null args="local::user_id,local::msg" out=local::uid;
```

## 3) Branch without writing labels

```imp
#call std_ctrl::if_else args="local::is_admin,local::admin_value,local::user_value" out=local::role_value;
```

## 4) Build display message

```imp
#call std_str::concat args="local::prefix,local::name" out=local::line;
```

## 5) Clamp numeric input

```imp
#call std_math::clamp args="local::score,local::min_score,local::max_score" out=local::safe_score;
```

## 6) Result-style return

```imp
#call std_res::ok args="local::payload" out=return::value;
#call core::exit;
```

## 7) Fallback for failed result

```imp
#call std_res::unwrap_or args="local::result,local::fallback" out=local::value;
```

## 8) Compose namespaced imports

```imp
#call core::import alias="std_math" path="../stdlib/math.imp";
#call core::import alias="std_map" path="../stdlib/map.imp";
#call core::import alias="std_res" path="../stdlib/result.imp";
```

## 9) Keep old scripts running

```imp
#call core::import alias="std" path="../stdlib/prelude.imp";
```

Use this only for compatibility; prefer namespaced modules in new files.
