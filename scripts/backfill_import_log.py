#!/usr/bin/env python3
"""
One-time backfill: populate import_log from existing trades, then delete
re-imported duplicate trades 1359-1366 (created by a second run of the
same Schwab JSON file before the import_log fix was in place).

Usage:
  python3 scripts/backfill_import_log.py trades.db
"""

import sqlite3
import sys

DB = sys.argv[1] if len(sys.argv) > 1 else "trades.db"

# Trades incorrectly re-imported on the second run (duplicates of already-existing trades).
# Their fingerprints are captured in the backfill step BEFORE deletion so they stay
# in import_log and block future re-imports.
DUPLICATE_IDS = [1359, 1360, 1361, 1362, 1363, 1364, 1365, 1366]


def main():
    print(f"Connecting to {DB}...")
    with sqlite3.connect(DB) as conn:

        # ── 1. Create import_log table if not already there ───────────────────
        conn.execute("""
            CREATE TABLE IF NOT EXISTS import_log (
                fingerprint TEXT PRIMARY KEY,
                trade_id    INTEGER,
                imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        """)
        conn.commit()
        print("import_log table ready.")

        # ── 2. Backfill fingerprints from ALL current trades ──────────────────
        #    (includes 1359-1366 so their fingerprints persist after deletion)
        rows = conn.execute(
            "SELECT id, trade_date, ticker, quantity, credit_received FROM trades"
        ).fetchall()

        inserted = 0
        for trade_id, trade_date, ticker, qty, credit in rows:
            fp = f"{trade_date[:10]}|{ticker}|{qty}|{credit:.2f}"
            result = conn.execute(
                "INSERT OR IGNORE INTO import_log (fingerprint, trade_id) VALUES (?, ?)",
                (fp, trade_id)
            )
            inserted += result.rowcount
        conn.commit()
        skipped = len(rows) - inserted
        print(f"Backfilled {inserted} fingerprint(s) into import_log "
              f"({skipped} already present, skipped).")

        # ── 3. Delete the re-imported duplicate trades ────────────────────────
        existing = conn.execute(
            f"SELECT id FROM trades WHERE id IN ({','.join(str(i) for i in DUPLICATE_IDS)})"
        ).fetchall()
        existing_ids = [r[0] for r in existing]

        if existing_ids:
            n = conn.execute(
                f"DELETE FROM trades WHERE id IN ({','.join(str(i) for i in existing_ids)})"
            ).rowcount
            conn.commit()
            print(f"Deleted {n} re-imported duplicate trade(s): {existing_ids}")
        else:
            print("No re-imported duplicates found (already cleaned up).")

        # ── 4. Summary ────────────────────────────────────────────────────────
        total_fps    = conn.execute("SELECT COUNT(*) FROM import_log").fetchone()[0]
        total_trades = conn.execute("SELECT COUNT(*) FROM trades").fetchone()[0]
        print(f"\nDone. {total_trades} trades in DB, {total_fps} fingerprints in import_log.")
        print("Re-run the import with the same Schwab JSON to verify 0 new trades are imported.")


if __name__ == "__main__":
    main()
