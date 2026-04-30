#!/usr/bin/env python3
"""
Split RFC-183 chapter 3 (Design) into nested README hubs/leaves with zero-padded paths.

Uses git revision 5dce38f~1 for the Design chapter body so moved-out sections (e.g. 3.3)
are still complete. Preserves the current README's ### 3.3 hub block when present.

Run from repo root:
  python3 rfc/rfc-000-000-183-chico-vegetation/tools/split_chapter3.py
"""
from __future__ import annotations

import re
import subprocess
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
RFC_DIR = REPO_ROOT / "rfc" / "rfc-000-000-183-chico-vegetation"
README = RFC_DIR / "README.md"  # RFC_DIR = .../wctp/rfc/rfc-000-000-183-chico-vegetation
GIT_README_PATH = "rfc/rfc-000-000-183-chico-vegetation/README.md"
# Last commit before ground-cover bodies were removed from the main README.
CHAPTER3_SOURCE_REF = "5dce38f~1"

HEADING_RE = re.compile(r"^(#{2,6}) (3(?:\.\d+)*):\s*(.*)\s*$")
ANCHOR_LINK_RE = re.compile(r"\]\(#([\d]+(?:-[a-z0-9]+)+)\)")


def slugify(title: str) -> str:
    s = title.strip().lower()
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode("ascii")
    s = re.sub(r"[^a-z0-9]+", "-", s)
    return s.strip("-") or "section"


def anchor_for(num_str: str, title: str) -> str:
    digits = num_str.replace(".", "")
    return f"{digits}-{slugify(title)}"


def parts_from_num(num_str: str) -> tuple[int, ...]:
    return tuple(int(x) for x in num_str.split("."))


def padded_prefix(parts: tuple[int, ...]) -> str:
    return "-".join(f"{p:02d}" for p in parts)


@dataclass
class Section:
    num_str: str
    parts: tuple[int, ...]
    title: str
    hashes: str
    line_idx: int
    heading_line: str
    anchor: str = ""
    rel_dir: Path = field(default_factory=Path)


def relpath_between(from_dir: Path, to_dir: Path) -> str:
    a = from_dir.parts
    b = to_dir.parts
    i = 0
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    up = len(a) - i
    down = b[i:]
    parts = [".."] * up + list(down)
    return "/".join(parts) if parts else "."


def find_chapter3_slice(file_lines: list[str]) -> tuple[int, int] | None:
    start = end = None
    for i, line in enumerate(file_lines):
        if line.startswith("## 3: Design"):
            start = i
        elif start is not None and line.startswith("## 4:"):
            end = i
            break
    if start is None or end is None:
        return None
    return start, end


def git_show_readme(ref: str) -> list[str]:
    raw = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "show", f"{ref}:{GIT_README_PATH}"],
        text=True,
    )
    return raw.splitlines(keepends=True)


def extract_33_hub_override(cur_lines: list[str]) -> str | None:
    s = e = None
    for i, line in enumerate(cur_lines):
        if line.startswith("### 3.3:"):
            s = i
        elif s is not None and line.startswith("### 3.4:"):
            e = i
            break
    if s is None or e is None:
        return None
    return "".join(cur_lines[s:e])


def extract_33_intro_only(cur_lines: list[str]) -> str | None:
    hub = extract_33_hub_override(cur_lines)
    if not hub:
        return None
    if "Subsections:" in hub:
        return hub.split("Subsections:", 1)[0].rstrip() + "\n\n"
    return hub


