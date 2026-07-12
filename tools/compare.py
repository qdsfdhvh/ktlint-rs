import subprocess, sys, os

# Run both tools and compare
project = sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/demo-gradle"

# JVM ktlint
result_jvm = subprocess.run(
    ["ktlint", project],
    capture_output=True, text=True, timeout=60
)
jvm_lines = {}
for line in result_jvm.stdout.split('\n') + result_jvm.stderr.split('\n'):
    if '.kt:' in line and '(standard:' in line:
        # Extract file:line:col and rule
        parts = line.split('(standard:')
        if len(parts) == 2:
            loc = parts[0].strip().rsplit('.kt:', 1)
            if len(loc) == 2:
                file_line = loc[0] + '.kt:' + loc[1]
                rule = 'standard:' + parts[1].rstrip(')')
                jvm_lines[file_line] = rule

# ktlint-rs  
os.environ['KTLINT_COMPAT'] = '1'
result_rs = subprocess.run(
    ["target/release/ktlint-rs", project],
    capture_output=True, text=True, timeout=60
)
rs_lines = {}
for line in result_rs.stdout.split('\n'):
    if '.kt:' in line and '(standard:' in line:
        parts = line.split('(standard:')
        if len(parts) == 2:
            loc = parts[0].strip().rsplit('.kt:', 1)
            if len(loc) == 2:
                file_line = loc[0] + '.kt:' + loc[1]
                rule = 'standard:' + parts[1].rstrip(')')
                rs_lines[file_line] = rule

# Compare
common = set(jvm_lines.keys()) & set(rs_lines.keys())
only_jvm = set(jvm_lines.keys()) - set(rs_lines.keys())
only_rs = set(rs_lines.keys()) - set(jvm_lines.keys())

print(f"JVM violations: {len(jvm_lines)}")
print(f"ktlint-rs violations: {len(rs_lines)}")
print(f"Common: {len(common)}")
print(f"Only JVM: {len(only_jvm)}")
print(f"Only ktlint-rs: {len(only_rs)}")

if only_jvm:
    print("\nJVM finds, ktlint-rs misses:")
    for l in sorted(only_jvm)[:10]:
        print(f"  {l} -> {jvm_lines[l]}")
if only_rs:
    print(f"\nktlint-rs finds, JVM misses ({len(only_rs)} total):")
    rs_rules = {}
    for l in only_rs:
        rs_rules[rs_lines[l]] = rs_rules.get(rs_lines[l], 0) + 1
    for r, c in sorted(rs_rules.items(), key=lambda x:-x[1]):
        print(f"  {r}: {c}")
