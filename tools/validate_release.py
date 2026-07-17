#!/usr/bin/env python3
"""Static release validation that does not require the Rust toolchain."""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import tomllib
from html.parser import HTMLParser
from pathlib import Path

try:
    import yaml
except ImportError:  # Standard-library fallback for minimal environments.
    yaml = None

ROOT = Path(__file__).resolve().parents[1]


class StrictishHtmlParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.scripts: list[str] = []
        self._in_script = False
        self._script_parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() == "script" and not dict(attrs).get("src"):
            self._in_script = True
            self._script_parts = []

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() == "script" and self._in_script:
            self.scripts.append("".join(self._script_parts))
            self._in_script = False
            self._script_parts = []

    def handle_data(self, data: str) -> None:
        if self._in_script:
            self._script_parts.append(data)


def run(command: list[str], *, cwd: Path = ROOT) -> None:
    process = subprocess.run(command, cwd=cwd, text=True, capture_output=True)
    if process.returncode:
        sys.stderr.write(process.stdout)
        sys.stderr.write(process.stderr)
        raise RuntimeError(f"command failed: {' '.join(command)}")


def validate_json() -> None:
    for path in ROOT.rglob("*.json"):
        if any(part in {"target", "dist", ".git"} for part in path.parts):
            continue
        json.loads(path.read_text(encoding="utf-8"))


def validate_toml() -> None:
    for path in [ROOT / "Cargo.toml", ROOT / "Trunk.toml"]:
        tomllib.loads(path.read_text(encoding="utf-8"))


def validate_yaml() -> None:
    for path in (ROOT / ".github").rglob("*.yml"):
        text = path.read_text(encoding="utf-8")
        if "\t" in text:
            raise RuntimeError(f"tab indentation in {path}")
        if yaml is not None:
            parsed = yaml.safe_load(text)
            if not isinstance(parsed, dict):
                raise RuntimeError(f"workflow is not a mapping: {path}")
        elif "jobs:" not in text or "name:" not in text:
            raise RuntimeError(f"workflow missing required top-level structure: {path}")


def validate_javascript_and_html() -> None:
    for path in [ROOT / "cloudflare_worker_proxy.js", ROOT / "static/service-worker.js"]:
        run(["node", "--check", str(path)])

    for path in ROOT.glob("*.html"):
        parser = StrictishHtmlParser()
        parser.feed(path.read_text(encoding="utf-8"))
        parser.close()
        if parser._in_script:
            raise RuntimeError(f"unclosed inline script in {path.name}")
        for index, script in enumerate(parser.scripts):
            if not script.strip():
                continue
            with tempfile.NamedTemporaryFile(
                "w", suffix=f"-{path.stem}-{index}.js", delete=False, encoding="utf-8"
            ) as temporary:
                temporary.write(script)
                temporary_path = Path(temporary.name)
            try:
                run(["node", "--check", str(temporary_path)])
            finally:
                temporary_path.unlink(missing_ok=True)


def validate_shell() -> None:
    for path in ROOT.glob("*.sh"):
        run(["bash", "-n", str(path)])


def rust_code_characters(source: str):
    """Yield code characters while skipping Rust strings and comments."""
    index = 0
    length = len(source)
    while index < length:
        char = source[index]
        nxt = source[index + 1] if index + 1 < length else ""

        if char == "/" and nxt == "/":
            index += 2
            while index < length and source[index] != "\n":
                index += 1
            continue
        if char == "/" and nxt == "*":
            depth = 1
            index += 2
            while index < length and depth:
                pair = source[index:index + 2]
                if pair == "/*":
                    depth += 1
                    index += 2
                elif pair == "*/":
                    depth -= 1
                    index += 2
                else:
                    index += 1
            continue

        # Raw strings: r"...", r#"..."#, br#"..."#, etc.
        raw_start = None
        if char == "r":
            raw_start = index
        elif char == "b" and nxt == "r":
            raw_start = index + 1
        if raw_start is not None:
            cursor = raw_start + 1
            hashes = 0
            while cursor < length and source[cursor] == "#":
                hashes += 1
                cursor += 1
            if cursor < length and source[cursor] == '"':
                closing = '"' + ("#" * hashes)
                cursor += 1
                close_at = source.find(closing, cursor)
                index = length if close_at < 0 else close_at + len(closing)
                continue

        if char == '"' or (char == "b" and nxt == '"'):
            index += 2 if char == "b" else 1
            while index < length:
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            continue

        if char == "'":
            # Skip a character literal, but not a lifetime such as 'a.
            cursor = index + 1
            if cursor < length and source[cursor] == "\\":
                cursor += 2
            else:
                cursor += 1
            if cursor < length and source[cursor] == "'":
                index = cursor + 1
                continue

        yield index, char
        index += 1


