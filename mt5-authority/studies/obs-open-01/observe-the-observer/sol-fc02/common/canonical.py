"""Pure-stdlib canonical bytes and hashing for FC-02."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Iterable


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_value(value: Any) -> str:
    return sha256_bytes(canonical_bytes(value))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_hash(paths: Iterable[Path], root: Path) -> str:
    entries = []
    for path in sorted(paths):
        if path.is_file() and "__pycache__" not in path.parts:
            entries.append((path.relative_to(root).as_posix(), sha256_file(path)))
    return sha256_value(entries)


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))
