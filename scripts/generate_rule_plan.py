#!/usr/bin/env python3
"""Generate and verify the ktlint 1.8.0 parity manifest and rule plan."""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import subprocess
import sys
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
ORACLE_DIR = ROOT / "tests/oracle/spotless-8.8.0-ktlint-1.8.0"
ORACLE_INVENTORY = ORACLE_DIR / "expected/rule-inventory.json"
EFFECTIVE_CONFIG = ORACLE_DIR / "effective-config.json"
MANIFEST = ORACLE_DIR / "parity-manifest.json"
RULE_PLAN = ROOT / "docs/RULE_PLAN.md"
README = ROOT / "README.md"

ALIASES = {
    "standard:double-colon-spacing": ["standard:spacing-around-double-colon"],
    "standard:no-empty-first-line-in-method-block": [
        "standard:no-leading-empty-lines-in-method"
    ],
    "standard:no-semi": ["standard:no-semicolons"],
    "standard:range-spacing": ["standard:spacing-around-range-operator"],
    "standard:spacing-between-function-name-and-opening-parenthesis": [
        "standard:spacing-between-function-name-and-parenthesis"
    ],
}

FORMATTER_PASSES = {
    "standard:colon-spacing": ["fix_colons"],
    "standard:comma-spacing": ["fix_commas"],
    "standard:comment-spacing": ["fix_comment_spacing"],
    "standard:curly-spacing": ["fix_curly_braces"],
    "standard:double-colon-spacing": ["fix_all_spacing"],
    "standard:final-newline": ["auto_fix final-newline normalization"],
    "standard:indent": ["fix_indentation"],
    "standard:no-blank-line-before-rbrace": ["fix_blank_lines"],
    "standard:no-blank-line-in-list": ["fix_blank_line_in_list"],
    "standard:no-consecutive-blank-lines": ["fix_blank_lines"],
    "standard:no-multi-spaces": ["fix_double_spaces"],
    "standard:no-semi": ["fix_semicolons"],
    "standard:no-trailing-spaces": ["fix_trailing_ws_protected"],
    "standard:op-spacing": ["fix_spread_operators", "fix_operators"],
    "standard:parameter-list-spacing": ["fix_parens", "fix_commas"],
    "standard:paren-spacing": ["fix_parens"],
    "standard:range-spacing": ["fix_range_spacing"],
    "standard:spacing-around-angle-brackets": ["fix_angle_brackets"],
    "standard:trailing-comma-on-call-site": ["fix_single_line_trailing_comma"],
    "standard:trailing-comma-on-declaration-site": ["fix_single_line_trailing_comma"],
    "standard:wrapping": ["fix_all_wrapping"],
}

FIXTURES = {
    "standard:op-spacing": [
        "tests/oracle/spotless-8.8.0-ktlint-1.8.0/src/main/kotlin/oracle/SpreadOperator.kt",
        "src/rules/spacing/operator.rs::spread_operator_*",
        "src/formatter/mod.rs::cs68_spread_operator_*",
    ],
}

KNOWN_MISMATCH_HITS = {
    "standard:colon-spacing": 1,
    "standard:function-signature": 2,
    "standard:function-start-of-body-spacing": 2,
    "standard:op-spacing": 2,
    "standard:parameter-list-spacing": 2,
    "standard:paren-spacing": 2,
}


