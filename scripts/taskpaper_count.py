#!/usr/bin/env python3
"""Count tasks in a TaskPaper file.

Usage:
    taskpaper_count.py FILE        summary of every top-level project + total
    taskpaper_count.py FILE ROW    summary of the project at/enclosing ROW
                                   (1-based; falls back to the whole file)

Counts are recursive over the whole subtree. A task is "open" unless it
carries a trailing @done/@cancelled tag or lives under an ancestor that
does (the same inheritance rule the Zed extension uses for fading).
Reads the file as saved on disk; stdlib only.
"""

import re
import sys

TRAILING_TAG = re.compile(r"[ \t]+@([^\s()]+)(\([^)\n]*\))?$")
LONE_TAG = re.compile(r"@([^\s()]+)(\([^)\n]*\))?$")


def split_trailing_tags(content):
    """Return (body, set of trailing tag names)."""
    names = set()
    s = content.rstrip()
    while True:
        m = TRAILING_TAG.search(s)
        if m:
            names.add(m.group(1))
            s = s[: m.start()]
            continue
        m = LONE_TAG.fullmatch(s)  # line consisting only of tags
        if m:
            names.add(m.group(1))
            s = ""
        break
    return s, names


def parse(path):
    items = []
    with open(path, encoding="utf-8") as f:
        text = f.read()
    stack = []  # (indent, closed?) for @done/@cancelled inheritance
    for row, raw in enumerate(text.split("\n")):
        if not raw.strip():
            continue
        indent = len(raw) - len(raw.lstrip(" \t"))
        content = raw[indent:]
        body, tags = split_trailing_tags(content)
        is_task = content == "-" or content[:2] in ("- ", "-\t")
        if is_task:
            kind, name = "task", body[2:].strip() or "(untitled)"
        elif body.endswith(":"):
            kind, name = "project", body[:-1].strip()
        else:
            kind, name = "note", body
        while stack and stack[-1][0] >= indent:
            stack.pop()
        closed = bool({"done", "cancelled"} & tags) or (stack and stack[-1][1])
        cancelled = "cancelled" in tags or (stack and stack[-1][1] == "cancelled")
        state = ("cancelled" if cancelled else "done") if closed else "open"
        stack.append((indent, state if closed else False))
        items.append({"row": row, "indent": indent, "kind": kind, "name": name, "state": state})
    return items


def subtree(items, i):
    yield items[i]
    for it in items[i + 1 :]:
        if it["indent"] <= items[i]["indent"]:
            break
        yield it


def summarize(scope):
    tasks = [it for it in scope if it["kind"] == "task"]
    open_ = sum(1 for t in tasks if t["state"] == "open")
    done = sum(1 for t in tasks if t["state"] == "done")
    cancelled = sum(1 for t in tasks if t["state"] == "cancelled")
    return open_, done, cancelled, len(tasks)


def report(label, scope):
    open_, done, cancelled, total = summarize(scope)
    parts = [f"{open_} open"]
    if done:
        parts.append(f"{done} done")
    if cancelled:
        parts.append(f"{cancelled} cancelled")
    print(f"{label}: {', '.join(parts)} ({total} task{'s' * (total != 1)})")


def enclosing_project(items, row):
    """Index of the project at `row`, or the nearest project enclosing it."""
    at = None
    for i, it in enumerate(items):
        if it["row"] <= row:
            at = i
        else:
            break
    while at is not None:
        if items[at]["kind"] == "project" and items[at]["row"] <= row:
            here = items[at]
            if here["row"] == row or any(it["row"] >= row for it in subtree(items, at)):
                return at
        # climb: nearest earlier item with smaller indent
        ind = items[at]["indent"]
        at = next((j for j in range(at - 1, -1, -1) if items[j]["indent"] < ind), None)
    return None


def main():
    path = sys.argv[1]
    row = int(sys.argv[2]) - 1 if len(sys.argv) > 2 else None
    items = parse(path)
    if row is not None:
        i = enclosing_project(items, row)
        if i is not None:
            report(items[i]["name"], list(subtree(items, i)))
            return
        print("(no enclosing project; counting whole file)")
    for i, it in enumerate(items):
        if it["kind"] == "project" and it["indent"] == 0:
            report(it["name"], list(subtree(items, i)))
    report("TOTAL", items)


if __name__ == "__main__":
    main()
