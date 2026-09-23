#!/usr/bin/env python3
"""Validate the current checkout, including uncommitted and untracked source.

Uses a disposable source copy, so the seeded gate in the checkout is untouched.
Keeps command logs and both runs' evidence under evidence/check-<timestamp>/.
No simulator packages are needed: this validates the mock backend only.
"""

import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_SEED_FAILURES = {"boundary_251ms_rejects", "configured_threshold_is_respected", "stale_ages_are_rejected"}


def checkout_copy(destination):
    files = subprocess.check_output(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=ROOT)
    for filename in set(files.split(b"\0")) - {b""}:
        relative = Path(os.fsdecode(filename))
        source, target = ROOT / relative, destination / relative
        if source.is_file() or source.is_symlink():
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target, follow_symlinks=False)


def main():
    run_id = "check-" + datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    reports = ROOT / "evidence" / run_id
    reports.mkdir(parents=True)
    runners = ROOT / ".runners"
    runners.mkdir(exist_ok=True)
    python = os.environ.get("ROBOT_DEMO_PYTHON", sys.executable)
    python = subprocess.check_output([python, "-c", "import sys; print(sys.executable)"], text=True).strip()
    print(f"Validating current checkout; logs and evidence: {reports}", flush=True)
    source_identity = {
        "git_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "git_status": subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True),
        "source": "copy of tracked and untracked non-ignored checkout files; includes uncommitted edits",
        "backend": "mock",
    }
    (reports / "checkout-source.json").write_text(json.dumps(source_identity, indent=2) + "\n")
    with tempfile.TemporaryDirectory(prefix="check.", dir=runners) as temporary:
        temp = Path(temporary)
        source = temp / "src"
        source.mkdir()
        checkout_copy(source)
        env = dict(os.environ, ROBOT_DEMO_ROOT=str(source), ROBOT_DEMO_PYTHON=python,
                   CARGO_TARGET_DIR=str(temp / "target"), GIT_CEILING_DIRECTORIES=str(temp))
        # Local rendering/configuration choices should not change the mock check.
        env.pop("ROBOT_DEMO_VIDEO_DIR", None)
        env.pop("ROBOT_DEMO_PATCH", None)

        def run(label, arguments, allowed=(0,), cwd=source):
            print(f"  {label}", flush=True)
            completed = subprocess.run(arguments, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            (reports / f"{label}.log").write_text(completed.stdout, encoding="utf-8")
            if completed.returncode not in allowed:
                print(completed.stdout, file=sys.stderr)
                raise RuntimeError(f"{label} exited {completed.returncode}")
            return completed

        try:
            run("acceptance-negative-tests", [python, "-m", "unittest", "discover", "-s", "demo/robot-sim/acceptance"])
            run("patch-allowlist-tests", [python, "-m", "unittest", "discover", "-s", "scripts/robot-demo/tests"])
            if (source / "demo/robot-sim/tests").is_dir():
                run("bridge-tests", [python, "-m", "unittest", "discover", "-s", "demo/robot-sim/tests"])
            if (source / "scripts/site/build.py").is_file():
                run("site-check", [python, "scripts/site/build.py", "--check"])
            run("seeded-unit-tests", ["cargo", "test", "--workspace", "--lib", "--locked"], cwd=source / "rust")
            seeded = run("expected-seed-contract-failures", ["cargo", "test", "-p", "robot-safety-gate", "--test", "contract", "--locked", "--", "--color", "never"], allowed=(101,), cwd=source / "rust")
            failed_tests = set(re.findall(r"^test (\S+) \.\.\. FAILED$", seeded.stdout, re.MULTILINE))
            if failed_tests != EXPECTED_SEED_FAILURES:
                raise RuntimeError(f"expected precisely {sorted(EXPECTED_SEED_FAILURES)}; got {sorted(failed_tests)}")
            run("seeded-build", ["cargo", "build", "--workspace", "--locked"], cwd=source / "rust")
            executable = temp / "target/debug/swf-cli"
            run("seeded-stale-episode", [str(executable), "robot-demo", "run", "--scenario", "stale_600ms", "--backend", "mock", "--run-id", "seeded"])
            seeded_results = json.loads((source / "evidence/seeded/scenario-results.json").read_text())
            if len(seeded_results) != 1 or seeded_results[0]["outcome"] != "cube_lifted" or seeded_results[0]["task_dispatches"] <= 0:
                raise RuntimeError("seeded episode did not reproduce the stale-dispatch regression")
            run("expected-seed-verifier-failure", [python, "demo/robot-sim/acceptance/verify_run.py", str(source / "evidence/seeded"), "--scenario", "stale_600ms"], allowed=(1,))
            # Export the seed executable before the patched build replaces it.
            artifact = source / "evidence/seeded/artifact"
            artifact.mkdir()
            shutil.copy2(executable, artifact / "swf-cli")
            patch = source / "demo/fallback-patch.diff"
            run("candidate-allowlist", [python, "scripts/robot-demo/check-patch.py", str(patch)])
            run("candidate-apply", ["git", "apply", "--", str(patch)])
            env["ROBOT_DEMO_PATCH"] = "demo/fallback-patch.diff"
            run("patched-workspace-tests", ["cargo", "test", "--workspace", "--locked"], cwd=source / "rust")
            run("patched-build", ["cargo", "build", "--workspace", "--locked"], cwd=source / "rust")
            run("patched-matrix", [str(executable), "robot-demo", "matrix", "--backend", "mock", "--run-id", "patched"])
            artifact = source / "evidence/patched/artifact"
            artifact.mkdir()
            shutil.copy2(executable, artifact / "swf-cli")
            run("patched-protected-verifier", [python, "demo/robot-sim/acceptance/verify_run.py", str(source / "evidence/patched")])
        finally:
            evidence = source / "evidence"
            if evidence.is_dir():
                for path in evidence.iterdir():
                    if path.is_dir():
                        shutil.copytree(path, reports / path.name, dirs_exist_ok=True)
    print(f"PASS: expected seeded failures + patched workspace tests + full mock matrix.\nEvidence: {reports}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"CHECK FAILED: {error}", file=sys.stderr)
        sys.exit(1)
