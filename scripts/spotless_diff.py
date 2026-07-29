#!/usr/bin/env python3
"""Structured helpers for the Spotless/ktlint differential harness."""

from __future__ import annotations

import difflib
import json
import pathlib
import sys
import shutil
from typing import Any


def write_json(path: str, value: Any) -> None:
    pathlib.Path(path).write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def normalized_path(value: str) -> str:
    value = value.replace("\\", "/")
    marker = "src/"
    index = value.find(marker)
    return value[index:] if index >= 0 else value.removeprefix("./")


def discover(root: str) -> list[str]:
    base = pathlib.Path(root)
    paths = []
    for path in base.glob("src/**/*.kt"):
        relative = path.relative_to(base).as_posix()
        if "generated" not in path.relative_to(base).parts:
            paths.append(relative)
    return sorted(paths)


def normalize_ktlint(value: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result = []
    for file_entry in value:
        if "errors" not in file_entry:
            result.append(
                {
                    "auto_fixable": file_entry["auto_fixable"],
                    "file": normalized_path(file_entry["file"]),
                    "line": file_entry["line"],
                    "column": file_entry["column"],
                    "rule": file_entry["rule"],
                    "message": file_entry["message"],
                }
            )
            continue
        for error in file_entry.get("errors", []):
            result.append(
                {
                    "auto_fixable": error.get("auto_fixable"),
                    "file": normalized_path(file_entry["file"]),
                    "line": error["line"],
                    "column": error["column"],
                    "rule": error["rule"],
                    "message": error["message"],
                }
            )
    return sorted(
        result,
        key=lambda item: (
            item["file"],
            item["line"],
            item["column"],
            item["rule"],
            item["message"],
        ),
    )


def normalize_rs(value: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result = [
        {
            "auto_fixable": item["auto_fixable"],
            "file": normalized_path(item["file"]),
            "line": item["line"],
            "column": item.get("column", item.get("col")),
            "rule": item["rule"],
            "message": item["message"],
        }
        for item in value
    ]
    return sorted(
        result,
        key=lambda item: (
            item["file"],
            item["line"],
            item["column"],
            item["rule"],
            item["message"],
        ),
    )


def compare_json(expected_path: str, actual_path: str, diff_path: str) -> int:
    expected = json.loads(pathlib.Path(expected_path).read_text(encoding="utf-8"))
    actual = json.loads(pathlib.Path(actual_path).read_text(encoding="utf-8"))
    expected_text = json.dumps(expected, indent=2, sort_keys=True).splitlines(keepends=True)
    actual_text = json.dumps(actual, indent=2, sort_keys=True).splitlines(keepends=True)
    diff = "".join(
        difflib.unified_diff(
            expected_text,
            actual_text,
            fromfile="oracle",
            tofile="ktlint-rs",
        )
    )
    pathlib.Path(diff_path).write_text(diff, encoding="utf-8")
    return 0 if not diff else 1


def check_config(actual_path: str, expected_path: str, output_path: str) -> int:
    actual = json.loads(pathlib.Path(actual_path).read_text(encoding="utf-8"))
    expected = json.loads(pathlib.Path(expected_path).read_text(encoding="utf-8"))["kotlin"]
    differences: list[str] = []
    scalar_keys = {
        "code_style": "ktlint_code_style",
        "indent_size": "indent_size",
        "indent_style": "indent_style",
        "tab_width": "tab_width",
        "max_line_length": "max_line_length",
        "insert_final_newline": "insert_final_newline",
        "trim_trailing_whitespace": "trim_trailing_whitespace",
    }
    for entry in actual:
        for actual_key, expected_key in scalar_keys.items():
            if entry.get(actual_key) != expected.get(expected_key):
                differences.append(
                    f"{entry['file']}: {actual_key}: expected {expected.get(expected_key)!r}, got {entry.get(actual_key)!r}"
                )
        rules = entry.get("rules", {})
        for rule_id in expected.get("disabledRules", []):
            if rules.get(rule_id, {}).get("enabled") is not False:
                differences.append(f"{entry['file']}: expected {rule_id} to be disabled")
        expected_annotation = expected.get("ktlint_function_naming_ignore_when_annotated_with")
        actual_annotation = rules.get(
            "ktlint_function_naming_ignore_when_annotated_with", {}
        ).get("properties", {}).get("annotated_with")
        if actual_annotation != expected_annotation:
            differences.append(
                f"{entry['file']}: function naming annotation: expected {expected_annotation!r}, got {actual_annotation!r}"
            )
        actual_ij = rules.get("ij_kotlin_properties", {}).get("properties", {})
        for key, value in expected.get("ij_kotlin_properties", {}).items():
            if actual_ij.get(key) != str(value).lower():
                differences.append(
                    f"{entry['file']}: {key}: expected {value!r}, got {actual_ij.get(key)!r}"
                )
    write_json(output_path, {"matches": not differences, "differences": differences})
    return 0 if not differences else 1


def mutate_config(path: str) -> None:
    config = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    config["kotlin"]["max_line_length"] += 1
    write_json(path, config)


def inject_diagnostic(path: str) -> None:
    diagnostics = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    diagnostics.append(
        {
            "auto_fixable": False,
            "col": 1,
            "file": "src/main/kotlin/oracle/SpreadOperator.kt",
            "line": 1,
            "message": "Injected differential self-test",
            "rule": "standard:injected-self-test",
        }
    )
    write_json(path, diagnostics)


def minimize_artifacts(
    input_root: str,
    oracle_root: str,
    actual_root: str,
    oracle_diagnostics: str,
    actual_diagnostics: str,
    output_root: str,
) -> None:
    inputs = pathlib.Path(input_root)
    oracle = pathlib.Path(oracle_root)
    actual = pathlib.Path(actual_root)
    output = pathlib.Path(output_root)
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True, exist_ok=True)
    mismatched: set[str] = set()

    oracle_items = json.loads(pathlib.Path(oracle_diagnostics).read_text(encoding="utf-8"))
    actual_items = json.loads(pathlib.Path(actual_diagnostics).read_text(encoding="utf-8"))
    if oracle_items != actual_items:
        mismatched.update(item["file"] for item in oracle_items)
        mismatched.update(item["file"] for item in actual_items)

    relative_files = {
        path.relative_to(oracle).as_posix() for path in oracle.glob("src/**/*.kt")
    } | {path.relative_to(actual).as_posix() for path in actual.glob("src/**/*.kt")}
    for relative in relative_files:
        oracle_path = oracle / relative
        actual_path = actual / relative
        if not oracle_path.exists() or not actual_path.exists():
            mismatched.add(relative)
        elif oracle_path.read_bytes() != actual_path.read_bytes():
            mismatched.add(relative)

    for relative in sorted(mismatched):
        for label, root in (("input", inputs), ("oracle", oracle), ("actual", actual)):
            source = root / relative
            if source.exists():
                destination = output / label / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
    write_json(output / "mismatched-files.json", sorted(mismatched))


def main() -> int:
    if len(sys.argv) < 2:
        raise SystemExit("missing command")
    command, *args = sys.argv[1:]
    if command == "discover" and len(args) == 2:
        write_json(args[1], discover(args[0]))
        return 0
    if command == "normalize-ktlint" and len(args) == 2:
        write_json(args[1], normalize_ktlint(json.loads(pathlib.Path(args[0]).read_text())))
        return 0
    if command == "normalize-rs" and len(args) == 2:
        write_json(args[1], normalize_rs(json.loads(pathlib.Path(args[0]).read_text())))
        return 0
    if command == "compare-json" and len(args) == 3:
        return compare_json(*args)
    if command == "check-config" and len(args) == 3:
        return check_config(*args)
    if command == "minimize-artifacts" and len(args) == 6:
        minimize_artifacts(*args)
        return 0
    if command == "mutate-config" and len(args) == 1:
        mutate_config(args[0])
        return 0
    if command == "inject-diagnostic" and len(args) == 1:
        inject_diagnostic(args[0])
        return 0
    raise SystemExit(f"invalid command or arguments: {command}")


if __name__ == "__main__":
    raise SystemExit(main())