def validate_rust_balance() -> None:
    pairs = {"(": ")", "[": "]", "{": "}"}
    closers = {value: key for key, value in pairs.items()}
    for path in (ROOT / "src").rglob("*.rs"):
        stack: list[tuple[str, int]] = []
        source = path.read_text(encoding="utf-8")
        for offset, character in rust_code_characters(source):
            if character in pairs:
                stack.append((character, offset))
            elif character in closers:
                if not stack or stack[-1][0] != closers[character]:
                    raise RuntimeError(f"unbalanced {character} in {path}:{offset}")
                stack.pop()
        if stack:
            raise RuntimeError(f"unclosed delimiter in {path}:{stack[-1][1]}")

def validate_release_boundary() -> None:
    forbidden_files = {
        "copernicus.rs", "darpa.rs", "devcom.rs", "keys.rs", "nasa.rs",
        "open_meteo.rs", "sats.rs", "sentinel.rs", "stormglass.rs",
        "usgs.rs", "vessel_detection.rs", "weather_gov.rs",
    }
    present = forbidden_files.intersection(path.name for path in (ROOT / "src/api").glob("*.rs"))
    if present:
        raise RuntimeError(f"legacy API modules still present: {sorted(present)}")

    secret_patterns = {
        "GitHub token": re.compile(r"gh[pousr]_[A-Za-z0-9]{20,}"),
        "OpenAI-style key": re.compile(r"sk-[A-Za-z0-9]{20,}"),
        "private key": re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
        "query bearer token": re.compile(r"[?&](?:token|access_token|apiKey)=", re.I),
    }
    candidates = [
        *ROOT.rglob("*.rs"), *ROOT.rglob("*.js"), *ROOT.rglob("*.html"),
        *ROOT.rglob("*.md"), *ROOT.rglob("*.yml"), ROOT / ".env.example",
    ]
    for path in candidates:
        if any(part in {"target", "dist", ".git"} for part in path.parts):
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        for label, pattern in secret_patterns.items():
            if pattern.search(text):
                raise RuntimeError(f"possible {label} in {path.relative_to(ROOT)}")

    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    if '"webgl"' in cargo or '"webgl2"' not in cargo:
        raise RuntimeError("Bevy 0.15 WebGL feature is not configured as webgl2")
    rust_sources = "\n".join(path.read_text(encoding="utf-8") for path in (ROOT / "src").rglob("*.rs"))
    if "delta_seconds()" in rust_sources or "elapsed_seconds()" in rust_sources:
        raise RuntimeError("pre-Bevy-0.15 Time API remains")


def main() -> None:
    checks = [
        ("JSON", validate_json),
        ("TOML", validate_toml),
        ("YAML", validate_yaml),
        ("JavaScript/HTML", validate_javascript_and_html),
        ("shell", validate_shell),
        ("Rust delimiter balance", validate_rust_balance),
        ("release boundary", validate_release_boundary),
    ]
    for label, check in checks:
        check()
        print(f"PASS {label}")
    run([sys.executable, str(ROOT / "tools/math_reference_check.py")])
    print("PASS independent numerical reference checks")


if __name__ == "__main__":
    main()
