#!/usr/bin/env python3
"""Rebrands user-visible text in the backend's Rust string literals.

Only the contents of string literals change; identifiers, crate names, wire
formats (headers, user agents, env vars, version strings) and references to
real Windmill services (Windmill Hub, Windmill Cloud, windmill.dev docs,
Windmill Labs attributions) are left alone.

Usage: tools/rebrand.py [--check] <dir>
  --check  print what would change and exit 1 if anything would; write nothing.
"""

import re
import sys
from pathlib import Path

# Applied in order to each string literal's contents.
FEATURE_GATES = [
    (r"this feature requires Windmill Enterprise Edition", "this feature is not supported in Gantry"),
    (r"requires a valid Windmill Enterprise Edition license", "is not supported in Gantry"),
    (r"requires Windmill Enterprise Edition", "is not supported in Gantry"),
    (r"\b(is|are) only available on Windmill Enterprise Edition", r"\1 not supported in Gantry"),
    (r"Only available on Windmill Enterprise Edition", "Not supported in Gantry"),
    (r"\b(is|are) an? (?:Windmill )?Enterprise Edition feature", r"\1 not supported in Gantry"),
    (r"\b(is|are) an? EE feature(?: only)?", r"\1 not supported in Gantry"),
    (r"\bis only available in the EE\b", "is not supported in Gantry"),
    (r"\brequires? EE\b", "is not supported in Gantry"),
    (r"\brequires an enterprise license\b", "is not supported in Gantry"),
    (r"\brequires Enterprise Edition\b", "is not supported in Gantry"),
    (r"[Nn]ot implemented in Windmill's Open Source repository",
     lambda m: m.group(0)[0] + "ot implemented in Gantry"),
    (r"License can't be validated in Windmill CE", "License keys are not used by Gantry"),
    (r"Audit log not not available in Windmill Community edition", "Audit logs are not supported in Gantry"),
    (r"Please contact (?:the )?Windmill [Tt]eam(?: for support)?", "Please report this as a bug"),
    (r"please report this issue to Windmill Team", "please report this as a bug"),
    (r"the Windmill Object Storage", "workspace object storage"),
    (r"Windmill Community Edition", "Gantry"),
]

# "Windmill" stays when it names a real Windmill service, an attribution or a
# wire identifier: followed by one of KEEP_AFTER or preceded by one of KEEP_BEFORE.
KEEP_AFTER = (" Hub", " Cloud", " Labs", " CLI", " documentation", "-Agent", "-Smoke")
KEEP_BEFORE = ("X-", "cloud hosted ", "entire ")

# Files whose literals quote Windmill's own material (its documentation), not
# text Gantry shows as its own.
EXCLUDED = {"windmill-api/src/docs/search/tests.rs"}

CHAR_LITERAL = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])'")
RAW_STRING = re.compile(r'[br]?r(#*)"')


def string_spans(src: str):
    """Yields (start, end) of the contents of each ordinary string literal.

    A small Rust lexer: comments, char literals, raw strings and byte strings
    are skipped, so a quote inside any of them is never taken for a string.
    """
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            i = src.find("\n", i)
            i = n if i < 0 else i
        elif src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif c == "'":
            m = CHAR_LITERAL.match(src, i)
            i = m.end() if m else i + 1  # a lifetime or label
        elif c in "br" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")) and (
            m := RAW_STRING.match(src, i)
        ):
            i = src.index('"' + m.group(1), m.end()) + 1 + len(m.group(1))
        elif c == "b" and src.startswith('b"', i) and (i == 0 or not src[i - 1].isalnum()):
            i = _string_end(src, i + 2)
        elif c == '"':
            end = _string_end(src, i + 1)
            yield i + 1, end - 1
            i = end
        else:
            i += 1


def _string_end(src: str, i: int) -> int:
    """Index just past the closing quote of a string whose contents start at i."""
    while src[i] != '"':
        i += 2 if src[i] == "\\" else 1
    return i + 1


def keep_name(text: str, start: int, end: int) -> bool:
    return text.startswith(KEEP_AFTER, end) or text.endswith(KEEP_BEFORE, 0, start)


def rebrand(text: str) -> str:
    for pattern, replacement in FEATURE_GATES:
        text = re.sub(pattern, replacement, text)
    return re.sub(
        r"\bWindmill\b",
        lambda m: m.group(0) if keep_name(text, m.start(), m.end()) else "Gantry",
        text,
    )


def rewrite_file(path: Path) -> str | None:
    original = path.read_text()
    parts, pos = [], 0
    for start, end in string_spans(original):
        parts += [original[pos:start], rebrand(original[start:end])]
        pos = end
    parts.append(original[pos:])
    updated = "".join(parts)
    return updated if updated != original else None


def main() -> int:
    args = sys.argv[1:]
    check = "--check" in args
    roots = [Path(a) for a in args if a != "--check"]
    if len(roots) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    changed = 0
    for path in sorted(roots[0].rglob("*.rs")):
        if "target" in path.parts or path.relative_to(roots[0]).as_posix() in EXCLUDED:
            continue
        updated = rewrite_file(path)
        if updated is None:
            continue
        changed += 1
        if check:
            print(path)
        else:
            path.write_text(updated)
    print(f"{changed} files {'would change' if check else 'changed'}", file=sys.stderr)
    return 1 if check and changed else 0


if __name__ == "__main__":
    sys.exit(main())
