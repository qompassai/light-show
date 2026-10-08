#!/usr/bin/env python3
"""tag_answers.py — replace plaintext answers in level JSONs with
keyed HMAC-SHA256 tags (answer-tag migration, 2026-10-07).

Ships in the repo so future level authoring can follow the same
path: author levels with plaintext answers locally, run this tool
on primo, commit only the tagged result.

What it tags (payload forms match game/src/answer_verify.rs):
  quiz          "correct_idx": i        -> "correct_tag"
                HMAC("quiz", question_id, str(i))
  api_sequence  "expected": [ops]       -> "expected_tag" +
                "expected_step_tags" (one tag per order prefix)
                HMAC("api_sequence", level_id, "OpA,OpB,...")
  alarm_triage  "expected_order": [ids] -> "expected_order_tag" +
                "expected_step_tags"
                HMAC("alarm_triage", level_id, "3,1,2")
  warehouse     game/src/warehouse/mod.rs QuizQuestion literals:
                "correct_idx": i -> "correct_tag", scoped by the
                literal's stable "id"
                HMAC("quiz", question_id, str(i))
                Literals already tagged are verified against the
                store instead, so re-runs are idempotent and any
                store/literal drift aborts.

Step tags exist because the consoles give per-step feedback;
the last step tag always equals the full-sequence tag.

Custody: the production key is read at runtime from the primo
sops answer store (never printed, never written anywhere). Every
plaintext answer found is cross-checked against the store first —
any drift aborts before a single file is touched. Power windows
and test fixtures are deliberately NOT tagged (see the audit in
~/workspace/security/answer-store/).

Usage (primo only):
  python3 tools/tag_answers.py [--dry-run]
Env overrides: LIGHT_SHOW_REPO, LIGHT_SHOW_ANSWER_STORE,
SOPS_AGE_KEY_FILE (default: the dedicated answer-store key).
"""
import argparse
import hashlib
import hmac
import json
import os
import re
import subprocess
import sys

HOME = os.path.expanduser("~")
REPO = os.environ.get("LIGHT_SHOW_REPO", os.path.join(HOME, "workspace/repos/light-show"))
STORE = os.environ.get(
    "LIGHT_SHOW_ANSWER_STORE",
    os.path.join(HOME, "workspace/security/answer-store/answers.enc.json"),
)
AGE_KEY = os.environ.get(
    "SOPS_AGE_KEY_FILE",
    os.path.join(HOME, ".config/sops/age/keys-light-show-answers.txt"),
)
LEVELS_DIR = os.path.join(REPO, "game/assets/levels")
WAREHOUSE_MOD = os.path.join(REPO, "game/src/warehouse/mod.rs")


def load_store():
    env = dict(os.environ, SOPS_AGE_KEY_FILE=AGE_KEY)
    out = subprocess.run(
        ["sops", "-d", "--input-type", "json", "--output-type", "json", STORE],
        capture_output=True, text=True, env=env, check=True,
    )
    return json.loads(out.stdout)


def tag(key: bytes, domain: str, scope: str, payload: str) -> str:
    msg = f"{domain}:{scope}:{payload}".encode()
    return hmac.new(key, msg, hashlib.sha256).hexdigest()


def op_names(ops) -> str:
    return ",".join(ops)


