//! Embedded SQLite projection for instant O(1) reads of CynicPeer accounts.

use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

use cynic_consensus::peer::{CitizenAccount, MandateAccount, NodeAccount, PeerState};

#[derive(Debug)]
pub struct PeerSqliteProjection {
    path: PathBuf,
}

impl PeerSqliteProjection {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("failed to create dir: {e}"))?;
        }

        let conn = Connection::open(&path).map_err(|e| format!("sqlite open failed: {e}"))?;
        Self::init_schema(&conn)?;

        Ok(Self { path })
    }

    fn init_schema(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS citizens (
                citizen_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                public_key_hex TEXT NOT NULL,
                is_admin INTEGER NOT NULL,
                status TEXT NOT NULL,
                json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS nodes (
                node_id TEXT PRIMARY KEY,
                owner_citizen_id TEXT NOT NULL,
                hostname TEXT NOT NULL,
                status TEXT NOT NULL,
                json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS offices (
                office_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS mandates (
                mandate_id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                office_id TEXT NOT NULL,
                fencing_token INTEGER NOT NULL,
                status TEXT NOT NULL,
                json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS resources (
                resource_id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                cpu_available INTEGER NOT NULL,
                ram_available INTEGER NOT NULL,
                json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS treasury (
                treasury_id TEXT PRIMARY KEY,
                cyd_credits INTEGER NOT NULL,
                json TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("init schema failed: {e}"))?;
        Ok(())
    }

    /// Project the entire PeerState into SQLite inside an atomic transaction.
    pub fn project_state(&self, state: &PeerState) -> Result<(), String> {
        let mut conn = Connection::open(&self.path).map_err(|e| format!("open failed: {e}"))?;
        let tx = conn.transaction().map_err(|e| format!("tx failed: {e}"))?;

        // Update meta
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('epoch', ?1), ('last_sequence', ?2), ('last_tx_hash', ?3)",
            params![state.epoch.to_string(), state.last_sequence.to_string(), state.last_tx_hash],
        ).map_err(|e| format!("meta update failed: {e}"))?;

        // Update citizens
        for citizen in state.citizens.values() {
            let json = serde_json::to_string(citizen).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO citizens (citizen_id, display_name, public_key_hex, is_admin, status, json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![citizen.citizen_id, citizen.display_name, citizen.public_key_hex, citizen.is_sovereign_admin as i32, format!("{:?}", citizen.status), json],
            ).map_err(|e| format!("citizen insert failed: {e}"))?;
        }

        // Update nodes
        for node in state.nodes.values() {
            let json = serde_json::to_string(node).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO nodes (node_id, owner_citizen_id, hostname, status, json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![node.node_id, node.owner_citizen_id, node.hostname, format!("{:?}", node.status), json],
            ).map_err(|e| format!("node insert failed: {e}"))?;
        }

        // Update offices
        for office in state.offices.values() {
            let json = serde_json::to_string(office).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO offices (office_id, title, json) VALUES (?1, ?2, ?3)",
                params![office.office_id, office.title, json],
            )
            .map_err(|e| format!("office insert failed: {e}"))?;
        }

        // Update mandates
        for mandate in state.mandates.values() {
            let json = serde_json::to_string(mandate).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO mandates (mandate_id, agent_id, office_id, fencing_token, status, json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![mandate.mandate_id, mandate.agent_id, mandate.office_id, mandate.fencing_token as i64, format!("{:?}", mandate.status), json],
            ).map_err(|e| format!("mandate insert failed: {e}"))?;
        }

        // Update resources
        for res in state.resources.values() {
            let json = serde_json::to_string(res).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO resources (resource_id, node_id, cpu_available, ram_available, json) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![res.resource_id, res.node_id, res.cpu_cores_available as i64, res.ram_bytes_available as i64, json],
            ).map_err(|e| format!("resource insert failed: {e}"))?;
        }

        // Update treasury
        for trs in state.treasury.values() {
            let json = serde_json::to_string(trs).unwrap_or_default();
            tx.execute(
                "INSERT OR REPLACE INTO treasury (treasury_id, cyd_credits, json) VALUES (?1, ?2, ?3)",
                params![trs.treasury_id, trs.cyd_calibration_credits as i64, json],
            ).map_err(|e| format!("treasury insert failed: {e}"))?;
        }

        tx.commit().map_err(|e| format!("tx commit failed: {e}"))?;
        Ok(())
    }

    /// Fast O(1) indexed query for a citizen
    pub fn get_citizen(&self, citizen_id: &str) -> Result<Option<CitizenAccount>, String> {
        let conn = Connection::open(&self.path).map_err(|e| format!("open failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT json FROM citizens WHERE citizen_id = ?1")
            .map_err(|e| format!("prepare failed: {e}"))?;
        let mut rows = stmt
            .query(params![citizen_id])
            .map_err(|e| format!("query failed: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("row failed: {e}"))? {
            let json: String = row.get(0).map_err(|e| format!("get failed: {e}"))?;
            let account =
                serde_json::from_str(&json).map_err(|e| format!("deserialize failed: {e}"))?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    /// Fast O(1) indexed query for a node
    pub fn get_node(&self, node_id: &str) -> Result<Option<NodeAccount>, String> {
        let conn = Connection::open(&self.path).map_err(|e| format!("open failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT json FROM nodes WHERE node_id = ?1")
            .map_err(|e| format!("prepare failed: {e}"))?;
        let mut rows = stmt
            .query(params![node_id])
            .map_err(|e| format!("query failed: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("row failed: {e}"))? {
            let json: String = row.get(0).map_err(|e| format!("get failed: {e}"))?;
            let account =
                serde_json::from_str(&json).map_err(|e| format!("deserialize failed: {e}"))?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    /// Fast O(1) indexed query for a mandate
    pub fn get_mandate(&self, mandate_id: &str) -> Result<Option<MandateAccount>, String> {
        let conn = Connection::open(&self.path).map_err(|e| format!("open failed: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT json FROM mandates WHERE mandate_id = ?1")
            .map_err(|e| format!("prepare failed: {e}"))?;
        let mut rows = stmt
            .query(params![mandate_id])
            .map_err(|e| format!("query failed: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("row failed: {e}"))? {
            let json: String = row.get(0).map_err(|e| format!("get failed: {e}"))?;
            let account =
                serde_json::from_str(&json).map_err(|e| format!("deserialize failed: {e}"))?;
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }
}
