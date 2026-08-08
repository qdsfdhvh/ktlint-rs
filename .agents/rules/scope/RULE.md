# Scope Rules — consumer projects stay untouched

ktlint-rs is validated against consumer projects (kataris-app, ktor,
nowinandroid, okhttp, compose-samples) as **test corpora only**.

## Hard rules

- ❌ NEVER modify consumer-project files: build config
  (`build.gradle.kts`, `libs.versions.toml`), CI workflows, source code,
  or scripts. Running ktlint-rs against them for validation is fine;
  editing them is not.
- ✅ If a consumer repo needs Gradle tasks / CI wiring (e.g. the #89/#90
  cutover), deliver the guide/script in this repo
  (`docs/APP_INTEGRATION.md`) and let the app team apply it.
- ❌ NEVER copy a consumer project's files into a ktlint-rs commit, and
  never reference consumer paths in code/tests committed here.
- ✅ Clean up probe artifacts (e.g. `__ktlint_probe__.kt`) and revert any
  accidental consumer-repo changes before finishing a task.
