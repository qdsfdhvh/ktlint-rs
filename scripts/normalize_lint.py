import sys, re
seen = set()
for line in sys.stdin:
    line = line.strip()
    if not line or 'Summary' in line:
        continue
    m = re.match(r'^(.*?\.kt):(\d+):(\d+)', line)
    if not m:
        continue
    path, ln, col = m.group(1), m.group(2), m.group(3)
    path = path.lstrip('./')
    seen.add(f"{path}:{ln}:{col}")
for s in sorted(seen):
    print(s)
