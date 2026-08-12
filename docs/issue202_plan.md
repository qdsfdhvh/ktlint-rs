# Task Plan — Issue #202: over-indent detection + indent parity

> 分支 `fix/indent-overindent-202` → PR #229（3 commits，已 push）。
> 目标：`standard:indent` 全语料与 JVM ktlint 1.8 逐 (file:line:col) 一致。

## 当前真实状态（2026-08-12 实测，正确 normalize）

⚠️ 早期结论「与 JVM 完全一致」**有误**：normalize 曾把 JVM 行的
`path:line:col: message` 当漏报、rs 的 `path:line:col` 当 FP。真实数字：

| 语料 | JVM 报 | 我们报 | agree | FP(我们独有) | missed(漏报) |
|---|---|---|---|---|---|
| ktor (2311) | 4 | 0 | 0 | 0 | 4 |
| okhttp | 2 | 0 | 0 | 0 | 2 |
| nowinandroid (310) | 7360 | 6517 | 6117 | **400** | **1243** |

- 探针 FP=0（ktor/okhttp）是真的，但 0 FP ≠ 一致——漏报一堆
- nowinandroid 6517 里 400 条是**假阳性**（正常模式就有，HEAD 也有，非回归，但确实是 FP）
- 「sweep 表与 JVM 一致」只覆盖函数体内单语句一个形状，不可推广

## 差距分类（已定位形状）

### A. missed — annotation 行（漏报大头，待量化）
我们 check 无条件跳过 `@` 开头行（`trimmed.starts_with('@') → continue`）。
JVM 报 `@Test` 等在错误缩进的 annotation 行（NavigationTest.kt 等 squished 文件）。
修复方向：annotation 行的期望 = 其修饰声明行的期望（不能简单不跳过——要避免
对注释/KDoc 误报）。先验证 JVM 对 annotation 缩进的完整行为。

### B. FP 400 — Scrollbar 风格（nowinandroid core/designsystem/component/scrollbar/*）
形状：
```kotlin
private fun ScrollbarTrack.thumbPosition(   (0)
  dimension: Float,                          (2)
): Float = max(                              (2)   ← 参数列表闭合行 + 表达式体折叠
  a = min(                                   (2)   ← max 的命名参数对齐签名行
    a = dimension / size,                    (4)
    b = 1f,                                  (4)
  ),                                         (2)
  b = 0f,                                    (2)
)
```
0 缩进顶层扩展函数 + `): Float = max(` 单行折叠 + 命名参数对齐。classifier 没建模。
文件：Scrollbar.kt(216) AppScrollbars.kt(118) LazyScrollbarUtilities.kt(31) ThumbExt.kt(21) 等。

### C. missed — ktor-compiler-plugin（4 条）
OpenApiAnalysisExtension.kt:89、ResourceRouteCallInference.kt:58/59、
OpenApiTestRunner.kt:52 — 深层 continuation 形状（(16)should be(12) 等，错 1 层）。

### D. missed — okhttp android-test（2 条）
OkHttpTest.kt:558/559 — (10)should be(8)，错 1 层。

## 修复顺序（每步跑 scripts/indent-diff.sh 全量对比）

1. **A. annotation 行**（漏报大头，相对独立，先做）
2. **B. Scrollbar 400 FP**（正常模式就有的假阳性，修完 normal/probe 都干净）
3. **C. ktor-compiler-plugin 4 条**（最难，深层 continuation）
4. **D. okhttp 2 条**（与 C 可能同源）
5. 全量验证：indent-diff 三语料全 0 FP + 0 missed（或记录残余）
   + cargo test + oracle-diff + format-diff + mutation + perf-gates

## 关键命令 / 陷阱

- 全量对比：`./scripts/indent-diff.sh ktor|okhttp|nowinandroid`
- **缓存陷阱**：`.cache` 不清会复用旧结果（RULES_VERSION 常量不变）。
  探针跑前必须 `find tests/fixtures -name .cache -type d -exec rm -rf {} +`
- 测试：`cargo test issue202`（indentation.rs 的 ast_matrix 模块）
- JVM oracle：`ktlint`（homebrew 1.8.0）
- rust 工具链被 rustup 代理弄坏：用绝对路径
  `/Users/seiko/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`
- 分支已有 3 commits（continuation 修复 / class-body / 多合法形状跳过），
  新修复继续追加 commit，PR #229 不 amend 已 push 的