def main() -> None:
    cur_lines = README.read_text(encoding="utf-8").splitlines(keepends=True)

    old_lines = git_show_readme(CHAPTER3_SOURCE_REF)
    sl_cur = find_chapter3_slice(cur_lines)
    sl_old = find_chapter3_slice(old_lines)
    if sl_cur is None or sl_old is None:
        raise SystemExit("Could not find ## 3 / ## 4 in current or historical README")

    start, end = sl_cur
    so, eo = sl_old
    lines = cur_lines[: start + 1] + old_lines[so + 1 : eo] + cur_lines[end:]

    design_start_line = start + 1
    end = end  # still index in merged `lines`

    design_lines = lines[design_start_line:end]
    raw: list[Section] = []
    for off, line in enumerate(design_lines):
        m = HEADING_RE.match(line)
        if not m:
            continue
        hashes, num_str, title = m.group(1), m.group(2), m.group(3)
        if num_str == "3" or not title.strip():
            continue
        raw.append(
            Section(
                num_str=num_str,
                parts=parts_from_num(num_str),
                title=title.strip(),
                hashes=hashes,
                line_idx=design_start_line + off,
                heading_line=line.rstrip("\n"),
            )
        )

    raw.sort(key=lambda s: s.line_idx)
    for s in raw:
        s.anchor = anchor_for(s.num_str, s.title)

    def true_parent(s: Section) -> Section | None:
        best: Section | None = None
        best_len = -1
        for t in raw:
            if t.line_idx >= s.line_idx:
                break
            if s.num_str.startswith(t.num_str + "."):
                L = len(t.num_str)
                if L > best_len:
                    best = t
                    best_len = L
        return best

    children_map: dict[int, list[Section]] = {}
    for s in raw:
        p = true_parent(s)
        if p:
            children_map.setdefault(p.line_idx, []).append(s)
    for k in list(children_map):
        children_map[k].sort(key=lambda c: c.line_idx)

    def folder_name(s: Section) -> str:
        return f"{padded_prefix(s.parts)}-{slugify(s.title)}"

    rel_by_line: dict[int, Path] = {}

    def assign_rel(s: Section) -> Path:
        if s.line_idx in rel_by_line:
            return rel_by_line[s.line_idx]
        fn = folder_name(s)
        p = true_parent(s)
        rel = (assign_rel(p) / fn) if p else Path(fn)
        rel_by_line[s.line_idx] = rel
        return rel

    for s in raw:
        s.rel_dir = assign_rel(s)

    anchor_to_rel: dict[str, Path] = {}
    for s in raw:
        anchor_to_rel[s.anchor] = s.rel_dir

    # Legacy in-doc anchors used shortened slugs (e.g. liams vs liam-s).
    link_aliases = {
        "3172-liams-conifer": "3172-liam-s-conifer",
        "31714-friends-conifer": "31714-friend-s-conifer",
        "31716-simplemans-hedge": "31716-simpleman-s-hedge",
        "3433-jims-collage": "3433-jim-s-collage",
    }
    for alias, canonical in link_aliases.items():
        if canonical in anchor_to_rel:
            anchor_to_rel[alias] = anchor_to_rel[canonical]

    def rewrite_links(chunk: str, from_readme_dir: Path) -> str:
        def repl(m: re.Match[str]) -> str:
            aid = m.group(1)
            if aid not in anchor_to_rel:
                return m.group(0)
            target_dir = anchor_to_rel[aid]
            rp = relpath_between(from_readme_dir, target_dir)
            return f"]({rp}/README.md#{aid})"

        return ANCHOR_LINK_RE.sub(repl, chunk)

    def section_body_end_line(s: Section) -> int:
        L = len(s.parts)
        for t in raw:
            if t.line_idx <= s.line_idx:
                continue
            if len(t.parts) <= L:
                return t.line_idx - 1
        return end - 1

    def first_child_line(s: Section) -> int | None:
        ch = children_map.get(s.line_idx, [])
        return ch[0].line_idx if ch else None

    def first_paragraph(text: str) -> str:
        paras = re.split(r"\n\n+", text, maxsplit=1)
        return paras[0] + ("\n\n" if len(paras) > 1 else "")

    def write_section_file(s: Section) -> None:
        ch = children_map.get(s.line_idx, [])
        body_start = s.line_idx + 1
        body_end = section_body_end_line(s)
        out_dir = RFC_DIR / s.rel_dir
        out_dir.mkdir(parents=True, exist_ok=True)
        out_file = out_dir / "README.md"

        h1 = f"# {s.num_str}: {s.title}"
        depth = len(s.rel_dir.parts)
        back = (
            f"This page is subsection **{s.num_str}** of [RFC-183: Chico Vegetation]("
            + ("../" * depth + "README.md")
            + ")\n\n"
        )

        if ch:
            fc = first_child_line(s)
            assert fc is not None
            intro = "".join(lines[body_start:fc])
            intro = rewrite_links(intro, s.rel_dir)
            bullets = ["Subsections:\n", "\n"]
            for c in ch:
                rp = relpath_between(s.rel_dir, c.rel_dir)
                href = f"./{rp}/README.md" if rp != "." else "./README.md"
                bullets.append(f"- [{c.num_str}: {c.title}]({href})\n")
            bullets.append("\n")
            body = intro + "".join(bullets)
        else:
            chunk = "".join(lines[body_start : body_end + 1])
            body = rewrite_links(chunk, s.rel_dir)

        out_file.write_text(h1 + "\n\n" + back + body, encoding="utf-8")

    for s in raw:
        write_section_file(s)

    roots = [s for s in raw if len(s.parts) == 2]
    roots.sort(key=lambda s: s.line_idx)

    out_blocks: list[str] = []
    for r in roots:
        ch = children_map.get(r.line_idx, [])
        body_start = r.line_idx + 1
        if r.num_str == "3.3":
            intro33 = extract_33_intro_only(cur_lines)
            if intro33 is None:
                fc = first_child_line(r) if ch else None
                intro33 = (
                    "".join(lines[body_start:fc])
                    if fc is not None
                    else "".join(lines[body_start : section_body_end_line(r) + 1])
                )
            intro33 = rewrite_links(intro33, Path("."))
            bullets = ["Subsections:\n", "\n"]
            bullets.append(
                "- [3.3.1: Bump Outs](./03-03-ground-cover/03-03-01-bump-outs/README.md)\n"
            )
            bullets.append(
                "- [3.3.2: Tufts](./03-03-ground-cover/03-03-02-tufts/README.md)\n"
            )
            bullets.append("\n---\n\n")
            out_blocks.append(f"### {r.num_str}: {r.title}\n\n{intro33}{''.join(bullets)}")
            continue
        if ch:
            fc = first_child_line(r)
            assert fc is not None
            intro = "".join(lines[body_start:fc])
            intro = rewrite_links(intro, Path("."))
            bullets = ["Subsections:\n", "\n"]
            for c in ch:
                rp = relpath_between(Path("."), c.rel_dir)
                href = f"./{rp}/README.md" if rp != "." else "./README.md"
                bullets.append(f"- [{c.num_str}: {c.title}]({href})\n")
            bullets.append("\n---\n\n")
            out_blocks.append(f"### {r.num_str}: {r.title}\n\n{intro}{''.join(bullets)}")
        else:
            full = "".join(lines[body_start : section_body_end_line(r) + 1])
            intro = first_paragraph(full)
            intro = rewrite_links(intro, Path("."))
            rp = "./" + (r.rel_dir / "README.md").as_posix()
            out_blocks.append(
                f"### {r.num_str}: {r.title}\n\n{intro}"
                "Subsections:\n\n"
                f"- [{r.num_str}: {r.title}]({rp})\n\n---\n\n"
            )

    if out_blocks and out_blocks[-1].endswith("---\n\n"):
        out_blocks[-1] = out_blocks[-1][:-5]

    new_design = "## 3: Design\n\n" + "".join(out_blocks)
    before = "".join(lines[:start])
    after = "".join(lines[end:])
    README.write_text(before + new_design + after, encoding="utf-8")

    # §3.3.1 / §3.3.2 bodies are not in CHAPTER3_SOURCE_REF (that revision only linked out).
    # Restore tracked leaf files when missing so bump/tuft pages stay available after a clean split.
    _restore = [
        "rfc/rfc-000-000-183-chico-vegetation/03-03-ground-cover/03-03-01-bump-outs/README.md",
        "rfc/rfc-000-000-183-chico-vegetation/03-03-ground-cover/03-03-02-tufts/README.md",
    ]
    for rel in _restore:
        p = REPO_ROOT / rel
        if not p.is_file():
            subprocess.run(
                ["git", "-C", str(REPO_ROOT), "restore", "HEAD", "--", rel],
                check=False,
                capture_output=True,
            )

    print(f"Wrote chapter 3 and {len(raw)} section README files under {RFC_DIR}")


if __name__ == "__main__":
    main()
