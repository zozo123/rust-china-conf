#!/usr/bin/env python3
"""Build both static landing editions from one template and recorded evidence.

No dependencies. Run with --check in CI to detect stale generated pages.
"""
import argparse
import html
import json
from pathlib import Path
from string import Template

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent


def build(lang, copy):
    strings = copy[lang]
    values = {key: html.escape(value, quote=True) for key, value in strings.items()}
    values.update(lang="en" if lang == "en" else "zh-CN", base="../" if lang == "en" else "./",
                  edition=lang, suffix=".en" if lang == "en" else "",
                  page="en/index.html" if lang == "en" else "index.html",
                  canonical="en/" if lang == "en" else "",
                  zh_current='aria-current="page"' if lang == "zh" else "",
                  en_current='aria-current="page"' if lang == "en" else "")
    recorded = json.loads((ROOT / "docs/examples/ec2-runner-b/scenario-results.json").read_text())
    by_name = {row["scenario"]: row for row in recorded}
    rows = []
    for place in ("center", "left", "right", "near", "far"):
        cells = []
        for age in (0, 50, 600):
            result = by_name[f"{place}-f{age}"]
            lifted = result["outcome"] == "cube_lifted"
            label = strings["lifted" if lifted else "refused"]
            cells.append(f'<td class="{"good" if lifted else "warn"}">{html.escape(label)}<small>{result["ticks"]} {html.escape(strings["ticks"])}</small></td>')
        rows.append(f'<tr><th scope="row">{html.escape(strings[place])}</th>{"".join(cells)}</tr>')
    values["matrix_rows"] = "\n".join(rows)
    values["episode_count"] = str(len(recorded))
    return Template((SOURCE / "landing.html").read_text()).substitute(values)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    copy = json.loads((SOURCE / "copy.json").read_text())
    if set(copy["zh"]) != set(copy["en"]):
        raise SystemExit("Translation keys must match across editions")
    stale = []
    for lang, target in (("zh", "docs/index.html"), ("en", "docs/en/index.html")):
        path = ROOT / target
        rendered = build(lang, copy)
        if args.check:
            if not path.exists() or path.read_text() != rendered:
                stale.append(target)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(rendered)
            print(f"wrote {target}")
    if stale:
        raise SystemExit("Run python3 scripts/site/build.py to update: " + ", ".join(stale))
    if args.check:
        print("Both static editions match the template, translations, and recorded matrix")


if __name__ == "__main__":
    main()
