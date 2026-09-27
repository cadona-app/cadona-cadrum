#!/usr/bin/env python3
"""Describe a complete OCCT release using the same version as the build script."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
TARGETS = (
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "wasm32-unknown-unknown",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
)


def build_version(source):
    values = []
    for name in ("OCCT_VERSION", "BUILD_REVISION"):
        matches = re.findall(rf'^const {name}: &str = "([A-Za-z0-9_]+)";', source, re.M)
        if len(matches) != 1:
            raise ValueError(f"expected exactly one {name} in build.rs")
        values.append(matches[0])
    return values


def describe(directory, source_commit, build_source):
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise ValueError("source revision must be a full Git commit")
    version, revision = build_version(build_source)
    tag = f"occt-{version.lstrip('Vv')}_{revision}"
    expected = {f"{tag}-{target.replace('-', '_')}.tar.gz": target for target in TARGETS}
    actual = {path.name for path in directory.glob("*.tar.gz")}
    if actual != expected.keys():
        raise ValueError(
            f"incomplete release: missing={sorted(expected.keys() - actual)}, "
            f"unexpected={sorted(actual - expected.keys())}"
        )
    artifacts = []
    for name, target in sorted(expected.items()):
        path = directory / name
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"empty or missing package: {name}")
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        artifacts.append({
            "name": name,
            "target": target,
            "bytes": path.stat().st_size,
            "sha256": digest,
        })
    return {
        "schema_version": 1,
        "source_commit": source_commit,
        "occt_version": version,
        "build_revision": revision,
        "release_tag": tag,
        "artifacts": artifacts,
    }


def main():
    directory, source_commit = Path(sys.argv[1]), sys.argv[2]
    manifest = describe(directory, source_commit, (ROOT / "build.rs").read_text())
    (directory / "BUILD_INFO.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (directory / "SHA256SUMS").write_text("".join(
        f"{item['sha256']}  {item['name']}\n" for item in manifest["artifacts"]
    ))
    print(manifest["release_tag"])


if __name__ == "__main__":
    main()
