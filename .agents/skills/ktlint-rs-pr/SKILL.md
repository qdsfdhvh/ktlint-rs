---
name: ktlint-rs-pr
description: 创建 ktlint-rs 项目的 PR — conventional commits + Closes keyword + checklist body。适用于提交代码变更到 qdsfdhvh/ktlint-rs 仓库。
---

# ktlint-rs PR 创建指南

## 分支命名

```
<type>/<short-desc>
```

| Type | 用途 |
|------|------|
| `feat/` | 新功能 |
| `fix/` | Bug 修复 |
| `docs/` | 文档变更 |
| `ci/` | CI/CD |
| `refactor/` | 重构 |
| `test/` | 测试 |
| `chore/` | 杂项（版本 bump、gitignore 等） |

例：`fix/issue45-correctness-gaps`、`feat/variable-naming-rule`、`chore/bump-0.1.1`

## Commit 格式

遵循 conventional commits：

```
<type>: <简短描述>

- 要点 1
- 要点 2
```

例：
```
fix: use CST parent to detect unary minus, add edge case tests

- Check if '-' node parent is 'unary_expression' in CST for accurate detection
- Covers: = -640f, (-1), foo(1,-2), x=-2, second=-2
- Add tests: unary minus attached to equals, after assignment no space
```

## PR Body 模板

```markdown
## Summary

<一句话描述这个 PR 做了什么>

## Changes

- <变更点 1>
- <变更点 2>
- <变更点 3>

## Related Issues

Closes #<issue-number>

## Verification

- <测试结果>
```

**关键规则：必须包含 `Closes #<issue-number>` 来自动关闭关联 issue。**

## 完整示例

```markdown
## Summary

Fix multiple correctness gaps in file discovery, cache, reporter, and formatter.

## Changes

### File Discovery
- Iterate **all** CLI positional patterns (not just the first)
- Walk project root only when no patterns provided
- Deduplicate paths

### Cache
- Fix race condition: collect results in parallel, write cache **sequentially**
- Bump cache version to 2

### Reporter
- Add `--limit`, `--reporter-output`, `--relative` support
- 10 new unit tests

### Rules
- **Operator spacing**: skip unary minus via CST parent detection
- **NoSingleExpressionBody**: skip trailing lambdas
- **FunctionExpressionBody**: better block-body analysis

## Related Issues

Closes #45

## Verification
- 480 unit tests pass
- 13 integration tests pass
- Build: 0 errors
```

## 完整流程

```bash
# 1. 从 main 切分支
git checkout main && git pull
git checkout -b fix/my-fix

# 2. 编码 + 测试
cargo build
cargo test -- --test-threads=4
cargo fmt -- --check

# 3. 提交
git add -A
git commit -m "<type>: <description>"

# 4. 推送 + 创建 PR
git push origin fix/my-fix
gh pr create \
  --base main \
  --head fix/my-fix \
  --title "<type>: <description>" \
  --body "$(cat <<'EOF'
## Summary
...

## Changes
- ...

## Related Issues
Closes #<num>

## Verification
- ...
EOF
)"
```

## 注意事项

- **不要**直接 push 到 main（仓库规则禁止）
- **一定要**在 PR body 里加 `Closes #<num>` 来关联 issue
- **一定要**检查并提交 `Cargo.lock`，版本 bump 或依赖变更后 lockfile 可能不同步
- CI 全部通过后，需要 `--admin` 标志才能合并（项目设置了 required checks）
- 合并用 squash：`gh pr merge <num> --squash --delete-branch --admin`
- 预发版本：合并后创建 tag + release
- **发版后必须**用 `cargo install --git` 从 GitHub 安装，不要本地 `cp` 到 `~/.local/bin`
```bash
cargo install --git https://github.com/qdsfdhvh/ktlint-rs --force
```
- Docs-only PR 可能被 branch protection 卡住——如果 CI jobs 被 `paths-ignore` 跳过但仍是 required status checks，需要从 GitHub Web UI 手动 bypass 或暂时关掉 branch protection
