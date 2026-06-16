#!/usr/bin/env python3
"""Normalize platform imports for Linux cross-compile."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src"

REPLACEMENTS = [
    (
        "#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::{next_control_id, read_window_text, to_wide};",
        "use crate::platform::next_control_id;\n#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::{read_window_text, to_wide};",
    ),
    (
        "#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::{next_control_id, to_wide};",
        "use crate::platform::next_control_id;\n#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::to_wide;",
    ),
    (
        "#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::next_control_id;",
        "use crate::platform::next_control_id;",
    ),
    (
        "#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::{next_menu_id, to_wide};",
        "use crate::platform::{next_menu_id, to_wide};",
    ),
    (
        "#[cfg(target_os = \"windows\")]\nuse crate::platform::win32::next_menu_id;",
        "use crate::platform::next_menu_id;",
    ),
]

NATIVE_HANDLE_FIX = re.compile(
    r"(fn native_handle\(&self\) -> isize \{\s*"
    r"#\[cfg\(target_os = \"windows\"\)\]\s*"
    r"\{\s*"
    r"self\.[\w]+ as isize\s*"
    r"\}\s*"
    r"\})",
    re.MULTILINE,
)


def fix_native_handle(content: str) -> str:
    def repl(match: re.Match[str]) -> str:
        block = match.group(1)
        if "#[cfg(not(target_os = \"windows\"))]" in block:
            return block
        inner = match.group(1)
        return inner[:-1] + "\n        #[cfg(not(target_os = \"windows\"))]\n        {\n            0\n        }\n    }"

    return NATIVE_HANDLE_FIX.sub(repl, content)


def main() -> None:
    changed = 0
    for path in ROOT.rglob("*.rs"):
        text = path.read_text(encoding="utf-8")
        original = text
        for old, new in REPLACEMENTS:
            text = text.replace(old, new)
        text = fix_native_handle(text)
        if text != original:
            path.write_text(text, encoding="utf-8")
            changed += 1
            print(f"updated {path.relative_to(ROOT.parent)}")


if __name__ == "__main__":
    main()
    print("done")
