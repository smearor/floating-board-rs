#!/usr/bin/env python3
import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def run_git(args: list[str], cwd: Path) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout.strip()


def detect_repo_root(script_path: Path) -> Path:
    candidate_dir = script_path.parent
    try:
        top_level = run_git(["rev-parse", "--show-toplevel"], cwd=candidate_dir)
        return Path(top_level).resolve()
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(
            f"No valid Git repository found starting from {candidate_dir}."
        ) from exc


def detect_remote(repo_root: Path, preferred_remote: str | None) -> tuple[str, str]:
    remotes_raw = run_git(["remote"], cwd=repo_root)
    remotes = [line.strip() for line in remotes_raw.splitlines() if line.strip()]
    if not remotes:
        raise RuntimeError("No Git remote is configured in the repository.")

    if preferred_remote:
        if preferred_remote not in remotes:
            raise RuntimeError(
                f"Remote '{preferred_remote}' does not exist. Found: {', '.join(remotes)}"
            )
        remote_name = preferred_remote
    elif "origin" in remotes:
        remote_name = "origin"
    else:
        remote_name = remotes[0]

    remote_url = run_git(["remote", "get-url", remote_name], cwd=repo_root)
    return remote_name, remote_url


def copy_directory_contents(src_dir: Path, dst_dir: Path) -> None:
    if not src_dir.is_dir():
        raise FileNotFoundError(f"Source directory not found: {src_dir}")

    items = list(src_dir.iterdir())
    if not items:
        raise RuntimeError(f"Source directory is empty: {src_dir}")

    for item in items:
        target = dst_dir / item.name
        if item.is_dir():
            shutil.copytree(item, target, symlinks=True)
        else:
            shutil.copy2(item, target, follow_symlinks=False)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create an orphan branch from <repo>/docs/main/ in an external temp directory and push it."
    )
    parser.add_argument(
        "--branch",
        default="gh-pages",
        help="Target branch on the remote (default: gh-pages)",
    )
    parser.add_argument(
        "--remote",
        default=None,
        help="Name of the remote (default: origin, otherwise the first configured remote)",
    )
    parser.add_argument(
        "--message",
        default="docs: publish gh-pages from docs/main",
        help="Commit message for the orphan commit",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Force push (--force) if the target branch already exists",
    )
    args = parser.parse_args()

    script_path = Path(__file__).resolve()
    repo_root = detect_repo_root(script_path)
    remote_name, remote_url = detect_remote(repo_root, args.remote)
    docs_source = repo_root / "docs" / "main"

    print(f"Repository root: {repo_root}")
    print(f"Source:          {docs_source}")
    print(f"Remote:          {remote_name} -> {remote_url}")
    print(f"Target branch:   {args.branch}")

    with tempfile.TemporaryDirectory(prefix="gh-pages-deploy-") as tmp_dir_str:
        work_dir = Path(tmp_dir_str)
        copy_directory_contents(docs_source, work_dir)

        run_git(["init"], cwd=work_dir)
        run_git(["checkout", "--orphan", args.branch], cwd=work_dir)
        run_git(["add", "-A"], cwd=work_dir)
        run_git(["commit", "-m", args.message], cwd=work_dir)
        run_git(["remote", "add", remote_name, remote_url], cwd=work_dir)

        push_args = ["push"]
        if args.force:
            push_args.append("--force")
        push_args.extend(["-u", remote_name, args.branch])
        run_git(push_args, cwd=work_dir)

    print("Successfully pushed and cleaned up temporary directory.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)
