# AGENTS.md — ktlint-rs 开发与 LSP 指南

> **设计目标**: AI Agent 的快速 Kotlin 预检工具  
> **核心理念**: 纯 Rust，0 JVM，<1s 扫描  
> **详细**: 见 [docs/DESIGN.md](docs/DESIGN.md)

## Agent LSP配置

推荐使用 **rust-analyzer** 作为 LSP。以 rust-analyzer 标准方式调用即可：

- **类型检查**: `rust-analyzer diagnostics` 或 `cargo check`
- **自动补全**: rust-analyzer 标准 completion API
- **跳转定义**: rust-analyzer 标准 goto-definition
- **查找引用**: rust-analyzer 标准 find-references

Agent 开发时的快捷命令:

```bash
# 快速类型检查 (推荐，比 cargo check 快 2x)
cargo check

# 严格 lint (启用所有警告)
cargo clippy --all-features -- -D warnings

# 运行全部测试
cargo test --all-features

# 格式化代码
cargo fmt --all

# 自检 (检测 rust lint 后格式化)
cargo fmt --all -- --check
```

## 项目约束

- **纯 Rust**: 零 JVM / kotlinc / Gradle 依赖
- **二进制 < 15MB**: release 模式
- **启动 < 50ms**: 无 daemon / rayon pool
- **内存释放**: lint 完成后即时释放
- **禁止 daemon**: 进程必须有明确退出点
EOF
git add AGENTS.md && git commit -m "docs: agent LSP guide — rust-analyzer + dev commands

- Removed unused rust-project.json (Cargo project)
- Added agent quick commands (check, clippy, test, fmt)
- Kept existing development workflow" && git push
