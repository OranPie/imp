# Imp-Core v2.0 规格（实现版）

## 执行流水线

1. 将 `#call` 语句解析为 AST。
2. 展开编译期注解（如 `@safe core::div`）。
3. 编译为 slot-based IR（`Instr`）。
4. 可选序列化为 AOT 字节码（`.impc`）。
5. 在 VM Frame 上执行。

## 源码语句

```imp
#call [@anno ...] target key=value key=value ... ;
```

## Atom

- `null`
- `true` / `false`
- 数字（`f64`）
- 字符串
- 引用（`namespace::name`）

## 命名空间

- `local::`：局部槽
- `arg::`：参数槽
- `return::`：返回槽
- `err::`：错误槽
- 其他命名空间：全局槽（如 `main::`、`mod::`、import alias）

## 编译期行为

- 函数定义通过 `core::fn::begin` / `core::fn::end` 建立。
- `core::*` 目标直接降级为 IR 指令。
- 非 `core::*` 目标降级为 `Instr::Invoke`。
- label 在编译期解析为具体 PC。
- `@safe core::div` 会展开为 try/jump/fallback 序列。

## 运行期行为

- Frame 字段：`code`、`pc`、`locals`、`args`、`ret`、`err`、`try_stack`、`meta`。
- 槽访问是索引访问（无运行时字符串解析）。
- `Exit` 根据函数元数据校验返回形状。
- `Throw` 向最近的 try handler 回退；无 handler 则向上传播。
- 跨模块函数调用通过外部函数句柄桥接。
- import 模块在 VM 生命周期内按路径缓存导出。

## AOT 字节码（`.impc`）

- 魔数：`IMPC`
- 版本：`1`
- 可编码完整 `CompiledModule` 图（含导入模块）
- 支持当前 IR 指令集的 roundtrip
- 解码阶段会报告 magic/version/tag/EOF 错误

## 当前扩展

- `core::host::print`
- 对象：`core::obj::new` / `set` / `get` / `has`
- 字符串：`core::str::concat` / `len`
- 模块元信息：`core::import` / `core::mod::export`

## 标准库定位

- 标准库以 `.imp` 模块实现（`stdlib/`）
- 推荐使用命名空间导入（不是平铺全局）
- `stdlib/prelude.imp` 保留兼容层
- VM 内核保持精简，高阶能力放在语言层模块

## JIT 后端

- VM 提供运行期 JIT（direct-threaded step plan）
- 默认开启（`VmConfig.enable_jit = true`）
- JIT 覆盖数据/算术/比较/控制流/invoke/return/exit/throw/try/object/host-print
- 可通过 `VmConfig.enable_jit = false` 或 `IMP_NO_JIT=1` 关闭

## CLI

- `imp run <file.imp|file.impc> [--strict-bytecode]`
- `imp dump-ir <file.imp|file.impc> [--strict-bytecode]`
- `imp build <file.imp> [-o out.impc]`
