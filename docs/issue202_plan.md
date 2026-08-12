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

## 已修复（2026-08-12）

### A. annotation 行 ✅（已提交 31e300b）
独立 annotation 行（行内只有 `@Name(...)`，`standalone_annotation_row`）的
期望 = 其修饰的声明行的期望（下一行代码行，跳过空行/注释/KDoc/后续
annotation 及其多行参数）。副作用修复：
- `contains("get(")`/`contains("set(")` 跳过会误杀 `@Target(...)`——改为
  `accessor_row`；`@InternalAPI set(v) {` 等注解+accessor 同行行仍跳过。
- `@JvmField val x` 等 annotation+declaration 同行行视为普通声明行。
- `row_starts_statement` 接受 `source_file` 与 annotation 行/其下声明头。
- **error 文件（tree-sitter 有 error）**: `line_expected` 全用 line-scan
  值（AST 期望被 error recovery 污染）——14 条 FP 全因此。

### B. nowinandroid 深层 continuation ✅（未提交，本次会话）
- **`.kts` 按扩展名跳过**（`check_with_path`，引擎给 indent 传 path）：
  内容启发式把只有顶层 val 的 .kt（Type.kt 等 test-data 文件）误判成 KTS
  整个跳过 → 修掉 ~311 条 missed。
- **`"` 开头行不再跳过**：字符串 continuation 行（`"foo" +`）照常检查
  → 修掉 ~130 条。
- **raw-string 重构**：opener 行（`value = """`）照常检查；内容行跳过；
  纯 delimiter/continuation 行（`"""`、`}""".trimIndent()`、`|content"""`、
  `"""content`）由 `raw_string_delimiter_row` 跳过（opener 判定 = `"""` 前
  的代码以 `=`/`(` 结尾）。
- **顶层 property 的 getter/setter**：`ast_expected` 的 getter 分支加祖先
  property_declaration 回退（原只找 class_body 兄弟）；scan 模型对
  `get(`/`set(` 前缀行在属性行后 +is（error 文件用 scan）。

### 试过但回退（ktor FP，记录教训）
- **call-site 命名参数 `=` 值 +1**（V3-V7 探针都显示 JVM lift，但 ktor
  TestEngineMultipartTest.kt 同形状 JVM 却接受对齐；找不出区分因子，+
  9 FP）→ 回退，记录为残余。
- **`//` comment 行检查**（同缩进豁免规则 13/13 探针吻合，但 ktor 91 FP，
  如 accessor 前注释）→ 回退，记录为残余。

## 当前状态（LC_ALL=C，probe 模式，未提交 B 步）

| 语料 | JVM 报 | 我们报 | agree | FP | missed |
|---|---|---|---|---|---|
| ktor | 4 | 0 | 0 | **0** | 4 |
| okhttp | 2 | 0 | 0 | **0** | 2 |
| nowinandroid | 7360 | 7346 | 7346 | **0** | **14** |

NavigationTest.kt 单文件：147/147 全对齐。非 probe（发货模式）全语料 0 FP。
门禁：cargo test 全绿、oracle-diff 108 ALL MATCH、mutation 40 files 通过、
spotless oracle match、fmt clean；perf-gates 的 per-file lint 在本机噪声下
基线同样 fail（非回归）。

## 差距分类（剩余 missed 20 = nowinandroid 14 + ktor 4 + okhttp 2）

### nowinandroid 14（B 步残余）
- TestUserDataRepository.kt:81-85 — `viewedNewsResources =\nif (viewed) {`
  命名参数 `=` 值（JVM lift +1，见回退教训）
- ForYouScreenTest.kt:157-162, 204-206 — `onboardingUiState =\n
  OnboardingUiState.Shown(` 同形状（含 `// Follow one topic` comment 行）

### C. ktor-compiler-plugin（4 条，与 D 可能同源）
OpenApiAnalysisExtension.kt:89、ResourceRouteCallInference.kt:58/59、
OpenApiTestRunner.kt:52 — 深层 continuation 形状（错 1 层）。

### D. okhttp android-test（2 条）
OkHttpTest.kt:558/559 — (10)should be(8)，错 1 层。

## 修复顺序（每步跑 scripts/indent-diff.sh 全量对比，注意 LC_ALL=C）

1. ~~A. annotation 行~~ ✅
2. ~~B. nowinandroid 深层 continuation~~ ✅（14 残余为 `=` 值 lift 形状）
3. **C. ktor-compiler-plugin 4 条**（下一步）
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
- 分支已有 5 commits（continuation / class-body / 多合法形状跳过 / A 步 /
  B 步），新修复继续追加 commit，PR #229 不 amend 已 push 的
