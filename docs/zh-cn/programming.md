# Imp 编程指南

本指南聚焦如何高效编写实际可用的 `.imp` 程序。

## 1) 心智模型

Imp-Core v2 的执行流程：

1. 解析 `#call` 语句为 AST。
2. 展开编译期注解（如 `@safe`）。
3. 编译为基于 slot 的 IR。
4. 在 VM 上执行（默认启用 JIT，可回退解释器）。

日常开发主要关注 `#call` 组合和模块 API。

## 2) 唯一语句形式

Imp 源码层只有一种语句：

```imp
#call [@anno ...] target key=value key=value ... ;
```

- `target` 可以是 `core::...` 或 `alias::function`
- `key=value` 是命名参数
- 每行以 `;` 结尾

## 3) Atom 与引用

参数中的值（atom）支持：

- `null`
- `true` / `false`
- 数字（`f64`）
- 字符串（`"text"`）
- 引用（`namespace::name`）

常用命名空间：

- `local::` 当前函数局部变量
- `arg::` 形参槽
- `return::` 返回槽
- `err::` 错误槽
- `main::` 与其他命名空间会映射到全局/模块导出

## 4) 函数定义

使用 `core::fn::begin` / `core::fn::end` 定义函数：

```imp
#call core::fn::begin name=main::sum2 args="a,b" retshape="scalar";
#call core::add a=arg::a b=arg::b out=return::value;
#call core::exit;
#call core::fn::end;
```

规则：

- `name` 一般放在 `main::...`
- `args` 是 CSV，会绑定到 `arg::...`
- `retshape` 在 `core::exit` 时做校验
- 每条返回路径都要 `core::exit`

## 5) 函数调用

- `core::...` 在编译期直接降级为 IR 指令
- 非 `core` 调用会编译为 `Instr::Invoke`

示例：

```imp
#call std_math::sum3 args="local::x,local::y,local::z" out=local::total;
```

## 6) 控制流

核心控制流：

- `core::label name="L"`
- `core::jump target="L"`
- `core::br cond=<ref> then="L1" else="L2"`

循环模板：

```imp
#call core::label name="loop";
#call core::lt a=local::i b=local::n out=local::keep;
#call core::br cond=local::keep then="body" else="done";
#call core::label name="body";
#call core::add a=local::i b=local::one out=local::i;
#call core::jump target="loop";
#call core::label name="done";
```

## 7) 异常与安全

抛出错误：

```imp
#call core::throw code="bad_input" msg="value invalid";
```

手动异常处理：

```imp
#call core::try::push handler="on_err";
#call core::div a=local::a b=local::b out=local::q;
#call core::try::pop;
#call core::jump target="ok";
#call core::label name="on_err";
#call core::const out=local::q value=null;
#call core::label name="ok";
```

快速安全除法：

```imp
#call @safe core::div a=local::a b=local::b out=local::q;
```

`@safe` 是编译期宏展开，不增加运行期反射成本。

## 8) 模块组织

导入：

```imp
#call core::import alias="std_map" path="../stdlib/map.imp";
```

导出：

```imp
#call core::mod::export name="set" value=main::set;
#call core::exit;
```

建议：

- 一个模块只做一类事情
- 导出小而稳的函数
- 优先使用命名空间导入，减少全局污染

## 9) 标准库优先

日常开发建议优先组合 `.imp` 标准库：

- `math.imp`、`bool.imp`、`control.imp`
- `map.imp`、`string.imp`、`result.imp`
- `validate.imp`、`calc.imp`
- `sort/mod.imp`、`collections.imp`、`iter.imp`、`algo.imp`、`output.imp`

这样可以保持 VM 内核简洁，同时在语言层获得高阶能力。

## 10) 常见模式

### 10.1 动态 key 映射

```imp
#call std_map::set args="local::obj,local::key,local::value" out=local::obj;
#call std_map::get_or args="local::obj,local::key,local::fallback" out=local::v;
```

### 10.2 先校验再计算

```imp
#call std_valid::require_positive args="local::qty,local::msg" out=local::qty_checked;
#call std_calc::taxed_total args="local::qty_checked,local::unit,local::disc,local::tax" out=local::total;
```

### 10.3 Result 流

```imp
#call std_res::from_nullable args="local::maybe_user,local::err" out=local::res;
#call std_res::unwrap_or args="local::res,local::fallback" out=return::value;
#call core::exit;
```

## 11) 调试建议

- 用 `core::host::print`（或 `std_io::print`）观察中间值
- 用 `imp dump-ir file.imp` 查看降级后的 IR
- 用 `IMP_NO_JIT=1` 对比解释器行为
- label 命名保持清晰（如 `loop`、`done`、`on_err`）

## 12) 性能建议

- 复用常量到 `local::`，避免重复构造
- 用标准库函数封装重复逻辑
- 热路径尽量保持数值计算和低分支
- 生产部署可使用 `.impc` 提升启动路径效率

## 13) 延伸阅读

- `docs/zh-cn/spec-v2.md`
- `docs/zh-cn/stdlib.md`
- `docs/zh-cn/stdlib_reference.md`
- `docs/zh-cn/stdlib_cookbook.md`
- `examples/complex_billing_pipeline.imp`
- `examples/sort_custom_comp_demo.imp`
