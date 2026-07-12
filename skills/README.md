# ktlint-rs Agent Skills

Agent skills that teach AI coding tools how to use ktlint-rs effectively.

## Install

```bash
npx skills add https://github.com/qdsfdhvh/ktlint-rs
```

Re-run the same command after updating ktlint-rs so the agent picks up the
latest CLI guidance. If your agent caches skills at startup, restart it
after installing or updating.

## Included skills

| Skill | Description |
|---|---|
| [ktlint-rs](./ktlint-rs/SKILL.md) | Kotlin linting & formatting — 100+ rules, auto-fix, .editorconfig support |

## Structure

Each skill lives in `<name>/SKILL.md` with YAML frontmatter followed by
a Markdown reference that the agent reads to understand when and how to
call the tool.
