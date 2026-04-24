#!/usr/bin/env python3
import argparse
import ctypes
import os
import sys
from pathlib import Path


SYMBOLIC_LINK_FLAG_DIRECTORY = 0x1
SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE = 0x2


def parse_link(raw: str) -> tuple[str, str]:
    dest, sep, src = raw.partition("=")
    if not sep or not dest or not src:
        raise argparse.ArgumentTypeError("expected DEST=SOURCE")
    return dest, src


def ensure_symlink(destination: Path, source: Path) -> None:
    if destination.exists() or destination.is_symlink():
        if destination.is_symlink():
            destination.unlink()
        else:
            raise RuntimeError(f"destination already exists and is not a symlink: {destination}")

    destination.parent.mkdir(parents=True, exist_ok=True)
    relative_target = os.path.relpath(source, destination.parent)
    create_symlink(relative_target, destination, source.is_dir())


def create_symlink(relative_target: str, destination: Path, is_dir: bool) -> None:
    if os.name != "nt":
        os.symlink(relative_target, destination, target_is_directory=is_dir)
        return

    flags = SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE
    if is_dir:
        flags |= SYMBOLIC_LINK_FLAG_DIRECTORY

    kernel32 = ctypes.windll.kernel32
    kernel32.CreateSymbolicLinkW.argtypes = (
        ctypes.c_wchar_p,
        ctypes.c_wchar_p,
        ctypes.c_uint32,
    )
    kernel32.CreateSymbolicLinkW.restype = ctypes.c_ubyte

    if kernel32.CreateSymbolicLinkW(str(destination), relative_target, flags):
        return

    last_error = ctypes.GetLastError()
    if last_error == 1314:
        raise RuntimeError(
            "symlink creation requires Windows Developer Mode or an elevated shell"
        )
    if last_error != 87:
        raise OSError(last_error, "CreateSymbolicLinkW failed", str(destination))

    fallback_flags = SYMBOLIC_LINK_FLAG_DIRECTORY if is_dir else 0
    if kernel32.CreateSymbolicLinkW(str(destination), relative_target, fallback_flags):
        return

    last_error = ctypes.GetLastError()
    if last_error == 1314:
        raise RuntimeError(
            "symlink creation requires Windows Developer Mode or an elevated shell"
        )
    raise OSError(last_error, "CreateSymbolicLinkW failed", str(destination))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create a linked example workspace that preserves example-relative layout."
    )
    parser.add_argument("--example-root", required=True)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--link", dest="links", action="append", type=parse_link, required=True)
    args = parser.parse_args()

    example_root = Path(args.example_root).resolve()
    workspace = Path(args.workspace).resolve()
    workspace.mkdir(parents=True, exist_ok=True)

    for dest_rel, src_rel in args.links:
        source = (example_root / src_rel).resolve()
        if not source.exists():
            raise RuntimeError(f"source does not exist: {source}")
        destination = workspace / dest_rel
        ensure_symlink(destination, source)
        print(f"linked {destination} -> {source}")

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
