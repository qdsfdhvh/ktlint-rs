# Task Plan — Issue #202: over-indent detection + indent parity

> 分支 `fix/indent-overindent-202` → PR #229。
> 目标：`standard:indent` 全语料与 JVM ktlint 1.8 逐 (file:line:col) 一致。

## 当前真实状态（2026-08-12 实测，LC_ALL=C + 正确 normalize）

⚠️ 早期数字有误：`comm` 在 macOS 默认 locale（UTF-8 collation）下对
python-sorted 输入会错配行，把 agreed 行计成 FP/missed。**所有对比必须
`export LC_ALL=C`**。真实基线：

| 语料 | JVM 报 | 我们报 | agree | FP(我们独有) | missed(漏报) |
|---|---|---|---|---|---|
| ktor (2311) | 4 | 0 | 0 | 0 | 4 |
| okhttp | 2 | 0 | 0 | 0 | 2 |
| nowinandroid (310) | 7360 | 6517 | 6503 | **14** | **857** |

## 已修复（2026-08-12，annotation 行，A 步完成）

- **A. annotation 行**：此前 check 无条件跳过 `@` 开头行；JVM 报错误缩进的
  annotation（NavigationTest.kt 等 squished 文件，missed 大头 ~352 条）。
  现在：独立 annotation 行（行内只有 `@Name(...)`，`standalone_annotation_row`）
  的期望 = 其修饰的声明行的期望（下一行代码行，跳过空行/注释/KDoc/后续
  annotation 及其多行参数）。副作用修复：
  - `contains("get(")`/`contains("set(")` 跳过会误杀 `@Target(...)`——改为
    `accessor_row`（仅匹配 accessor 头部行）；`@InternalAPI set(v) {` 等
    注解+accessor 同行行仍跳过（accessor 期望未建模）。
  - `@JvmField val x` 等 annotation+declaration 同行行视为普通声明行。
  - `row_starts_statement` 接受 `source_file`（顶层注解类）与
    annotation 行/其下声明头；不再用裸 `is_decl_header`（会误判
    `suspend …` continuation）。
  - **error 文件（tree-sitter 有 error）**: `line_expected` 全用 line-scan
    值（AST 期望被 error recovery 污染）。此前该 fallback 只在 probe 置信
    度生效，under-indent 路径仍用垃圾 AST 值——14 条 FP 全因此。

### 新状态（LC_ALL=C，probe 模式）

| 语料 | JVM 报 | 我们报 | agree | FP | missed |
|---|---|---|---|---|---|
| ktor | 4 | 0 | 0 | **0** | 4 |
| okhttp | 2 | 0 | 0 | **0** | 2 |
| nowinandroid | 7360 | 6891 | 6891 | **0** | **469** |

NavigationTest.kt 单文件：147/147 全对齐（0 FP / 0 missed）。
非 probe（发货模式）全语料 0 FP。门禁：cargo test 全绿、oracle-diff 108
cases ALL MATCH、mutation 40 files 通过、spotless oracle match；perf-gates
的 per-file lint 在本机噪声下基线同样 fail（非回归）。

## 差距分类（剩余 missed 469，全部为 continuation 形状）

### B. nowinandroid 深层 continuation（大头）
Type.kt(144) UserNewsResourcesTestData.kt(76) NewsResourcesTestData.kt(47)
FollowableTopicTestData.kt(33) DesignSystemDetector.kt(24) …
形状：squished 文件里 `TextStyle(`/`listOf(` 等命名参数 continuation 行
（`lineHeight = 24.sp,` 等）JVM 报 should-be-4，我们不报。

### C. ktor-compiler-plugin（4 条）
OpenApiAnalysisExtension.kt:89、ResourceRouteCallInference.kt:58/59、
OpenApiTestRunner.kt:52 — 深层 continuation 形状（错 1 层）。

### D. okhttp android-test（2 条）
OkHttpTest.kt:558/559 — (10)should be(8)，错 1 层。

### E. 杂项小类
- 独立 `//` comment 行错误缩进（JVM 报，nowinandroid 仅 1 条，ktor 无）
- annotation+accessor 同行/accessor 上的注解（期望未建模，保守跳过）

## 修复顺序（每步跑 scripts/indent-diff.sh 全量对比，注意 LC_ALL=C）

1. ~~A. annotation 行~~ ✅ 已提交
2. **B. nowinandroid 深层 continuation**（下一步，最难）
3. **C. ktor-compiler-plugin 4 条**（与 D 可能同源）
4. **D. okhttp 2 条**
5. 全量验证：indent-diff 三语料全 0 FP + 0 missed（或记录残余）
   + cargo test + oracle-diff + mutation + perf-gates

## 关键命令 / 陷阱

- 全量对比：`export LC_ALL=C && ./scripts/indent-diff.sh ktor|okhttp|nowinandroid`
  （**comm 必须 LC_ALL=C**，否则数字虚高）
- **缓存陷阱**：`.cache` 不清会复用旧结果（RULES_VERSION 常量不变）。
  探针跑前必须 `find tests/fixtures -name .cache -type d -exec rm -rf {} +`
- 测试：`cargo test issue202`（indentation.rs 的 ast_matrix 模块）
- JVM oracle：`ktlint`（homebrew 1.8.0）
- rust 工具链被 rustup 代理弄坏：用绝对路径
  `/Users/seiko/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`；
  fmt 需 `PATH="/…/bin:$PATH" cargo fmt`（cargo-fmt 子命令走 PATH）
- 分支已有 4 commits（continuation / class-body / 多合法形状跳过 / A 步），
  新修复继续追加 commit，PR #229 不 amend 已 push 的
