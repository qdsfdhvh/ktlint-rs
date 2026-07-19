# ktlint-rs 设计目标

## 核心定位

**ktlint-rs 是 AI 编程 Agent 的快速 Kotlin 预检工具。**

Agent 在提交代码前需要检查代码质量。调用 JVM 工具链（Gradle/ktlint/detekt）需要：
- JVM 冷启动: 2-5 秒
- Gradle 配置: 3-10 秒
- ktlint 运行: 5-30 秒

**总计: 10-45 秒/次**

ktlint-rs 的目标是把这些时间压缩到 **<1 秒**：

| 操作 | JVM 工具链 | ktlint-rs |
|---|---|---|
| 启动 | 2-5s (JVM) | <2ms |
| lint nowinandroid | 7-30s | **0.29s** |
| detekt nowinandroid | 10-60s | **0.36s** |
| 二进制大小 | N/A | **11MB** |

## 不追求 100% 对齐

ktlint-rs 不需要和 JVM ktlint/detekt 完全一致，它只需要**提前发现大部分问题**，让 agent 少调用 JVM 工具：

| 规则 | 对齐率 | 效果 |
|---|---|---|
| indent | 100% | ✅ 完全覆盖 |
| blank-line-before-declaration | 90%+ | ✅ 覆盖大部分 |
| no-empty-first-line | 100% | ✅ 完全覆盖 |
| annotation | 76% | 🟡 基本覆盖 |

Agent 工作流：
1. ktlint-rs 快速扫描 (<1s) → 发现大部分问题，agent 直接修复
2. 如果 ktlint-rs 通过，大概率 JVM 检查也会通过
3. 少数漏检的情况，JVM 工具链作为最终兜底

## 纯 Rust 约束

- ❌ 不依赖 JVM / kotlinc / Gradle
- ❌ 不依赖外部进程
- ✅ 单一二进制 < 15MB
- ✅ 启动 < 50ms
- ✅ 内存即时释放