def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def rust_inventory(binary: pathlib.Path) -> list[dict[str, Any]]:
    output = subprocess.run(
        [str(binary), "--ruleset", "ktlint", "--print-rule-inventory"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(output.stdout)


def priority(rule_id: str, disabled: bool, hit_count: int) -> str:
    if disabled:
        return "P2"
    if hit_count > 0 or any(
        token in rule_id
        for token in (
            "indent",
            "wrapping",
            "spacing",
            "comment",
            "string-template",
            "trailing-comma",
        )
    ):
        return "P0"
    return "P1"


def validate_manifest(manifest: dict[str, Any], oracle_ids: list[str]) -> None:
    rules = manifest["rules"]
    if len(rules) != len(oracle_ids) or [rule["id"] for rule in rules] != oracle_ids:
        raise RuntimeError("manifest must cover every oracle rule exactly once and in oracle order")
    allowed = set(manifest["policy"]["allowedStatuses"])
    owners: list[str] = []
    for rule in rules:
        if rule["status"] not in allowed:
            raise RuntimeError(f"unknown status for {rule['id']}: {rule['status']}")
        if rule["status"] == "missing" and rule["ownerRustRuleIds"]:
            raise RuntimeError(f"missing rule unexpectedly has an owner: {rule['id']}")
        if rule["status"] == "parity-verified" and (
            not rule["ownerRustRuleIds"] or not rule["fixtures"]
        ):
            raise RuntimeError(
                f"parity-verified rule requires implementation and differential fixtures: {rule['id']}"
            )
        if "formatter" not in rule or "status" not in rule["formatter"]:
            raise RuntimeError(f"formatter coverage missing for {rule['id']}")
        owners.extend(rule["ownerRustRuleIds"])
    duplicate_owners = sorted(
        owner for owner, count in collections.Counter(owners).items() if count > 1
    )
    if duplicate_owners:
        raise RuntimeError(f"Rust rule owners map to multiple oracle rules: {duplicate_owners}")


def generate(binary: pathlib.Path) -> tuple[dict[str, Any], str]:
    oracle = load_json(ORACLE_INVENTORY)["ruleSetProviders"]
    if len(oracle) != 1 or oracle[0]["id"] != "standard":
        raise RuntimeError("expected exactly the pinned standard rule set")
    oracle_ids = oracle[0]["rules"]
    if len(oracle_ids) != 101 or len(set(oracle_ids)) != len(oracle_ids):
        raise RuntimeError("pinned oracle inventory must contain 101 unique rules")

    actual_all = rust_inventory(binary)
    actual = [entry for entry in actual_all if entry["enabled_by_ruleset"]]
    all_counts = collections.Counter(entry["id"] for entry in actual_all)
    all_duplicates = sorted(rule_id for rule_id, count in all_counts.items() if count > 1)
    if all_duplicates:
        raise RuntimeError(f"duplicate ktlint-rs registry ids: {all_duplicates}")
    actual_by_id = {entry["id"]: entry for entry in actual}

    effective = load_json(EFFECTIVE_CONFIG)["kotlin"]
    disabled_rules = set(effective["disabledRules"])
    matched_actual: set[str] = set()
    rules = []
    for rule_id in oracle_ids:
        rust_ids = [rule_id] if rule_id in actual_by_id else []
        match = "exact" if rust_ids else "missing"
        if not rust_ids:
            rust_ids = [alias for alias in ALIASES.get(rule_id, []) if alias in actual_by_id]
            if rust_ids:
                match = "alias"
        matched_actual.update(rust_ids)
        disabled = rule_id in disabled_rules
        if disabled:
            status = "disabled-by-kataris"
        elif not rust_ids:
            status = "missing"
        else:
            # Registration is evidence of code presence only, never parity proof.
            status = "partial"
        auto_fixable = any(actual_by_id[item]["auto_fixable"] for item in rust_ids)
        passes = FORMATTER_PASSES.get(rule_id, [])
        if disabled:
            formatter_status = "not-evaluated"
        elif not rust_ids:
            formatter_status = "missing"
        elif not auto_fixable:
            formatter_status = "not-fixable"
        else:
            formatter_status = "unverified"
        hit_count = KNOWN_MISMATCH_HITS.get(rule_id, 0)
        rules.append(
            {
                "id": rule_id,
                "katarisEnabled": not disabled,
                "status": status,
                "registrationMatch": match,
                "ownerRustRuleIds": rust_ids,
                "registeredAutoFixable": auto_fixable,
                "requiresTypeResolution": any(
                    actual_by_id[item]["requires_type_resolution"] for item in rust_ids
                ),
                "formatter": {"status": formatter_status, "passes": passes},
                "fixtures": FIXTURES.get(rule_id, []),
                "knownDirtyMismatchHits": hit_count,
                "priority": priority(rule_id, disabled, hit_count),
            }
        )

    status_counts = collections.Counter(rule["status"] for rule in rules)
    extra = sorted(set(actual_by_id) - matched_actual)
    manifest = {
        "schemaVersion": 1,
        "oracle": {
            "spotlessVersion": "8.8.0",
            "ktlintVersion": "1.8.0",
            "ruleSet": "standard",
            "ruleCount": len(oracle_ids),
        },
        "policy": {
            "registrationAloneNeverMeansParity": True,
            "parityVerifiedRequiresDifferentialFixtures": True,
            "allowedStatuses": [
                "missing",
                "partial",
                "check-only",
                "fixable",
                "parity-verified",
                "disabled-by-kataris",
            ],
        },
        "summary": {
            "statuses": dict(sorted(status_counts.items())),
            "registeredKtlintRuleIds": len(actual_by_id),
            "matchedOracleRules": sum(bool(rule["ownerRustRuleIds"]) for rule in rules),
            "extraRustRuleIds": len(extra),
            "registeredUniqueAll": len(actual_all),
            "registeredStandardOriented": sum(
                entry["id"].startswith("standard:") for entry in actual_all
            ),
            "registeredDetekt": sum(
                entry["id"].startswith("detekt:") for entry in actual_all
            ),
        },
        "rules": rules,
        "extraRustRuleIds": extra,
    }
    validate_manifest(manifest, oracle_ids)
    return manifest, render_markdown(manifest)


def render_markdown(manifest: dict[str, Any]) -> str:
    summary = manifest["summary"]
    statuses = summary["statuses"]
    lines = [
        "# ktlint 1.8.0 parity plan",
        "",
        "> Generated by `scripts/generate_rule_plan.py`; do not edit manually.",
        "> Target: Kataris Spotless 8.8.0 + ktlint 1.8.0 standard rules.",
        "",
        "## Summary",
        "",
        f"- Oracle rules: **{manifest['oracle']['ruleCount']}**",
        f"- Matched to ktlint-rs registrations: **{summary['matchedOracleRules']}**",
        f"- Missing: **{statuses.get('missing', 0)}**",
        f"- Partial/unverified: **{statuses.get('partial', 0)}**",
        f"- Disabled by Kataris: **{statuses.get('disabled-by-kataris', 0)}**",
        f"- Extra ktlint-rs standard IDs: **{summary['extraRustRuleIds']}**",
        "",
        "Registration is not parity evidence. Only byte/diagnostic differential fixtures may move a rule to `parity-verified`.",
        "",
        "## Rules",
        "",
        "| Priority | Oracle rule | Kataris | Status | Rust owner(s) | Formatter | Fixtures | Known dirty hits |",
        "|---|---|---:|---|---|---|---:|---:|",
    ]
    ordered = sorted(
        manifest["rules"],
        key=lambda rule: (
            {"P0": 0, "P1": 1, "P2": 2}[rule["priority"]],
            -rule["knownDirtyMismatchHits"],
            rule["id"],
        ),
    )
    for rule in ordered:
        owners = "<br>".join(f"`{item}`" for item in rule["ownerRustRuleIds"]) or "—"
        formatter = rule["formatter"]["status"]
        if rule["formatter"]["passes"]:
            formatter += ": " + ", ".join(rule["formatter"]["passes"])
        lines.append(
            "| {priority} | `{rule_id}` | {enabled} | {status} | {owners} | {formatter} | {fixtures} | {hits} |".format(
                priority=rule["priority"],
                rule_id=rule["id"],
                enabled="yes" if rule["katarisEnabled"] else "no",
                status=rule["status"],
                owners=owners,
                formatter=formatter,
                fixtures=len(rule["fixtures"]),
                hits=rule["knownDirtyMismatchHits"],
            )
        )
    lines.extend(
        [
            "",
            "## Extra ktlint-rs standard IDs",
            "",
            "These IDs do not directly match a ktlint 1.8.0 oracle rule after configured aliases:",
            "",
            *[f"- `{rule_id}`" for rule_id in manifest["extraRustRuleIds"]],
            "",
            "## Validation",
            "",
            "```sh",
            "cargo build --release",
            "python3 scripts/generate_rule_plan.py --binary target/release/ktlint-rs --check",
            "```",
            "",
            "Detekt inventory is intentionally out of scope for the Kataris Spotless replacement.",
            "",
        ]
    )
    return "\n".join(lines)


def readme_counts_match(manifest: dict[str, Any]) -> bool:
    summary = manifest["summary"]
    text = README.read_text(encoding="utf-8")
    expected = (
        f"badge/rules/{summary['registeredUniqueAll']}/blue",
        f"**{summary['registeredUniqueAll']} unique registry rules**",
        f"| Enabled ktlint-compatible registry IDs | {summary['registeredKtlintRuleIds']} |",
        f"| Oracle rules matched to Rust owners | {summary['matchedOracleRules']} |",
        f"| Oracle rules currently missing | {summary['statuses'].get('missing', 0)} |",
        f"| Detekt registry IDs | {summary['registeredDetekt']} |",
    )
    return all(snippet in text for snippet in expected)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=pathlib.Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    manifest, markdown = generate(args.binary.resolve())
    manifest_text = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    if args.check:
        failures = []
        if not MANIFEST.exists() or MANIFEST.read_text(encoding="utf-8") != manifest_text:
            failures.append(str(MANIFEST.relative_to(ROOT)))
        if not RULE_PLAN.exists() or RULE_PLAN.read_text(encoding="utf-8") != markdown:
            failures.append(str(RULE_PLAN.relative_to(ROOT)))
        if not readme_counts_match(manifest):
            failures.append(str(README.relative_to(ROOT)))
        if failures:
            print("generated parity artifacts are stale: " + ", ".join(failures), file=sys.stderr)
            return 1
        return 0
    MANIFEST.write_text(manifest_text, encoding="utf-8")
    RULE_PLAN.write_text(markdown, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
