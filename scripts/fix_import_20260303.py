#!/usr/bin/env python3
"""
One-time fix for 2026-03-02 import issues.

Changes:
  A. Delete AMZN duplicates (1353, 1354, 1355) — already exist as 1266, 1267, 1268
  B. AMD IC 1337: update call legs 240/245 → 230/235 (roll), credit 1.29 → 1.49; delete SCV 1356
  C. BE IC 1301: update put legs 120/115 → 115/110 (roll), credit 3.19 → 2.80; delete SPV 1357, 1358
  D. BE SPV 1338: close with pnl = -6.0 (closed 2026-03-02 but orphan match missed it)

Usage:
  python3 scripts/fix_import_20260303.py trades.db
"""

import sqlite3
import json
import sys

DB = sys.argv[1] if len(sys.argv) > 1 else "trades.db"

# ── B: AMD IC 1337 updated legs (call side rolled: 240/245 → 230/235) ─────────
AMD_LEGS = [
    {"type": "short_put",  "strike": 175.0, "premium": 3.81, "closePremium": None, "expirationDate": "2026-03-27"},
    {"type": "long_put",   "strike": 170.0, "premium": 2.91, "closePremium": None, "expirationDate": "2026-03-27"},
    {"type": "short_call", "strike": 230.0, "premium": 1.74, "closePremium": None, "expirationDate": "2026-03-27"},
    {"type": "long_call",  "strike": 235.0, "premium": 1.31, "closePremium": None, "expirationDate": "2026-03-27"},
]

# ── C: BE IC 1301 updated legs (put side rolled: 120/115 → 115/110) ───────────
BE_LEGS = [
    {"type": "short_put",  "strike": 115.0, "premium": 4.87, "closePremium": None},
    {"type": "long_put",   "strike": 110.0, "premium": 3.80, "closePremium": None},
    {"type": "short_call", "strike": 180.0, "premium": 16.05, "closePremium": None},
    {"type": "long_call",  "strike": 185.0, "premium": 14.32, "closePremium": None},
]


def verify_preconditions(conn: sqlite3.Connection) -> bool:
    """Confirm expected trades exist before making changes."""
    ok = True

    def check(label: str, query: str, expected):
        nonlocal ok
        row = conn.execute(query).fetchone()
        val = row[0] if row else None
        if val != expected:
            print(f"  WARN: {label}: expected {expected!r}, got {val!r}")
            ok = False
        else:
            print(f"  OK: {label}")

    print("\nPre-flight checks:")
    check("AMZN 1353 exists",  "SELECT id FROM trades WHERE id=1353", 1353)
    check("AMZN 1354 exists",  "SELECT id FROM trades WHERE id=1354", 1354)
    check("AMZN 1355 exists",  "SELECT id FROM trades WHERE id=1355", 1355)
    check("AMD 1337 exists",   "SELECT id FROM trades WHERE id=1337", 1337)
    check("AMD 1356 exists",   "SELECT id FROM trades WHERE id=1356", 1356)
    check("BE 1301 exists",    "SELECT id FROM trades WHERE id=1301", 1301)
    check("BE 1338 exists",    "SELECT id FROM trades WHERE id=1338", 1338)
    check("BE 1357 exists",    "SELECT id FROM trades WHERE id=1357", 1357)
    check("BE 1358 exists",    "SELECT id FROM trades WHERE id=1358", 1358)
    return ok


def main():
    print(f"Connecting to {DB}...")
    with sqlite3.connect(DB) as conn:
        conn.row_factory = sqlite3.Row

        if not verify_preconditions(conn):
            print("\nPre-flight FAILED — aborting (no changes made).")
            sys.exit(1)

        # "Closed" state = exit_date IS NOT NULL (no separate status column)
        print("\nApplying changes...")

        # ── A. Delete AMZN duplicates ──────────────────────────────────────────
        n = conn.execute("DELETE FROM trades WHERE id IN (1353,1354,1355)").rowcount
        print(f"  A. Deleted {n} AMZN duplicate trade(s): 1353, 1354, 1355")

        # ── B. AMD IC 1337: rolled call side + new credit ──────────────────────
        conn.execute(
            "UPDATE trades SET credit_received=1.49, legs_json=? WHERE id=1337",
            (json.dumps(AMD_LEGS),)
        )
        print("  B. Updated AMD IC 1337: call legs 240/245 → 230/235, credit 1.29 → 1.49")

        n = conn.execute("DELETE FROM trades WHERE id=1356").rowcount
        print(f"     Deleted AMD SCV 1356 ({n} row)")

        # ── C. BE IC 1301: rolled put side + new credit + strikes ──────────────
        conn.execute(
            """UPDATE trades SET
                 credit_received = 2.80,
                 short_strike    = 115.0,
                 long_strike     = 110.0,
                 short_premium   = 4.87,
                 long_premium    = 3.80,
                 legs_json       = ?
               WHERE id = 1301""",
            (json.dumps(BE_LEGS),)
        )
        print("  C. Updated BE IC 1301: put legs 120/115 → 115/110, credit 3.19 → 2.80")

        n = conn.execute("DELETE FROM trades WHERE id IN (1357,1358)").rowcount
        print(f"     Deleted BE SPV 1357 and 1358 ({n} rows)")

        # ── D. Close BE SPV 1338 ───────────────────────────────────────────────
        conn.execute(
            "UPDATE trades SET exit_date='2026-03-02T16:00:00Z', pnl=-6.0 WHERE id=1338"
        )
        print("  D. Closed BE SPV 1338 (exit_date=2026-03-02, pnl=-6.0)")

        conn.commit()
        print("\nAll changes committed successfully.")

        # ── Verification summary ───────────────────────────────────────────────
        print("\nVerification:")

        deleted_ids = conn.execute(
            "SELECT id FROM trades WHERE id IN (1353,1354,1355,1356,1357,1358)"
        ).fetchall()
        if deleted_ids:
            print(f"  FAIL: still-existing deleted IDs: {[r[0] for r in deleted_ids]}")
        else:
            print("  OK: IDs 1353,1354,1355,1356,1357,1358 all deleted")

        amd = conn.execute(
            "SELECT credit_received, legs_json FROM trades WHERE id=1337"
        ).fetchone()
        if amd:
            cr = amd[0]
            call_strikes = [l["strike"] for l in json.loads(amd[1]) if "call" in l["type"]]
            print(f"  AMD 1337: credit={cr}, call strikes={call_strikes}")

        be_ic = conn.execute(
            "SELECT credit_received, short_strike, long_strike FROM trades WHERE id=1301"
        ).fetchone()
        if be_ic:
            print(f"  BE IC 1301: credit={be_ic[0]}, short_strike={be_ic[1]}, long_strike={be_ic[2]}")

        be_spv = conn.execute(
            "SELECT exit_date, pnl FROM trades WHERE id=1338"
        ).fetchone()
        if be_spv:
            print(f"  BE SPV 1338: exit_date={be_spv[0]}, pnl={be_spv[1]}")


if __name__ == "__main__":
    main()
