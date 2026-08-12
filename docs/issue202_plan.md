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

## 当前状态（LC_ALL=C，probe 模式，C/D 步完成）

| 语料 | JVM 报 | 我们报 | agree | FP | missed |
|---|---|---|---|---|---|
| ktor | 4 | 4 | 4 | **0** | **0** |
| okhttp | 2 | 2 | 2 | **0** | **0** |
| nowinandroid | 7360 | 7360 | 7360 | **0** | **0** |

**三语料全对齐（7366/7366）**。非 probe（发货模式）同样 0 FP。
门禁：cargo test 768 全绿、oracle-diff 108 ALL MATCH、mutation 40 files
通过、spotless oracle match、fmt clean；perf-gates 的 per-file lint 在本机
噪声下基线同样 fail（非回归）。

NavigationTest.kt 单文件：147/147 全对齐。非 probe（发货模式）全语料 0 FP。
门禁：cargo test 全绿、oracle-diff 108 ALL MATCH、mutation 40 files 通过、
spotless oracle match、fmt clean；perf-gates 的 per-file lint 在本机噪声下
基线同样 fail（非回归）。

## 差距分类（全部清零）

### C. ktor-compiler-plugin（4 条，与 D 可能同源）
OpenApiAnalysisExtension.kt:89、ResourceRouteCallInference.kt:58/59、
OpenApiTestRunner.kt:52 — 深层 continuation 形状（错 1 层）。

#### C 步调查记录（2026-08-12，全部尝试已回退，保持 0 FP）
二进制表达式 continuation 的 JVM 规则（探针实证）：
- **EOL 运算符**（`a() &&\n    b()`、`"a" +\n    "b"`）→ continuation = 表达式
  首行 + 1 层，双向严格检查（8 正确，12 报 should-be-8，0 报 should-be-8）。
- **行首运算符**（`first\n    + second`）→ 期望 = 语句行，+1 层容忍
  （4/8 接受，0/12 报 should-be-4）。`return` 下则严格 4（8 报错）——
  val/return 行为不一致，原因未定位。
- 首行锚点：`byteString =\n  (\n    "a" +` → 括号子行 12，cont 14（is=2）。

尝试的实现（均回退）：AST 二进制分支（首行+1）、scan 运算符列表 lift、
probe 置信度放宽。失败原因：
- scan 运算符列表误伤 `import libcurl.*`（`*`）、`configuration++`（`+`）、
  行首 `-`/`+` 字面量。
- AST 分支对二进制表达式任意后代（块内 `}`、`ByteReadChannel.Empty` 等）
  都 lift → under-indent FP。需限定“行开始新 operand”+直接父节点。
- probe 置信度放宽暴露所有未建模锚点（括号-after-`=` 形状的 paren 上下文
  期望本身就不对：is=2 时 `(` 应 10 我们给 8）。
- 结论：4 条 ktor + 2 条 okhttp 维持 residual，0 FP 优先（parity gate）。

### D. okhttp android-test（2 条）
OkHttpTest.kt:558/559 — (10)should be(8)，错 1 层。

## 修复顺序（全部完成 ✅）

1. ~~A. annotation 行~~ ✅（31e300b）
2. ~~B. nowinandroid 深层 continuation~~ ✅（fc1bc56）
3. ~~C. ktor-compiler-plugin 4 条~~ ✅（本轮）
4. ~~D. okhttp 2 条~~ ✅（本轮，与 C 同源）
5. 全量验证 ✅：三语料 7366/7366 全对齐（0 FP / 0 missed），
   cargo test 768 全绿 + oracle-diff 108 ALL MATCH + mutation 40 通过
   + spotless oracle match + fmt clean

## C/D 步修复内容（本轮）

关键发现：**命名参数 `=`-值 lift 与二进制 continuation 都是
code-style 相关的**（探针+真实文件比对+反编译确认）：
- ktor fixture 的 `.editorconfig` 有 `ktlint_code_style = intellij_idea`
  → 对齐的命名参数值被接受（不 lift）；nowinandroid/okhttp 用默认
  （KtlintOfficial）→ lift。此前的「TestEngineMultipart 之谜」是 code
  style 差异，不是形状差异。
- 实现：`Indentation` 持有 `code_style`（registry 传入），`ast_expected`
  用 thread_local `CODE_STYLE` 读取；`=`-值 lift 仅非 IntelliJIdea 生效，
  裸标识符/数字字面量仍不 lift。
- 二进制 continuation（EOL 运算符 `a() &&` 或行首运算符 `+ second`）：
  AST 分支 = 表达式首行 + 1，`binary_continuation_row` 限定「statement
  直接表达式」（return/jump_expression、property、fun-body、expression
  statement、assignment），排除 if/while 条件、调用参数、链式；scan 模型
  加运算符检测（排除 `.*` 导入、`*/` KDoc、`++`/`--`、`::`）+ 链行保持
  层级；probe 置信度对二进制续行放宽。
- `//` comment 行：按下一代码行期望检查，仅报 under-indent；豁免：
  注释块内、上一非空行同缩进、下一行是闭合括号、注释掉的代码
  （`//` 后 2+ 空格）。


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