def warehouse_pass(key, entries, dry_run):
    """Tag (or verify) the code-authored warehouse quiz literals.

    The warehouse questions live in Rust source, not level JSON, so
    this pass rewrites game/src/warehouse/mod.rs in place: each
    QuizQuestion literal's `correct_idx: N,` line becomes
    `correct_tag: "<hex>",`, scoped by the literal's stable `id`.
    Store entries are `warehouse/<tool-id>/<question-id>`; any
    disagreement between store and literal aborts before the file is
    touched. Already-tagged literals are re-verified against the
    store, which makes the pass idempotent and turns it into the
    regeneration path: update the store entry, restore a plaintext
    literal by hand, re-run.
    """
    store_idx = {}
    for eid, e in entries.items():
        parts = eid.split("/")
        if len(parts) == 3 and parts[0] == "warehouse":
            store_idx[parts[2]] = e["value"]
    assert store_idx, "no warehouse entries in the store"

    with open(WAREHOUSE_MOD) as fh:
        text = fh.read()

    seen = {}
    out = []
    current = None
    tagged = verified = 0
    for line in text.splitlines(keepends=True):
        m = re.match(r'\s*id: "([^"]+)",\s*$', line)
        if m and m.group(1) in store_idx:
            current = m.group(1)
        m = re.match(r"(\s*)correct_idx: (\d+),\s*$", line)
        if m:
            assert current is not None, "warehouse correct_idx with no question id above it"
            idx = int(m.group(2))
            sv = store_idx[current]
            assert sv == idx, f"store drift on warehouse {current}: {sv} != {idx}"
            hex_tag = tag(key, "quiz", current, str(idx))
            line = f'{m.group(1)}correct_tag: "{hex_tag}",\n'
            seen[current] = idx
            tagged += 1
            current = None
        else:
            m = re.match(r'\s*correct_tag: "([0-9a-f]{64})",\s*$', line)
            if m and current is not None:
                want = tag(key, "quiz", current, str(store_idx[current]))
                assert m.group(1) == want, f"store drift on warehouse {current}: tag mismatch"
                seen[current] = store_idx[current]
                verified += 1
                current = None
        out.append(line)
    assert set(seen) == set(store_idx), (
        f"warehouse literal/store mismatch: literals {sorted(seen)} "
        f"vs store {sorted(store_idx)}"
    )
    if tagged and not dry_run:
        with open(WAREHOUSE_MOD, "w") as fh:
            fh.write("".join(out))
    return tagged, verified


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    store = load_store()
    entries = store["entries"]
    key_hex = store["answer_key"]["production"]
    assert len(key_hex) == 64, "production key must be 64 hex chars"
    # The verifier (game/src/answer_verify.rs) HMACs with the key
    # string's own bytes — the hex text as UTF-8 — NOT the bytes the
    # hex decodes to. Match it exactly, or tags verify nowhere.
    key = key_hex.encode()

    def entry_value(eid):
        e = entries.get(eid)
        return None if e is None else e["value"]

    counts = {"quiz": 0, "api": 0, "alarm": 0}
    changed = []
    for name in sorted(os.listdir(LEVELS_DIR)):
        if not name.endswith(".json"):
            continue
        path = os.path.join(LEVELS_DIR, name)
        with open(path) as fh:
            text = fh.read()
        data = json.loads(text)
        level_id = data["id"]
        dirty = False

        quiz = data.get("quiz")
        if quiz:
            for q in quiz["questions"]:
                if "correct_idx" not in q:
                    continue
                idx = q["correct_idx"]
                sv = entry_value(f"quiz/{level_id}/{q['id']}")
                assert sv == idx, f"store drift on quiz {q['id']}: {sv} != {idx}"
                q["correct_tag"] = tag(key, "quiz", q["id"], str(idx))
                del q["correct_idx"]
                counts["quiz"] += 1
                dirty = True

        seq = data.get("api_sequence")
        if seq and "expected" in seq:
            expected = seq["expected"]
            sv = entry_value(f"api_sequence/{level_id}")
            assert sv == expected, f"store drift on api {level_id}"
            seq["expected_tag"] = tag(key, "api_sequence", level_id, op_names(expected))
            seq["expected_step_tags"] = [
                tag(key, "api_sequence", level_id, op_names(expected[:i]))
                for i in range(1, len(expected) + 1)
            ]
            assert seq["expected_step_tags"][-1] == seq["expected_tag"]
            del seq["expected"]
            counts["api"] += 1
            dirty = True

        tri = data.get("alarm_triage")
        if tri and "expected_order" in tri:
            order = tri["expected_order"]
            sv = entry_value(f"alarm_triage/{level_id}")
            assert sv == order, f"store drift on triage {level_id}"
            payload = ",".join(str(i) for i in order)
            tri["expected_order_tag"] = tag(key, "alarm_triage", level_id, payload)
            tri["expected_step_tags"] = [
                tag(key, "alarm_triage", level_id, ",".join(str(i) for i in order[:k]))
                for k in range(1, len(order) + 1)
            ]
            assert tri["expected_step_tags"][-1] == tri["expected_order_tag"]
            del tri["expected_order"]
            counts["alarm"] += 1
            dirty = True

        if dirty:
            changed.append(name)
            if not args.dry_run:
                # Surgical rewrite: regenerate only this file's JSON with
                # 2-space indent (matches the shipped level format).
                with open(path, "w") as fh:
                    json.dump(data, fh, indent=2)
                    fh.write("\n")

    w_tagged, w_verified = warehouse_pass(key, entries, args.dry_run)
    counts["warehouse"] = w_tagged

    print(f"quiz tagged: {counts['quiz']}, api: {counts['api']}, alarm: {counts['alarm']}")
    print(f"warehouse tagged: {w_tagged}, warehouse verified: {w_verified}")
    print(f"files changed: {len(changed)}")
    if not any(counts.values()):
        print("no plaintext answers found — tree is already fully tagged")
    return 0


if __name__ == "__main__":
    sys.exit(main())
