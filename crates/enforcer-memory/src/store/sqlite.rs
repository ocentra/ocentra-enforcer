//! SQLite operational graph/read model (`rusqlite`, `bundled` feature —
//! no system SQLite dependency). This is a REBUILT read model: the
//! source of truth is the append-only graph-event log
//! ([`crate::schema::GraphEventLogEntry`]); this module deterministically
//! replays that log into a `nodes`/`edges` table pair. "Indexes are
//! disposable, knowledge is not" (owner intent) — the SQLite file itself
//! is never hand-edited and can always be thrown away and rebuilt from
//! the log.

use rusqlite::{Connection, OpenFlags};

use crate::error::Result;
use crate::schema::GraphEventKind;
use crate::schema::GraphEventLogEntry;

/// An in-memory-or-file-backed SQLite operational read model.
pub struct OperationalGraph {
    conn: Connection,
}

impl OperationalGraph {
    /// Open (or create) the SQLite database at `path` and ensure the
    /// schema exists.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// Open an existing SQLite read model without creating or migrating
    /// anything. Query surfaces use this when they need to consume the
    /// Store-backed projection without turning a read into a hidden index
    /// build or filesystem mutation.
    pub fn open_read_only(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(Self { conn })
    }

    /// An in-memory database, useful for tests and one-shot rebuilds
    /// that do not need to persist to disk.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nodes (
                node_id   TEXT PRIMARY KEY,
                node_kind TEXT NOT NULL,
                last_seq  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS edges (
                from_id TEXT NOT NULL,
                to_id   TEXT NOT NULL,
                label   TEXT NOT NULL,
                seq     INTEGER NOT NULL,
                PRIMARY KEY (from_id, to_id, label)
            );
            CREATE TABLE IF NOT EXISTS applied_events (
                seq INTEGER PRIMARY KEY
            );",
        )?;
        Ok(Self { conn })
    }

    /// Apply one graph-event log entry, skipping it if its `seq` has
    /// already been applied (idempotent replay — rebuilding from seq 0
    /// against an already-populated database is safe).
    pub fn apply(&mut self, entry: &GraphEventLogEntry) -> Result<()> {
        let already_applied: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM applied_events WHERE seq = ?1)",
            [entry.seq],
            |row| row.get(0),
        )?;
        if already_applied {
            return Ok(());
        }
        match &entry.event {
            GraphEventKind::NodeAdded { node_id, node_kind } => {
                self.conn.execute(
                    "INSERT INTO nodes (node_id, node_kind, last_seq) VALUES (?1, ?2, ?3)
                     ON CONFLICT(node_id) DO UPDATE SET node_kind = excluded.node_kind, last_seq = excluded.last_seq",
                    (node_id, node_kind, entry.seq),
                )?;
            }
            GraphEventKind::EdgeAdded { from, to, label } => {
                self.conn.execute(
                    "INSERT INTO edges (from_id, to_id, label, seq) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(from_id, to_id, label) DO UPDATE SET seq = excluded.seq",
                    (from, to, label, entry.seq),
                )?;
            }
        }
        self.conn
            .execute("INSERT INTO applied_events (seq) VALUES (?1)", [entry.seq])?;
        Ok(())
    }

    /// Rebuild the read model from scratch by replaying every entry in
    /// `entries` (in order) through [`OperationalGraph::apply`].
    /// Deterministic: replaying the same entries twice into two fresh
    /// databases yields identical node/edge sets (see the rebuild
    /// determinism test).
    pub fn rebuild(&mut self, entries: &[GraphEventLogEntry]) -> Result<()> {
        for entry in entries {
            self.apply(entry)?;
        }
        Ok(())
    }

    pub fn node_count(&self) -> Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?)
    }

    pub fn edge_count(&self) -> Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?)
    }

    /// Snapshot of every `(node_id, node_kind)` pair, sorted by
    /// `node_id`, for deterministic comparison in tests.
    pub fn nodes_snapshot(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT node_id, node_kind FROM nodes ORDER BY node_id")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Snapshot of every `(from_id, to_id, label)` edge, sorted for
    /// deterministic projection replay and tests.
    pub fn edges_snapshot(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT from_id, to_id, label FROM edges ORDER BY from_id, to_id, label")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
