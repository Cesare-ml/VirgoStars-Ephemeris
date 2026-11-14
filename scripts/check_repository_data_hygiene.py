# EXHIBIT A. Common Public Attribution License Version 1.0.
# The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
#
# Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
#
# The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
#
# The Original Developer is the Initial Developer.
#
# The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
#
# Contributors retain applicable rights in their own contributions under the terms of this License.
#
# [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]
# SPDX-License-Identifier: CPAL-1.0

"""Fail CI if tracked files accidentally bundle external ephemeris/kernel data.

This is a repository distribution guard, NOT a scientific data validator.
"""

from __future__ import annotations

import argparse
from pathlib import Path, PurePosixPath
import subprocess
import sys

KERNEL_EXTENSIONS = frozenset({
    ".bsp", ".eph", ".se1", ".bpc", ".tpc", ".tls", ".tf", ".tsc",
})
# Modification (2026-11-14): recognize layered compression and tarball aliases
# when screening names of tracked external-kernel files.
ARCHIVE_EXTENSIONS = (".gz", ".xz", ".bz2", ".zst", ".zip", ".tar", ".tgz", ".tbz2", ".txz", ".tzst")
SPK_MAGIC = b"DAF/SPK "
CLASSIC_MAGIC = b"JPL Planetary Ephemeris DE"


def is_kernel_path(relative_path: str) -> bool:
    """Reject kernel names, including case variants and common compressed forms."""
    name = PurePosixPath(relative_path).name.lower()
    # Strip *all* known archive layers: e.g. .bsp.tar.gz, .eph.zip.xz,
    # or .tls.tgz. The previous single-pass check missed these names.
    while True:
        for extension in ARCHIVE_EXTENSIONS:
            if name.endswith(extension):
                name = name[: -len(extension)]
                break
        else:
            break
    return any(name.endswith(ext) for ext in KERNEL_EXTENSIONS)


def _indexed_magic(root: Path, oid: str) -> bytes:
    """Read only the first 64 bytes of a staged Git blob, not the worktree file.

    Terminate Git after the prefix so even a disguised multi-GB kernel is not
    copied into Python memory. A genuinely short blob must exit successfully.
    """
    with subprocess.Popen(
        ["git", "cat-file", "blob", oid],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ) as process:
        assert process.stdout is not None and process.stderr is not None
        prefix = process.stdout.read(64)
        if len(prefix) == 64:
            # Deliberate early stop: Git may still be streaming a large blob.
            process.stdout.close()
            process.stderr.close()
            if process.poll() is None:
                try:
                    process.terminate()
                except ProcessLookupError:
                    pass
            return prefix
        error = process.stderr.read()
        status = process.wait()
        if status != 0:
            raise OSError(
                f"cannot inspect indexed blob {oid}: "
                f"{error.decode('utf-8', errors='replace').strip()}"
            )
        return prefix


def inspect_path(
    root: Path,
    relative_path: str,
    *,
    indexed_blob: str | None = None,
    indexed_mode: str | None = None,
) -> str | None:
    """Check a path; repository audits use the *staged* blob and Git mode.

    Without an index object this helper can inspect a working-tree fixture in
    isolated unit tests. Production check_repository always supplies the oid.
    """
    if is_kernel_path(relative_path):
        return "external kernel/ephemeris filename"
    if indexed_blob is not None:
        # Do not dereference indexed symlinks or submodule gitlinks.
        if indexed_mode not in ("100644", "100755"):
            return None
        magic = _indexed_magic(root, indexed_blob)
    else:
        path = root / relative_path
        if path.is_symlink() or not path.is_file():
            return None
        try:
            with path.open("rb") as source:
                magic = source.read(64)
        except OSError as exc:
            return f"cannot inspect tracked file: {exc}"
    if magic.startswith(SPK_MAGIC):
        return "NAIF DAF/SPK magic header"
    if magic.startswith(CLASSIC_MAGIC):
        return "classic JPL planetary ephemeris magic header"
    return None


def tracked_entries(root: Path) -> list[tuple[str, str, str]]:
    """Return (path, mode, blob oid) for stage-zero index entries.

    A conflicted index has no single shippable blob and must fail closed.
    """
    result = subprocess.run(
        ["git", "ls-files", "--stage", "-z"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    entries = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        metadata, separator, path_bytes = raw.partition(b"\t")
        parts = metadata.split(b" ")
        if not separator or len(parts) != 3:
            raise ValueError("malformed Git index entry")
        mode, oid, stage = parts
        if stage != b"0":
            raise ValueError("unmerged Git index: resolve conflicts before distribution checks")
        if mode not in (b"100644", b"100755", b"120000", b"160000"):
            raise ValueError(f"unsupported indexed Git file mode: {mode!r}")
        entries.append((
            path_bytes.decode("utf-8", errors="surrogateescape"),
            mode.decode("ascii"),
            oid.decode("ascii"),
        ))
    return entries


def check_repository(root: Path) -> list[tuple[str, str]]:
    # Modification (2026-11-14): inspect exactly the staged Git blob bytes,
    # not potentially divergent worktree files. Filename checks include
    # symlinks/submodules, while byte checks apply only to regular blobs.
    return [
        (path, reason)
        for path, mode, oid in tracked_entries(root)
        if (reason := inspect_path(root, path, indexed_blob=oid, indexed_mode=mode)) is not None
    ]

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo", type=Path, default=Path(__file__).resolve().parents[1],
        help="Git checkout to inspect (default: parent of scripts/)",
    )
    args = parser.parse_args()
    try:
        violations = check_repository(args.repo.resolve())
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"Kernel distribution check failed: {exc}", file=sys.stderr)
        return 2
    if violations:
        for name, reason in violations:
            print(f"Forbidden tracked file {name!r}: {reason}", file=sys.stderr)
        print("Keep external JPL/NAIF kernels outside Git; gitignore alone is not sufficient.", file=sys.stderr)
        return 1
    print("OK: no tracked external ephemeris or NAIF kernel files detected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
