//! SQLite operational graph/read model (`rusqlite`, `bundled` feature —
//! no system SQLite dependency). This is a REBUILT read model: the
//! source of truth is the append-only graph-event log
//! ([`crate::schema::GraphEventLogEntry`]); this module deterministically
//! replays that log into a `nodes`/`edges` table pair. "Indexes are
//! disposable, knowledge is not" (owner intent) — the SQLite file itself
//! is never hand-edited and can always be thrown away and rebuilt from
//! the log.

use rusqlite::Connection;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA_VERSION;

    fn node_entry(seq: u64, id: &str, kind: &str) -> GraphEventLogEntry {
        GraphEventLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: format!("evt-{seq}"),
            event: GraphEventKind::NodeAdded {
                node_id: id.to_owned(),
                node_kind: kind.to_owned(),
            },
            ts: "2026-07-04T00:00:00Z".to_owned(),
            supersedes_seq: None,
        }
    }

    fn edge_entry(seq: u64, from: &str, to: &str, label: &str) -> GraphEventLogEntry {
        GraphEventLogEntry {
            schema_version: SCHEMA_VERSION,
            seq,
            id: format!("evt-{seq}"),
            event: GraphEventKind::EdgeAdded {
                from: from.to_owned(),
                to: to.to_owned(),
                label: label.to_owned(),
            },
            ts: "2026-07-04T00:00:00Z".to_owned(),
            supersedes_seq: None,
        }
    }

    #[test]
    fn apply_and_counts() -> Result<()> {
        let mut graph = OperationalGraph::open_in_memory()?;
        graph.apply(&node_entry(0, "a", "file"))?;
        graph.apply(&node_entry(1, "b", "file"))?;
        graph.apply(&edge_entry(2, "a", "b", "imports"))?;
        assert_eq!(graph.node_count()?, 2);
        assert_eq!(graph.edge_count()?, 1);
        Ok(())
    }

    #[test]
    fn rebuild_is_deterministic() -> Result<()> {
        let entries = vec![
            node_entry(0, "a", "file"),
            node_entry(1, "b", "file"),
            edge_entry(2, "a", "b", "imports"),
            node_entry(3, "c", "symbol"),
        ];
        let mut first = OperationalGraph::open_in_memory()?;
        first.rebuild(&entries)?;
        let mut second = OperationalGraph::open_in_memory()?;
        second.rebuild(&entries)?;
        assert_eq!(first.nodes_snapshot()?, second.nodes_snapshot()?);
        assert_eq!(first.node_count()?, second.node_count()?);
        assert_eq!(first.edge_count()?, second.edge_count()?);

        // Rebuilding a THIRD time by replaying twice into the same
        // database (idempotent apply) must not change the counts.
        first.rebuild(&entries)?;
        assert_eq!(first.node_count()?, 3);
        assert_eq!(first.edge_count()?, 1);
        Ok(())
    }

    #[test]
    fn later_seq_supersedes_node_kind_for_the_same_id() -> Result<()> {
        let mut graph = OperationalGraph::open_in_memory()?;
        graph.apply(&node_entry(0, "a", "file"))?;
        graph.apply(&node_entry(1, "a", "symbol"))?;
        let snapshot = graph.nodes_snapshot()?;
        assert_eq!(snapshot, vec![("a".to_owned(), "symbol".to_owned())]);
        Ok(())
    }
}
