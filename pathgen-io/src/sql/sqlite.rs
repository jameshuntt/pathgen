use std::{io::{self, Write}, path::Path};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    ArtifactOutputPlan, ArtifactWriteClock, ArtifactWriteReceipt, PayloadDigest, ResultExt,
};

/// Schema identifier for SQLite migration receipts.
pub const SQLITE_MIGRATION_RECEIPT_SCHEMA: &str = "pathgen.sqlite_migration_receipt.v1";

/// Logical schema name managed by this SQLite sink.
pub const SQLITE_AUDIT_SCHEMA_NAME: &str = "pathgen.sqlite.audit";

/// Schema identifier for SQLite retention/compaction planning receipts.
pub const SQLITE_RETENTION_RECEIPT_SCHEMA: &str = "pathgen.sqlite_retention_receipt.v1";

/// Current schema version for the pathgen SQLite audit/index sink.
pub const SQLITE_AUDIT_SCHEMA_VERSION: u32 = 3;

const SQLITE_AUDIT_SCHEMA_MIGRATION_ID: &str = "pathgen_sqlite_audit_schema_v3";
const SQLITE_AUDIT_SCHEMA_APPLIED_BY: &str = "pathgen-io";

/// Open a SQLite connection for an already-resolved database path.
///
/// This helper intentionally accepts a path that has already gone through
/// pathgen's route/policy layer. The SQLite adapter is a sink, not a path
/// authority.
pub fn open_connection<P>(path: P) -> io::Result<Connection>
where
    P: AsRef<Path>,
{
    Connection::open(path).invalid_data()
}

/// Small SQLite audit/index sink for pathgen artifact plans and write receipts.
///
/// This sink is deliberately not the canonical artifact store. The canonical
/// bytes can still live in `file::json`, `file::txt`, etc. SQLite is used here
/// as a searchable catalog of output plans and write evidence.
pub struct SqliteArtifactAuditSink {
    conn: Connection,
}

impl SqliteArtifactAuditSink {
    pub fn open<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            conn: open_connection(path)?,
        })
    }

    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn into_inner(self) -> Connection {
        self.conn
    }

    /// Create the audit/index schema if it does not already exist.
    pub fn init_schema(&self) -> io::Result<()> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS pathgen_output_plans (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    app_name TEXT NOT NULL,
                    env TEXT NOT NULL,
                    route_id TEXT NOT NULL,
                    schema_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    version INTEGER NOT NULL,
                    endpoint TEXT NOT NULL,
                    target_kind TEXT NOT NULL,
                    target_path TEXT NOT NULL,
                    bytes_len INTEGER NOT NULL,
                    payload_digest TEXT NOT NULL,
                    policy_version TEXT NOT NULL,
                    op_word TEXT,
                    op_clean_addr TEXT
                );

                CREATE INDEX IF NOT EXISTS idx_pathgen_output_plans_route
                    ON pathgen_output_plans(route_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_output_plans_schema
                    ON pathgen_output_plans(schema_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_output_plans_payload_digest
                    ON pathgen_output_plans(payload_digest);

                CREATE TABLE IF NOT EXISTS pathgen_write_receipts (
                    receipt_id TEXT PRIMARY KEY,
                    created_at_unix_ms INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    app_name TEXT NOT NULL,
                    env TEXT NOT NULL,
                    route_id TEXT NOT NULL,
                    schema_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    version INTEGER NOT NULL,
                    endpoint TEXT NOT NULL,
                    path TEXT NOT NULL,
                    bytes_written INTEGER NOT NULL,
                    payload_digest TEXT NOT NULL,
                    policy_version TEXT NOT NULL,
                    op_word TEXT,
                    previous_receipt_digest TEXT,
                    receipt_digest TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_pathgen_write_receipts_route
                    ON pathgen_write_receipts(route_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_write_receipts_schema
                    ON pathgen_write_receipts(schema_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_write_receipts_payload_digest
                    ON pathgen_write_receipts(payload_digest);
                CREATE INDEX IF NOT EXISTS idx_pathgen_write_receipts_receipt_digest
                    ON pathgen_write_receipts(receipt_digest);

                CREATE TABLE IF NOT EXISTS pathgen_schema_version (
                    schema_name TEXT PRIMARY KEY,
                    version INTEGER NOT NULL,
                    updated_at_unix_ms INTEGER NOT NULL,
                    last_migration_receipt_id TEXT
                );

                CREATE TABLE IF NOT EXISTS pathgen_schema_migration_receipts (
                    receipt_id TEXT PRIMARY KEY,
                    receipt_schema TEXT NOT NULL,
                    created_at_unix_ms INTEGER NOT NULL,
                    schema_name TEXT NOT NULL,
                    migration_id TEXT NOT NULL,
                    from_version INTEGER NOT NULL,
                    to_version INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    applied_by TEXT NOT NULL,
                    statement_digest TEXT NOT NULL,
                    previous_migration_receipt_digest TEXT,
                    schema_mutated INTEGER NOT NULL,
                    receipt_digest TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_pathgen_schema_migration_receipts_schema
                    ON pathgen_schema_migration_receipts(schema_name);
                CREATE INDEX IF NOT EXISTS idx_pathgen_schema_migration_receipts_migration
                    ON pathgen_schema_migration_receipts(migration_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_schema_migration_receipts_receipt_digest
                    ON pathgen_schema_migration_receipts(receipt_digest);

                CREATE TABLE IF NOT EXISTS pathgen_retention_receipts (
                    receipt_id TEXT PRIMARY KEY,
                    receipt_schema TEXT NOT NULL,
                    created_at_unix_ms INTEGER NOT NULL,
                    policy_id TEXT NOT NULL,
                    target_table TEXT NOT NULL,
                    filter_kind TEXT NOT NULL,
                    filter_value TEXT,
                    export_required INTEGER NOT NULL,
                    export_receipt_digest TEXT,
                    destructive_delete_requested INTEGER NOT NULL,
                    destructive_delete_executed INTEGER NOT NULL,
                    rows_matched INTEGER NOT NULL,
                    rows_eligible INTEGER NOT NULL,
                    rows_deleted INTEGER NOT NULL,
                    previous_retention_receipt_digest TEXT,
                    receipt_digest TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_pathgen_retention_receipts_policy
                    ON pathgen_retention_receipts(policy_id);
                CREATE INDEX IF NOT EXISTS idx_pathgen_retention_receipts_target
                    ON pathgen_retention_receipts(target_table);
                CREATE INDEX IF NOT EXISTS idx_pathgen_retention_receipts_receipt_digest
                    ON pathgen_retention_receipts(receipt_digest);
                "#,
            )
            .invalid_data()
    }

    /// Insert a pre-write output plan into the searchable catalog.
    pub fn insert_output_plan(&self, plan: &ArtifactOutputPlan) -> io::Result<i64> {
        self.conn
            .execute(
                r#"
                INSERT INTO pathgen_output_plans (
                    app_name,
                    env,
                    route_id,
                    schema_id,
                    kind,
                    version,
                    endpoint,
                    target_kind,
                    target_path,
                    bytes_len,
                    payload_digest,
                    policy_version,
                    op_word,
                    op_clean_addr
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                params![
                    plan.app_name.as_str(),
                    plan.env.to_string(),
                    plan.route_id.as_str(),
                    plan.schema.as_str(),
                    plan.kind.as_str(),
                    i64::from(plan.version),
                    plan.endpoint.to_string(),
                    plan.target.label(),
                    plan.target_path().display().to_string(),
                    usize_to_i64(plan.bytes_len)?,
                    plan.payload_digest.to_string(),
                    plan.policy_version.as_str(),
                    hex_word(plan.op_word),
                    hex_word(plan.op_clean_addr),
                ],
            )
            .invalid_data()?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Insert a hardened write receipt into the searchable catalog.
    pub fn insert_write_receipt(&self, receipt: &ArtifactWriteReceipt) -> io::Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO pathgen_write_receipts (
                    receipt_id,
                    created_at_unix_ms,
                    status,
                    app_name,
                    env,
                    route_id,
                    schema_id,
                    kind,
                    version,
                    endpoint,
                    path,
                    bytes_written,
                    payload_digest,
                    policy_version,
                    op_word,
                    previous_receipt_digest,
                    receipt_digest
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                "#,
                params![
                    receipt.receipt_id.as_str(),
                    u64_to_i64(receipt.created_at_unix_ms)?,
                    receipt.status.to_string(),
                    receipt.app_name.as_str(),
                    receipt.env.to_string(),
                    receipt.route_id.as_str(),
                    receipt.schema.as_str(),
                    receipt.kind.as_str(),
                    i64::from(receipt.version),
                    receipt.endpoint.to_string(),
                    receipt.path.display().to_string(),
                    usize_to_i64(receipt.bytes_written)?,
                    receipt.payload_digest.to_string(),
                    receipt.policy_version.as_str(),
                    hex_word(receipt.op_word),
                    receipt.previous_receipt_digest.as_ref().map(ToString::to_string),
                    receipt.receipt_digest.to_string(),
                ],
            )
            .invalid_data()?;

        Ok(())
    }

    pub fn count_output_plans(&self) -> io::Result<u64> {
        self.count_rows("pathgen_output_plans")
    }

    pub fn count_write_receipts(&self) -> io::Result<u64> {
        self.count_rows("pathgen_write_receipts")
    }

    pub fn count_migration_receipts(&self) -> io::Result<u64> {
        self.count_rows("pathgen_schema_migration_receipts")
    }

    pub fn count_retention_receipts(&self) -> io::Result<u64> {
        self.count_rows("pathgen_retention_receipts")
    }

    /// Initialize the schema and record a migration receipt.
    ///
    /// This keeps SQLite schema setup auditable without making SQLite the
    /// canonical artifact byte store. Re-running this method records an
    /// `already_current` receipt instead of mutating the schema again.
    pub fn init_schema_with_migration_receipt(
        &self,
        clock: &impl ArtifactWriteClock,
    ) -> io::Result<SqliteMigrationReceipt> {
        self.init_schema_with_named_migration_receipt(
            SQLITE_AUDIT_SCHEMA_MIGRATION_ID,
            SQLITE_AUDIT_SCHEMA_APPLIED_BY,
            clock,
        )
    }

    /// Initialize the schema and record a named migration receipt.
    pub fn init_schema_with_named_migration_receipt(
        &self,
        migration_id: &str,
        applied_by: &str,
        clock: &impl ArtifactWriteClock,
    ) -> io::Result<SqliteMigrationReceipt> {
        validate_sqlite_metadata("migration_id", migration_id)?;
        validate_sqlite_metadata("applied_by", applied_by)?;

        self.init_schema()?;

        let from_version = self.current_schema_version()?.unwrap_or(0);
        let previous_migration_receipt_digest = self
            .latest_migration_receipt()?
            .map(|row| parse_prefixed_digest(&row.receipt_digest));
        let status = if from_version == SQLITE_AUDIT_SCHEMA_VERSION {
            SqliteMigrationStatus::AlreadyCurrent
        } else {
            SqliteMigrationStatus::Applied
        };
        let schema_mutated = status == SqliteMigrationStatus::Applied;
        let created_at_unix_ms = clock
            .now_unix_ms()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        let receipt = SqliteMigrationReceipt::new(
            created_at_unix_ms,
            SQLITE_AUDIT_SCHEMA_NAME,
            migration_id,
            from_version,
            SQLITE_AUDIT_SCHEMA_VERSION,
            status,
            applied_by,
            sqlite_schema_statement_digest(),
            previous_migration_receipt_digest,
            schema_mutated,
        );

        self.insert_migration_receipt(&receipt)?;
        self.set_schema_version(
            SQLITE_AUDIT_SCHEMA_VERSION,
            created_at_unix_ms,
            &receipt.receipt_id,
        )?;

        Ok(receipt)
    }

    /// Return the recorded schema version for the pathgen SQLite audit schema.
    pub fn current_schema_version(&self) -> io::Result<Option<u32>> {
        if !self.table_exists("pathgen_schema_version")? {
            return Ok(None);
        }

        self.conn
            .query_row(
                r#"
                SELECT version
                FROM pathgen_schema_version
                WHERE schema_name = ?1
                "#,
                params![SQLITE_AUDIT_SCHEMA_NAME],
                |row| {
                    let version: i64 = row.get(0)?;
                    sqlite_i64_to_u32(version)
                },
            )
            .optional()
            .invalid_data()
    }

    /// Find a migration receipt by public receipt id.
    pub fn find_migration_receipt(
        &self,
        receipt_id: &str,
    ) -> io::Result<Option<SqliteMigrationReceiptRow>> {
        self.select_migration_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                schema_name,
                migration_id,
                from_version,
                to_version,
                status,
                applied_by,
                statement_digest,
                previous_migration_receipt_digest,
                schema_mutated,
                receipt_digest
            FROM pathgen_schema_migration_receipts
            WHERE receipt_id = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![receipt_id],
        )
        .map(|rows| rows.into_iter().next())
    }

    /// Return all migration receipts ordered by creation time and id.
    pub fn list_migration_receipts(&self) -> io::Result<Vec<SqliteMigrationReceiptRow>> {
        self.select_migration_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                schema_name,
                migration_id,
                from_version,
                to_version,
                status,
                applied_by,
                statement_digest,
                previous_migration_receipt_digest,
                schema_mutated,
                receipt_digest
            FROM pathgen_schema_migration_receipts
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            [],
        )
    }

    /// Return the latest migration receipt, if one has been recorded.
    pub fn latest_migration_receipt(&self) -> io::Result<Option<SqliteMigrationReceiptRow>> {
        if !self.table_exists("pathgen_schema_migration_receipts")? {
            return Ok(None);
        }

        self.select_migration_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                schema_name,
                migration_id,
                from_version,
                to_version,
                status,
                applied_by,
                statement_digest,
                previous_migration_receipt_digest,
                schema_mutated,
                receipt_digest
            FROM pathgen_schema_migration_receipts
            ORDER BY created_at_unix_ms DESC, receipt_id DESC
            LIMIT 1
            "#,
            [],
        )
        .map(|rows| rows.into_iter().next())
    }

    /// Export schema migration receipts as newline-delimited JSON.
    pub fn export_migration_receipts_jsonl<W>(&self, writer: W) -> io::Result<u64>
    where
        W: Write,
    {
        let rows = self.list_migration_receipts()?;
        write_migration_receipt_rows_jsonl(writer, &rows)?;
        Ok(rows.len() as u64)
    }

    /// Plan retention or compaction for indexed SQLite audit metadata.
    ///
    /// This records intent and row counts only. It never deletes rows.
    pub fn plan_retention(
        &self,
        policy: &SqliteRetentionPolicy,
        clock: &impl ArtifactWriteClock,
    ) -> io::Result<SqliteRetentionReceipt> {
        self.plan_retention_with_export_digest(policy, None, clock)
    }

    /// Plan retention after an export receipt has been produced.
    ///
    /// This is still metadata-only: it records that an export digest was supplied
    /// and keeps `destructive_delete_executed=false`.
    pub fn plan_retention_with_export_digest(
        &self,
        policy: &SqliteRetentionPolicy,
        export_receipt_digest: Option<PayloadDigest>,
        clock: &impl ArtifactWriteClock,
    ) -> io::Result<SqliteRetentionReceipt> {
        policy.validate()?;
        if policy.export_required && export_receipt_digest.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sqlite retention policy requires export receipt digest before planning retention",
            ));
        }
        if policy.destructive_delete_requested && !policy.export_required {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sqlite destructive retention requires export_required=true",
            ));
        }

        self.init_schema()?;

        let rows_matched = self.count_retention_matches(policy)?;
        let previous_retention_receipt_digest = self
            .latest_retention_receipt()?
            .map(|row| parse_prefixed_digest(&row.receipt_digest));
        let created_at_unix_ms = clock
            .now_unix_ms()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        let receipt = SqliteRetentionReceipt::new(
            created_at_unix_ms,
            policy,
            rows_matched,
            rows_matched,
            0,
            export_receipt_digest,
            previous_retention_receipt_digest,
        );

        self.insert_retention_receipt(&receipt)?;
        Ok(receipt)
    }

    /// Find a retention receipt by public receipt id.
    pub fn find_retention_receipt(
        &self,
        receipt_id: &str,
    ) -> io::Result<Option<SqliteRetentionReceiptRow>> {
        self.select_retention_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                policy_id,
                target_table,
                filter_kind,
                filter_value,
                export_required,
                export_receipt_digest,
                destructive_delete_requested,
                destructive_delete_executed,
                rows_matched,
                rows_eligible,
                rows_deleted,
                previous_retention_receipt_digest,
                receipt_digest
            FROM pathgen_retention_receipts
            WHERE receipt_id = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![receipt_id],
        )
        .map(|rows| rows.into_iter().next())
    }

    /// Return all retention receipts ordered by creation time and id.
    pub fn list_retention_receipts(&self) -> io::Result<Vec<SqliteRetentionReceiptRow>> {
        self.select_retention_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                policy_id,
                target_table,
                filter_kind,
                filter_value,
                export_required,
                export_receipt_digest,
                destructive_delete_requested,
                destructive_delete_executed,
                rows_matched,
                rows_eligible,
                rows_deleted,
                previous_retention_receipt_digest,
                receipt_digest
            FROM pathgen_retention_receipts
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            [],
        )
    }

    /// Return the latest retention receipt, if one has been recorded.
    pub fn latest_retention_receipt(&self) -> io::Result<Option<SqliteRetentionReceiptRow>> {
        if !self.table_exists("pathgen_retention_receipts")? {
            return Ok(None);
        }

        self.select_retention_receipts(
            r#"
            SELECT
                receipt_id,
                receipt_schema,
                created_at_unix_ms,
                policy_id,
                target_table,
                filter_kind,
                filter_value,
                export_required,
                export_receipt_digest,
                destructive_delete_requested,
                destructive_delete_executed,
                rows_matched,
                rows_eligible,
                rows_deleted,
                previous_retention_receipt_digest,
                receipt_digest
            FROM pathgen_retention_receipts
            ORDER BY created_at_unix_ms DESC, receipt_id DESC
            LIMIT 1
            "#,
            [],
        )
        .map(|rows| rows.into_iter().next())
    }

    /// Export retention planning receipts as newline-delimited JSON.
    pub fn export_retention_receipts_jsonl<W>(&self, writer: W) -> io::Result<u64>
    where
        W: Write,
    {
        let rows = self.list_retention_receipts()?;
        write_retention_receipt_rows_jsonl(writer, &rows)?;
        Ok(rows.len() as u64)
    }

    /// Find a compact receipt index row by public receipt id.
    pub fn find_write_receipt(&self, receipt_id: &str) -> io::Result<Option<SqliteWriteReceiptRow>> {
        self.conn
            .query_row(
                r#"
                SELECT
                    receipt_id,
                    route_id,
                    schema_id,
                    endpoint,
                    path,
                    payload_digest,
                    previous_receipt_digest,
                    receipt_digest
                FROM pathgen_write_receipts
                WHERE receipt_id = ?1
                "#,
                params![receipt_id],
                |row| {
                    Ok(SqliteWriteReceiptRow {
                        receipt_id: row.get(0)?,
                        route_id: row.get(1)?,
                        schema: row.get(2)?,
                        endpoint: row.get(3)?,
                        path: row.get(4)?,
                        payload_digest: row.get(5)?,
                        previous_receipt_digest: row.get(6)?,
                        receipt_digest: row.get(7)?,
                    })
                },
            )
            .optional()
            .invalid_data()
    }

    /// Find a compact receipt index row by receipt digest.
    pub fn find_write_receipt_by_digest(
        &self,
        receipt_digest: &str,
    ) -> io::Result<Option<SqliteWriteReceiptRow>> {
        self.select_write_receipts(
            r#"
            SELECT
                receipt_id,
                route_id,
                schema_id,
                endpoint,
                path,
                payload_digest,
                previous_receipt_digest,
                receipt_digest
            FROM pathgen_write_receipts
            WHERE receipt_digest = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![receipt_digest],
        )
        .map(|rows| rows.into_iter().next())
    }

    /// List output plans for a route id, ordered by insertion id.
    pub fn list_output_plans_by_route(
        &self,
        route_id: &str,
    ) -> io::Result<Vec<SqliteOutputPlanRow>> {
        self.select_output_plans(
            r#"
            SELECT
                id,
                app_name,
                env,
                route_id,
                schema_id,
                kind,
                version,
                endpoint,
                target_kind,
                target_path,
                bytes_len,
                payload_digest,
                policy_version,
                op_word,
                op_clean_addr
            FROM pathgen_output_plans
            WHERE route_id = ?1
            ORDER BY id
            "#,
            params![route_id],
        )
    }

    /// List output plans for a schema id, ordered by insertion id.
    pub fn list_output_plans_by_schema(
        &self,
        schema: &str,
    ) -> io::Result<Vec<SqliteOutputPlanRow>> {
        self.select_output_plans(
            r#"
            SELECT
                id,
                app_name,
                env,
                route_id,
                schema_id,
                kind,
                version,
                endpoint,
                target_kind,
                target_path,
                bytes_len,
                payload_digest,
                policy_version,
                op_word,
                op_clean_addr
            FROM pathgen_output_plans
            WHERE schema_id = ?1
            ORDER BY id
            "#,
            params![schema],
        )
    }

    /// List output plans for a payload digest, ordered by insertion id.
    pub fn list_output_plans_by_payload_digest(
        &self,
        payload_digest: &str,
    ) -> io::Result<Vec<SqliteOutputPlanRow>> {
        self.select_output_plans(
            r#"
            SELECT
                id,
                app_name,
                env,
                route_id,
                schema_id,
                kind,
                version,
                endpoint,
                target_kind,
                target_path,
                bytes_len,
                payload_digest,
                policy_version,
                op_word,
                op_clean_addr
            FROM pathgen_output_plans
            WHERE payload_digest = ?1
            ORDER BY id
            "#,
            params![payload_digest],
        )
    }

    /// List write receipts for a route id, ordered by creation time and id.
    pub fn list_write_receipts_by_route(
        &self,
        route_id: &str,
    ) -> io::Result<Vec<SqliteWriteReceiptRow>> {
        self.select_write_receipts(
            r#"
            SELECT
                receipt_id,
                route_id,
                schema_id,
                endpoint,
                path,
                payload_digest,
                previous_receipt_digest,
                receipt_digest
            FROM pathgen_write_receipts
            WHERE route_id = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![route_id],
        )
    }

    /// List write receipts for a schema id, ordered by creation time and id.
    pub fn list_write_receipts_by_schema(
        &self,
        schema: &str,
    ) -> io::Result<Vec<SqliteWriteReceiptRow>> {
        self.select_write_receipts(
            r#"
            SELECT
                receipt_id,
                route_id,
                schema_id,
                endpoint,
                path,
                payload_digest,
                previous_receipt_digest,
                receipt_digest
            FROM pathgen_write_receipts
            WHERE schema_id = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![schema],
        )
    }

    /// List write receipts for a payload digest, ordered by creation time and id.
    pub fn list_write_receipts_by_payload_digest(
        &self,
        payload_digest: &str,
    ) -> io::Result<Vec<SqliteWriteReceiptRow>> {
        self.select_write_receipts(
            r#"
            SELECT
                receipt_id,
                route_id,
                schema_id,
                endpoint,
                path,
                payload_digest,
                previous_receipt_digest,
                receipt_digest
            FROM pathgen_write_receipts
            WHERE payload_digest = ?1
            ORDER BY created_at_unix_ms, receipt_id
            "#,
            params![payload_digest],
        )
    }

    /// Export indexed output plans as newline-delimited JSON.
    ///
    /// This is an audit/export helper only. It does not read artifact bytes and
    /// does not become the canonical artifact store.
    pub fn export_output_plans_jsonl<W>(&self, writer: W) -> io::Result<u64>
    where
        W: Write,
    {
        let rows = self.select_all_output_plans()?;
        write_output_plan_rows_jsonl(writer, &rows)?;
        Ok(rows.len() as u64)
    }

    /// Export indexed write receipts as newline-delimited JSON.
    ///
    /// This is an audit/export helper only. It exports receipt metadata from the
    /// SQLite catalog; it does not rewrite receipt files or artifact payloads.
    pub fn export_write_receipts_jsonl<W>(&self, writer: W) -> io::Result<u64>
    where
        W: Write,
    {
        let rows = self.select_all_write_receipts()?;
        write_write_receipt_rows_jsonl(writer, &rows)?;
        Ok(rows.len() as u64)
    }

    fn select_all_output_plans(&self) -> io::Result<Vec<SqliteOutputPlanRow>> {
        self.select_output_plans(
            r#"
            SELECT
                id,
                app_name,
                env,
                route_id,
                schema_id,
                kind,
                version,
                endpoint,
                target_kind,
                target_path,
                bytes_len,
                payload_digest,
                policy_version,
                op_word,
                op_clean_addr
            FROM pathgen_output_plans
            ORDER BY id
            "#,
            [],
        )
    }

    fn select_all_write_receipts(&self) -> io::Result<Vec<SqliteWriteReceiptExportRow>> {
        let mut statement = self
            .conn
            .prepare(
                r#"
                SELECT
                    receipt_id,
                    created_at_unix_ms,
                    status,
                    app_name,
                    env,
                    route_id,
                    schema_id,
                    kind,
                    version,
                    endpoint,
                    path,
                    bytes_written,
                    payload_digest,
                    policy_version,
                    op_word,
                    previous_receipt_digest,
                    receipt_digest
                FROM pathgen_write_receipts
                ORDER BY created_at_unix_ms, receipt_id
                "#,
            )
            .invalid_data()?;

        let rows = statement
            .query_map([], sqlite_write_receipt_export_row)
            .invalid_data()?;

        collect_rows(rows)
    }

    fn select_output_plans<P>(
        &self,
        sql: &str,
        params: P,
    ) -> io::Result<Vec<SqliteOutputPlanRow>>
    where
        P: rusqlite::Params,
    {
        let mut statement = self.conn.prepare(sql).invalid_data()?;
        let rows = statement
            .query_map(params, sqlite_output_plan_row)
            .invalid_data()?;

        collect_rows(rows)
    }

    fn select_write_receipts<P>(
        &self,
        sql: &str,
        params: P,
    ) -> io::Result<Vec<SqliteWriteReceiptRow>>
    where
        P: rusqlite::Params,
    {
        let mut statement = self.conn.prepare(sql).invalid_data()?;
        let rows = statement
            .query_map(params, sqlite_write_receipt_row)
            .invalid_data()?;

        collect_rows(rows)
    }

    fn insert_migration_receipt(&self, receipt: &SqliteMigrationReceipt) -> io::Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO pathgen_schema_migration_receipts (
                    receipt_id,
                    receipt_schema,
                    created_at_unix_ms,
                    schema_name,
                    migration_id,
                    from_version,
                    to_version,
                    status,
                    applied_by,
                    statement_digest,
                    previous_migration_receipt_digest,
                    schema_mutated,
                    receipt_digest
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    receipt.receipt_id.as_str(),
                    receipt.receipt_schema,
                    u64_to_i64(receipt.created_at_unix_ms)?,
                    receipt.schema_name.as_str(),
                    receipt.migration_id.as_str(),
                    i64::from(receipt.from_version),
                    i64::from(receipt.to_version),
                    receipt.status.to_string(),
                    receipt.applied_by.as_str(),
                    receipt.statement_digest.to_string(),
                    receipt
                        .previous_migration_receipt_digest
                        .as_ref()
                        .map(ToString::to_string),
                    if receipt.schema_mutated { 1_i64 } else { 0_i64 },
                    receipt.receipt_digest.to_string(),
                ],
            )
            .invalid_data()?;

        Ok(())
    }

    fn set_schema_version(
        &self,
        version: u32,
        updated_at_unix_ms: u64,
        last_migration_receipt_id: &str,
    ) -> io::Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO pathgen_schema_version (
                    schema_name,
                    version,
                    updated_at_unix_ms,
                    last_migration_receipt_id
                ) VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(schema_name) DO UPDATE SET
                    version = excluded.version,
                    updated_at_unix_ms = excluded.updated_at_unix_ms,
                    last_migration_receipt_id = excluded.last_migration_receipt_id
                "#,
                params![
                    SQLITE_AUDIT_SCHEMA_NAME,
                    i64::from(version),
                    u64_to_i64(updated_at_unix_ms)?,
                    last_migration_receipt_id,
                ],
            )
            .invalid_data()?;

        Ok(())
    }

    fn select_migration_receipts<P>(
        &self,
        sql: &str,
        params: P,
    ) -> io::Result<Vec<SqliteMigrationReceiptRow>>
    where
        P: rusqlite::Params,
    {
        let mut statement = self.conn.prepare(sql).invalid_data()?;
        let rows = statement
            .query_map(params, sqlite_migration_receipt_row)
            .invalid_data()?;

        collect_rows(rows)
    }

    fn insert_retention_receipt(&self, receipt: &SqliteRetentionReceipt) -> io::Result<()> {
        self.conn
            .execute(
                r#"
                INSERT INTO pathgen_retention_receipts (
                    receipt_id,
                    receipt_schema,
                    created_at_unix_ms,
                    policy_id,
                    target_table,
                    filter_kind,
                    filter_value,
                    export_required,
                    export_receipt_digest,
                    destructive_delete_requested,
                    destructive_delete_executed,
                    rows_matched,
                    rows_eligible,
                    rows_deleted,
                    previous_retention_receipt_digest,
                    receipt_digest
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                "#,
                params![
                    receipt.receipt_id.as_str(),
                    receipt.receipt_schema,
                    u64_to_i64(receipt.created_at_unix_ms)?,
                    receipt.policy_id.as_str(),
                    receipt.target_table.label(),
                    receipt.filter.kind_label(),
                    receipt.filter.value(),
                    if receipt.export_required { 1_i64 } else { 0_i64 },
                    receipt.export_receipt_digest.as_ref().map(ToString::to_string),
                    if receipt.destructive_delete_requested { 1_i64 } else { 0_i64 },
                    if receipt.destructive_delete_executed { 1_i64 } else { 0_i64 },
                    u64_to_i64(receipt.rows_matched)?,
                    u64_to_i64(receipt.rows_eligible)?,
                    u64_to_i64(receipt.rows_deleted)?,
                    receipt
                        .previous_retention_receipt_digest
                        .as_ref()
                        .map(ToString::to_string),
                    receipt.receipt_digest.to_string(),
                ],
            )
            .invalid_data()?;

        Ok(())
    }

    fn select_retention_receipts<P>(
        &self,
        sql: &str,
        params: P,
    ) -> io::Result<Vec<SqliteRetentionReceiptRow>>
    where
        P: rusqlite::Params,
    {
        let mut statement = self.conn.prepare(sql).invalid_data()?;
        let rows = statement
            .query_map(params, sqlite_retention_receipt_row)
            .invalid_data()?;

        collect_rows(rows)
    }

    fn count_retention_matches(&self, policy: &SqliteRetentionPolicy) -> io::Result<u64> {
        let (sql, value) = policy.count_query()?;
        let count: i64 = match value {
            Some(value) => self
                .conn
                .query_row(sql, params![value], |row| row.get(0))
                .invalid_data()?,
            None => self.conn.query_row(sql, [], |row| row.get(0)).invalid_data()?,
        };
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative retention row count from sqlite: {count}"),
            ));
        }
        Ok(count as u64)
    }

    fn table_exists(&self, table: &str) -> io::Result<bool> {
        let exists: Option<i64> = self
            .conn
            .query_row(
                r#"
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = ?1
                LIMIT 1
                "#,
                params![table],
                |row| row.get(0),
            )
            .optional()
            .invalid_data()?;

        Ok(exists.is_some())
    }

    fn count_rows(&self, table: &'static str) -> io::Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        let count: i64 = self.conn.query_row(&sql, [], |row| row.get(0)).invalid_data()?;
        if count < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("negative row count from sqlite table {table}: {count}"),
            ));
        }
        Ok(count as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SqliteMigrationStatus {
    Applied,
    AlreadyCurrent,
}

impl SqliteMigrationStatus {
    pub const fn label(self) -> &'static str {
        match self {
            SqliteMigrationStatus::Applied => "applied",
            SqliteMigrationStatus::AlreadyCurrent => "already_current",
        }
    }
}

impl std::fmt::Display for SqliteMigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteMigrationReceipt {
    pub receipt_schema: &'static str,
    pub receipt_id: String,
    pub created_at_unix_ms: u64,
    pub schema_name: String,
    pub migration_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub status: SqliteMigrationStatus,
    pub applied_by: String,
    pub statement_digest: PayloadDigest,
    pub previous_migration_receipt_digest: Option<PayloadDigest>,
    pub schema_mutated: bool,
    pub receipt_digest: PayloadDigest,
}

impl SqliteMigrationReceipt {
    #[allow(clippy::too_many_arguments)]
    fn new(
        created_at_unix_ms: u64,
        schema_name: &str,
        migration_id: &str,
        from_version: u32,
        to_version: u32,
        status: SqliteMigrationStatus,
        applied_by: &str,
        statement_digest: PayloadDigest,
        previous_migration_receipt_digest: Option<PayloadDigest>,
        schema_mutated: bool,
    ) -> Self {
        let receipt_id = format!(
            "sqlite-migration-{migration_id}-{from_version}-{to_version}-{created_at_unix_ms}"
        );
        let receipt_digest = sqlite_migration_receipt_digest(
            &receipt_id,
            created_at_unix_ms,
            schema_name,
            migration_id,
            from_version,
            to_version,
            status,
            applied_by,
            &statement_digest,
            previous_migration_receipt_digest.as_ref(),
            schema_mutated,
        );

        Self {
            receipt_schema: SQLITE_MIGRATION_RECEIPT_SCHEMA,
            receipt_id,
            created_at_unix_ms,
            schema_name: schema_name.to_string(),
            migration_id: migration_id.to_string(),
            from_version,
            to_version,
            status,
            applied_by: applied_by.to_string(),
            statement_digest,
            previous_migration_receipt_digest,
            schema_mutated,
            receipt_digest,
        }
    }

    pub fn audit_line(&self) -> String {
        format!(
            "receipt_schema={} receipt_id={} schema_name={} migration_id={} from_version={} to_version={} status={} schema_mutated={} statement_digest={} previous_migration_receipt_digest={} receipt_digest={}",
            self.receipt_schema,
            self.receipt_id,
            self.schema_name,
            self.migration_id,
            self.from_version,
            self.to_version,
            self.status,
            self.schema_mutated,
            self.statement_digest,
            self.previous_migration_receipt_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            self.receipt_digest,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteMigrationReceiptRow {
    pub receipt_id: String,
    pub receipt_schema: String,
    pub created_at_unix_ms: u64,
    pub schema_name: String,
    pub migration_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub status: String,
    pub applied_by: String,
    pub statement_digest: String,
    pub previous_migration_receipt_digest: Option<String>,
    pub schema_mutated: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SqliteRetentionTarget {
    OutputPlans,
    WriteReceipts,
    MigrationReceipts,
}

impl SqliteRetentionTarget {
    pub const fn label(self) -> &'static str {
        match self {
            SqliteRetentionTarget::OutputPlans => "pathgen_output_plans",
            SqliteRetentionTarget::WriteReceipts => "pathgen_write_receipts",
            SqliteRetentionTarget::MigrationReceipts => "pathgen_schema_migration_receipts",
        }
    }
}

impl std::fmt::Display for SqliteRetentionTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SqliteRetentionFilter {
    All,
    RouteId(String),
    Schema(String),
    PayloadDigest(String),
    ReceiptDigest(String),
    OlderThanCreatedAtUnixMs(u64),
}

impl SqliteRetentionFilter {
    pub const fn kind_label(&self) -> &'static str {
        match self {
            SqliteRetentionFilter::All => "all",
            SqliteRetentionFilter::RouteId(_) => "route_id",
            SqliteRetentionFilter::Schema(_) => "schema",
            SqliteRetentionFilter::PayloadDigest(_) => "payload_digest",
            SqliteRetentionFilter::ReceiptDigest(_) => "receipt_digest",
            SqliteRetentionFilter::OlderThanCreatedAtUnixMs(_) => "older_than_created_at_unix_ms",
        }
    }

    pub fn value(&self) -> Option<String> {
        match self {
            SqliteRetentionFilter::All => None,
            SqliteRetentionFilter::RouteId(value)
            | SqliteRetentionFilter::Schema(value)
            | SqliteRetentionFilter::PayloadDigest(value)
            | SqliteRetentionFilter::ReceiptDigest(value) => Some(value.clone()),
            SqliteRetentionFilter::OlderThanCreatedAtUnixMs(value) => Some(value.to_string()),
        }
    }

    fn validate_for_target(&self, target: SqliteRetentionTarget) -> io::Result<()> {
        match self {
            SqliteRetentionFilter::All => Ok(()),
            SqliteRetentionFilter::RouteId(value) => {
                validate_sqlite_metadata("retention route_id", value)?;
                match target {
                    SqliteRetentionTarget::OutputPlans | SqliteRetentionTarget::WriteReceipts => Ok(()),
                    SqliteRetentionTarget::MigrationReceipts => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "route_id retention filter does not apply to sqlite migration receipts",
                    )),
                }
            }
            SqliteRetentionFilter::Schema(value) => {
                validate_sqlite_metadata("retention schema", value)?;
                Ok(())
            }
            SqliteRetentionFilter::PayloadDigest(value) => {
                validate_sqlite_metadata("retention payload_digest", value)?;
                match target {
                    SqliteRetentionTarget::OutputPlans | SqliteRetentionTarget::WriteReceipts => Ok(()),
                    SqliteRetentionTarget::MigrationReceipts => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "payload_digest retention filter does not apply to sqlite migration receipts",
                    )),
                }
            }
            SqliteRetentionFilter::ReceiptDigest(value) => {
                validate_sqlite_metadata("retention receipt_digest", value)?;
                match target {
                    SqliteRetentionTarget::WriteReceipts | SqliteRetentionTarget::MigrationReceipts => Ok(()),
                    SqliteRetentionTarget::OutputPlans => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "receipt_digest retention filter does not apply to sqlite output plans",
                    )),
                }
            }
            SqliteRetentionFilter::OlderThanCreatedAtUnixMs(_) => match target {
                SqliteRetentionTarget::WriteReceipts | SqliteRetentionTarget::MigrationReceipts => Ok(()),
                SqliteRetentionTarget::OutputPlans => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "created_at retention filter does not apply to sqlite output plans",
                )),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteRetentionPolicy {
    pub policy_id: String,
    pub target_table: SqliteRetentionTarget,
    pub filter: SqliteRetentionFilter,
    pub export_required: bool,
    pub destructive_delete_requested: bool,
}

impl SqliteRetentionPolicy {
    pub fn plan_only(policy_id: impl Into<String>, target_table: SqliteRetentionTarget) -> Self {
        Self {
            policy_id: policy_id.into(),
            target_table,
            filter: SqliteRetentionFilter::All,
            export_required: false,
            destructive_delete_requested: false,
        }
    }

    pub fn export_before_retention(
        policy_id: impl Into<String>,
        target_table: SqliteRetentionTarget,
    ) -> Self {
        Self {
            policy_id: policy_id.into(),
            target_table,
            filter: SqliteRetentionFilter::All,
            export_required: true,
            destructive_delete_requested: false,
        }
    }

    pub fn with_filter(mut self, filter: SqliteRetentionFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_export_required(mut self, export_required: bool) -> Self {
        self.export_required = export_required;
        self
    }

    pub fn request_destructive_delete(mut self) -> Self {
        self.destructive_delete_requested = true;
        self
    }

    pub fn validate(&self) -> io::Result<()> {
        validate_sqlite_metadata("retention policy_id", &self.policy_id)?;
        self.filter.validate_for_target(self.target_table)
    }

    fn count_query(&self) -> io::Result<(&'static str, Option<String>)> {
        self.validate()?;
        let query = match (self.target_table, &self.filter) {
            (SqliteRetentionTarget::OutputPlans, SqliteRetentionFilter::All) => {
                ("SELECT COUNT(*) FROM pathgen_output_plans", None)
            }
            (SqliteRetentionTarget::OutputPlans, SqliteRetentionFilter::RouteId(value)) => {
                ("SELECT COUNT(*) FROM pathgen_output_plans WHERE route_id = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::OutputPlans, SqliteRetentionFilter::Schema(value)) => {
                ("SELECT COUNT(*) FROM pathgen_output_plans WHERE schema_id = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::OutputPlans, SqliteRetentionFilter::PayloadDigest(value)) => {
                ("SELECT COUNT(*) FROM pathgen_output_plans WHERE payload_digest = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::All) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts", None)
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::RouteId(value)) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts WHERE route_id = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::Schema(value)) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts WHERE schema_id = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::PayloadDigest(value)) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts WHERE payload_digest = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::ReceiptDigest(value)) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts WHERE receipt_digest = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::WriteReceipts, SqliteRetentionFilter::OlderThanCreatedAtUnixMs(value)) => {
                ("SELECT COUNT(*) FROM pathgen_write_receipts WHERE created_at_unix_ms < ?1", Some(value.to_string()))
            }
            (SqliteRetentionTarget::MigrationReceipts, SqliteRetentionFilter::All) => {
                ("SELECT COUNT(*) FROM pathgen_schema_migration_receipts", None)
            }
            (SqliteRetentionTarget::MigrationReceipts, SqliteRetentionFilter::Schema(value)) => {
                ("SELECT COUNT(*) FROM pathgen_schema_migration_receipts WHERE schema_name = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::MigrationReceipts, SqliteRetentionFilter::ReceiptDigest(value)) => {
                ("SELECT COUNT(*) FROM pathgen_schema_migration_receipts WHERE receipt_digest = ?1", Some(value.clone()))
            }
            (SqliteRetentionTarget::MigrationReceipts, SqliteRetentionFilter::OlderThanCreatedAtUnixMs(value)) => {
                ("SELECT COUNT(*) FROM pathgen_schema_migration_receipts WHERE created_at_unix_ms < ?1", Some(value.to_string()))
            }
            _ => unreachable!("retention policy validation should reject incompatible filters"),
        };
        Ok(query)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteRetentionReceipt {
    pub receipt_schema: &'static str,
    pub receipt_id: String,
    pub created_at_unix_ms: u64,
    pub policy_id: String,
    pub target_table: SqliteRetentionTarget,
    pub filter: SqliteRetentionFilter,
    pub export_required: bool,
    pub export_receipt_digest: Option<PayloadDigest>,
    pub destructive_delete_requested: bool,
    pub destructive_delete_executed: bool,
    pub rows_matched: u64,
    pub rows_eligible: u64,
    pub rows_deleted: u64,
    pub previous_retention_receipt_digest: Option<PayloadDigest>,
    pub receipt_digest: PayloadDigest,
}

impl SqliteRetentionReceipt {
    #[allow(clippy::too_many_arguments)]
    fn new(
        created_at_unix_ms: u64,
        policy: &SqliteRetentionPolicy,
        rows_matched: u64,
        rows_eligible: u64,
        rows_deleted: u64,
        export_receipt_digest: Option<PayloadDigest>,
        previous_retention_receipt_digest: Option<PayloadDigest>,
    ) -> Self {
        let receipt_id = format!(
            "sqlite-retention-{}-{}-{}",
            policy.policy_id,
            policy.target_table.label(),
            created_at_unix_ms
        );
        let destructive_delete_executed = false;
        let receipt_digest = sqlite_retention_receipt_digest(
            &receipt_id,
            created_at_unix_ms,
            &policy.policy_id,
            policy.target_table,
            &policy.filter,
            policy.export_required,
            export_receipt_digest.as_ref(),
            policy.destructive_delete_requested,
            destructive_delete_executed,
            rows_matched,
            rows_eligible,
            rows_deleted,
            previous_retention_receipt_digest.as_ref(),
        );

        Self {
            receipt_schema: SQLITE_RETENTION_RECEIPT_SCHEMA,
            receipt_id,
            created_at_unix_ms,
            policy_id: policy.policy_id.clone(),
            target_table: policy.target_table,
            filter: policy.filter.clone(),
            export_required: policy.export_required,
            export_receipt_digest,
            destructive_delete_requested: policy.destructive_delete_requested,
            destructive_delete_executed,
            rows_matched,
            rows_eligible,
            rows_deleted,
            previous_retention_receipt_digest,
            receipt_digest,
        }
    }

    pub fn audit_line(&self) -> String {
        format!(
            "receipt_schema={} receipt_id={} policy_id={} target_table={} filter_kind={} filter_value={} export_required={} export_receipt_digest={} destructive_delete_requested={} destructive_delete_executed={} rows_matched={} rows_eligible={} rows_deleted={} previous_retention_receipt_digest={} receipt_digest={}",
            self.receipt_schema,
            self.receipt_id,
            self.policy_id,
            self.target_table,
            self.filter.kind_label(),
            self.filter.value().unwrap_or_else(|| "none".to_string()),
            self.export_required,
            self.export_receipt_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            self.destructive_delete_requested,
            self.destructive_delete_executed,
            self.rows_matched,
            self.rows_eligible,
            self.rows_deleted,
            self.previous_retention_receipt_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            self.receipt_digest,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteRetentionReceiptRow {
    pub receipt_id: String,
    pub receipt_schema: String,
    pub created_at_unix_ms: u64,
    pub policy_id: String,
    pub target_table: String,
    pub filter_kind: String,
    pub filter_value: Option<String>,
    pub export_required: bool,
    pub export_receipt_digest: Option<String>,
    pub destructive_delete_requested: bool,
    pub destructive_delete_executed: bool,
    pub rows_matched: u64,
    pub rows_eligible: u64,
    pub rows_deleted: u64,
    pub previous_retention_receipt_digest: Option<String>,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteOutputPlanRow {
    pub id: i64,
    pub app_name: String,
    pub env: String,
    pub route_id: String,
    pub schema: String,
    pub kind: String,
    pub version: u32,
    pub endpoint: String,
    pub target_kind: String,
    pub target_path: String,
    pub bytes_len: u64,
    pub payload_digest: String,
    pub policy_version: String,
    pub op_word: Option<String>,
    pub op_clean_addr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteWriteReceiptRow {
    pub receipt_id: String,
    pub route_id: String,
    pub schema: String,
    pub endpoint: String,
    pub path: String,
    pub payload_digest: String,
    pub previous_receipt_digest: Option<String>,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SqliteWriteReceiptExportRow {
    receipt_id: String,
    created_at_unix_ms: u64,
    status: String,
    app_name: String,
    env: String,
    route_id: String,
    schema: String,
    kind: String,
    version: u32,
    endpoint: String,
    path: String,
    bytes_written: u64,
    payload_digest: String,
    policy_version: String,
    op_word: Option<String>,
    previous_receipt_digest: Option<String>,
    receipt_digest: String,
}

fn sqlite_migration_receipt_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SqliteMigrationReceiptRow> {
    let created_at_unix_ms: i64 = row.get(2)?;
    let from_version: i64 = row.get(5)?;
    let to_version: i64 = row.get(6)?;
    let schema_mutated: i64 = row.get(11)?;

    Ok(SqliteMigrationReceiptRow {
        receipt_id: row.get(0)?,
        receipt_schema: row.get(1)?,
        created_at_unix_ms: sqlite_i64_to_u64(created_at_unix_ms)?,
        schema_name: row.get(3)?,
        migration_id: row.get(4)?,
        from_version: sqlite_i64_to_u32(from_version)?,
        to_version: sqlite_i64_to_u32(to_version)?,
        status: row.get(7)?,
        applied_by: row.get(8)?,
        statement_digest: row.get(9)?,
        previous_migration_receipt_digest: row.get(10)?,
        schema_mutated: schema_mutated != 0,
        receipt_digest: row.get(12)?,
    })
}

fn sqlite_retention_receipt_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SqliteRetentionReceiptRow> {
    let created_at_unix_ms: i64 = row.get(2)?;
    let export_required: i64 = row.get(7)?;
    let destructive_delete_requested: i64 = row.get(9)?;
    let destructive_delete_executed: i64 = row.get(10)?;
    let rows_matched: i64 = row.get(11)?;
    let rows_eligible: i64 = row.get(12)?;
    let rows_deleted: i64 = row.get(13)?;

    Ok(SqliteRetentionReceiptRow {
        receipt_id: row.get(0)?,
        receipt_schema: row.get(1)?,
        created_at_unix_ms: sqlite_i64_to_u64(created_at_unix_ms)?,
        policy_id: row.get(3)?,
        target_table: row.get(4)?,
        filter_kind: row.get(5)?,
        filter_value: row.get(6)?,
        export_required: export_required != 0,
        export_receipt_digest: row.get(8)?,
        destructive_delete_requested: destructive_delete_requested != 0,
        destructive_delete_executed: destructive_delete_executed != 0,
        rows_matched: sqlite_i64_to_u64(rows_matched)?,
        rows_eligible: sqlite_i64_to_u64(rows_eligible)?,
        rows_deleted: sqlite_i64_to_u64(rows_deleted)?,
        previous_retention_receipt_digest: row.get(14)?,
        receipt_digest: row.get(15)?,
    })
}

fn sqlite_output_plan_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SqliteOutputPlanRow> {
    let version: i64 = row.get(6)?;
    let bytes_len: i64 = row.get(10)?;

    Ok(SqliteOutputPlanRow {
        id: row.get(0)?,
        app_name: row.get(1)?,
        env: row.get(2)?,
        route_id: row.get(3)?,
        schema: row.get(4)?,
        kind: row.get(5)?,
        version: sqlite_i64_to_u32(version)?,
        endpoint: row.get(7)?,
        target_kind: row.get(8)?,
        target_path: row.get(9)?,
        bytes_len: sqlite_i64_to_u64(bytes_len)?,
        payload_digest: row.get(11)?,
        policy_version: row.get(12)?,
        op_word: row.get(13)?,
        op_clean_addr: row.get(14)?,
    })
}

fn sqlite_write_receipt_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SqliteWriteReceiptRow> {
    Ok(SqliteWriteReceiptRow {
        receipt_id: row.get(0)?,
        route_id: row.get(1)?,
        schema: row.get(2)?,
        endpoint: row.get(3)?,
        path: row.get(4)?,
        payload_digest: row.get(5)?,
        previous_receipt_digest: row.get(6)?,
        receipt_digest: row.get(7)?,
    })
}

fn sqlite_write_receipt_export_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SqliteWriteReceiptExportRow> {
    let created_at_unix_ms: i64 = row.get(1)?;
    let version: i64 = row.get(8)?;
    let bytes_written: i64 = row.get(11)?;

    Ok(SqliteWriteReceiptExportRow {
        receipt_id: row.get(0)?,
        created_at_unix_ms: sqlite_i64_to_u64(created_at_unix_ms)?,
        status: row.get(2)?,
        app_name: row.get(3)?,
        env: row.get(4)?,
        route_id: row.get(5)?,
        schema: row.get(6)?,
        kind: row.get(7)?,
        version: sqlite_i64_to_u32(version)?,
        endpoint: row.get(9)?,
        path: row.get(10)?,
        bytes_written: sqlite_i64_to_u64(bytes_written)?,
        payload_digest: row.get(12)?,
        policy_version: row.get(13)?,
        op_word: row.get(14)?,
        previous_receipt_digest: row.get(15)?,
        receipt_digest: row.get(16)?,
    })
}

fn collect_rows<T, I>(rows: I) -> io::Result<Vec<T>>
where
    I: IntoIterator<Item = rusqlite::Result<T>>,
{
    let mut collected = Vec::new();
    for row in rows {
        collected.push(row.invalid_data()?);
    }
    Ok(collected)
}

fn write_migration_receipt_rows_jsonl<W>(
    mut writer: W,
    rows: &[SqliteMigrationReceiptRow],
) -> io::Result<()>
where
    W: Write,
{
    for row in rows {
        let mut line = String::new();
        line.push('{');
        push_json_string_field(&mut line, "receipt_id", &row.receipt_id);
        push_json_string_field(&mut line, "receipt_schema", &row.receipt_schema);
        push_json_u64(&mut line, "created_at_unix_ms", row.created_at_unix_ms);
        push_json_string_field(&mut line, "schema_name", &row.schema_name);
        push_json_string_field(&mut line, "migration_id", &row.migration_id);
        push_json_u64(&mut line, "from_version", u64::from(row.from_version));
        push_json_u64(&mut line, "to_version", u64::from(row.to_version));
        push_json_string_field(&mut line, "status", &row.status);
        push_json_string_field(&mut line, "applied_by", &row.applied_by);
        push_json_string_field(&mut line, "statement_digest", &row.statement_digest);
        push_json_optional_string_field(
            &mut line,
            "previous_migration_receipt_digest",
            row.previous_migration_receipt_digest.as_deref(),
        );
        push_json_bool(&mut line, "schema_mutated", row.schema_mutated);
        push_json_string_field(&mut line, "receipt_digest", &row.receipt_digest);
        line.push_str("}\n");
        writer.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn write_retention_receipt_rows_jsonl<W>(
    mut writer: W,
    rows: &[SqliteRetentionReceiptRow],
) -> io::Result<()>
where
    W: Write,
{
    for row in rows {
        let mut line = String::new();
        line.push('{');
        push_json_string_field(&mut line, "receipt_id", &row.receipt_id);
        push_json_string_field(&mut line, "receipt_schema", &row.receipt_schema);
        push_json_u64(&mut line, "created_at_unix_ms", row.created_at_unix_ms);
        push_json_string_field(&mut line, "policy_id", &row.policy_id);
        push_json_string_field(&mut line, "target_table", &row.target_table);
        push_json_string_field(&mut line, "filter_kind", &row.filter_kind);
        push_json_optional_string_field(&mut line, "filter_value", row.filter_value.as_deref());
        push_json_bool(&mut line, "export_required", row.export_required);
        push_json_optional_string_field(
            &mut line,
            "export_receipt_digest",
            row.export_receipt_digest.as_deref(),
        );
        push_json_bool(
            &mut line,
            "destructive_delete_requested",
            row.destructive_delete_requested,
        );
        push_json_bool(
            &mut line,
            "destructive_delete_executed",
            row.destructive_delete_executed,
        );
        push_json_u64(&mut line, "rows_matched", row.rows_matched);
        push_json_u64(&mut line, "rows_eligible", row.rows_eligible);
        push_json_u64(&mut line, "rows_deleted", row.rows_deleted);
        push_json_optional_string_field(
            &mut line,
            "previous_retention_receipt_digest",
            row.previous_retention_receipt_digest.as_deref(),
        );
        push_json_string_field(&mut line, "receipt_digest", &row.receipt_digest);
        line.push_str("}\n");
        writer.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn write_output_plan_rows_jsonl<W>(mut writer: W, rows: &[SqliteOutputPlanRow]) -> io::Result<()>
where
    W: Write,
{
    for row in rows {
        let mut line = String::new();
        line.push('{');
        push_json_i64(&mut line, "id", row.id);
        push_json_string_field(&mut line, "app_name", &row.app_name);
        push_json_string_field(&mut line, "env", &row.env);
        push_json_string_field(&mut line, "route_id", &row.route_id);
        push_json_string_field(&mut line, "schema", &row.schema);
        push_json_string_field(&mut line, "kind", &row.kind);
        push_json_u64(&mut line, "version", u64::from(row.version));
        push_json_string_field(&mut line, "endpoint", &row.endpoint);
        push_json_string_field(&mut line, "target_kind", &row.target_kind);
        push_json_string_field(&mut line, "target_path", &row.target_path);
        push_json_u64(&mut line, "bytes_len", row.bytes_len);
        push_json_string_field(&mut line, "payload_digest", &row.payload_digest);
        push_json_string_field(&mut line, "policy_version", &row.policy_version);
        push_json_optional_string_field(&mut line, "op_word", row.op_word.as_deref());
        push_json_optional_string_field(&mut line, "op_clean_addr", row.op_clean_addr.as_deref());
        line.push_str("}\n");
        writer.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn write_write_receipt_rows_jsonl<W>(
    mut writer: W,
    rows: &[SqliteWriteReceiptExportRow],
) -> io::Result<()>
where
    W: Write,
{
    for row in rows {
        let mut line = String::new();
        line.push('{');
        push_json_string_field(&mut line, "receipt_id", &row.receipt_id);
        push_json_u64(&mut line, "created_at_unix_ms", row.created_at_unix_ms);
        push_json_string_field(&mut line, "status", &row.status);
        push_json_string_field(&mut line, "app_name", &row.app_name);
        push_json_string_field(&mut line, "env", &row.env);
        push_json_string_field(&mut line, "route_id", &row.route_id);
        push_json_string_field(&mut line, "schema", &row.schema);
        push_json_string_field(&mut line, "kind", &row.kind);
        push_json_u64(&mut line, "version", u64::from(row.version));
        push_json_string_field(&mut line, "endpoint", &row.endpoint);
        push_json_string_field(&mut line, "path", &row.path);
        push_json_u64(&mut line, "bytes_written", row.bytes_written);
        push_json_string_field(&mut line, "payload_digest", &row.payload_digest);
        push_json_string_field(&mut line, "policy_version", &row.policy_version);
        push_json_optional_string_field(&mut line, "op_word", row.op_word.as_deref());
        push_json_optional_string_field(
            &mut line,
            "previous_receipt_digest",
            row.previous_receipt_digest.as_deref(),
        );
        push_json_string_field(&mut line, "receipt_digest", &row.receipt_digest);
        line.push_str("}\n");
        writer.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn push_json_string_field(line: &mut String, name: &str, value: &str) {
    push_json_name(line, name);
    push_json_string(line, value);
}

fn push_json_optional_string_field(line: &mut String, name: &str, value: Option<&str>) {
    push_json_name(line, name);
    match value {
        Some(value) => push_json_string(line, value),
        None => line.push_str("null"),
    }
}

fn push_json_bool(line: &mut String, name: &str, value: bool) {
    push_json_name(line, name);
    line.push_str(if value { "true" } else { "false" });
}

fn push_json_i64(line: &mut String, name: &str, value: i64) {
    push_json_name(line, name);
    line.push_str(&value.to_string());
}

fn push_json_u64(line: &mut String, name: &str, value: u64) {
    push_json_name(line, name);
    line.push_str(&value.to_string());
}

fn push_json_name(line: &mut String, name: &str) {
    if !line.ends_with('{') {
        line.push(',');
    }
    push_json_string(line, name);
    line.push(':');
}

fn push_json_string(line: &mut String, value: &str) {
    line.push('"');
    for ch in value.chars() {
        match ch {
            '"' => line.push_str("\\\""),
            '\\' => line.push_str("\\\\"),
            '\n' => line.push_str("\\n"),
            '\r' => line.push_str("\\r"),
            '\t' => line.push_str("\\t"),
            ch if ch < ' ' => line.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => line.push(ch),
        }
    }
    line.push('"');
}

#[allow(clippy::too_many_arguments)]
fn sqlite_retention_receipt_digest(
    receipt_id: &str,
    created_at_unix_ms: u64,
    policy_id: &str,
    target_table: SqliteRetentionTarget,
    filter: &SqliteRetentionFilter,
    export_required: bool,
    export_receipt_digest: Option<&PayloadDigest>,
    destructive_delete_requested: bool,
    destructive_delete_executed: bool,
    rows_matched: u64,
    rows_eligible: u64,
    rows_deleted: u64,
    previous_retention_receipt_digest: Option<&PayloadDigest>,
) -> PayloadDigest {
    let preimage = format!(
        "schema={}|receipt_id={}|created_at_unix_ms={}|policy_id={}|target_table={}|filter_kind={}|filter_value={}|export_required={}|export_receipt_digest={}|destructive_delete_requested={}|destructive_delete_executed={}|rows_matched={}|rows_eligible={}|rows_deleted={}|previous={}",
        SQLITE_RETENTION_RECEIPT_SCHEMA,
        receipt_id,
        created_at_unix_ms,
        policy_id,
        target_table,
        filter.kind_label(),
        filter.value().unwrap_or_else(|| "none".to_string()),
        export_required,
        export_receipt_digest
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string()),
        destructive_delete_requested,
        destructive_delete_executed,
        rows_matched,
        rows_eligible,
        rows_deleted,
        previous_retention_receipt_digest
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string()),
    );
    PayloadDigest::sha256(preimage.as_bytes())
}

fn sqlite_schema_statement_digest() -> PayloadDigest {
    PayloadDigest::sha256(
        format!(
            "schema_name={} version={} tables=pathgen_output_plans,pathgen_write_receipts,pathgen_schema_version,pathgen_schema_migration_receipts,pathgen_retention_receipts",
            SQLITE_AUDIT_SCHEMA_NAME,
            SQLITE_AUDIT_SCHEMA_VERSION,
        )
        .as_bytes(),
    )
}

#[allow(clippy::too_many_arguments)]
fn sqlite_migration_receipt_digest(
    receipt_id: &str,
    created_at_unix_ms: u64,
    schema_name: &str,
    migration_id: &str,
    from_version: u32,
    to_version: u32,
    status: SqliteMigrationStatus,
    applied_by: &str,
    statement_digest: &PayloadDigest,
    previous_migration_receipt_digest: Option<&PayloadDigest>,
    schema_mutated: bool,
) -> PayloadDigest {
    let preimage = format!(
        "schema={}|receipt_id={}|created_at_unix_ms={}|schema_name={}|migration_id={}|from_version={}|to_version={}|status={}|applied_by={}|statement_digest={}|previous={}|schema_mutated={}",
        SQLITE_MIGRATION_RECEIPT_SCHEMA,
        receipt_id,
        created_at_unix_ms,
        schema_name,
        migration_id,
        from_version,
        to_version,
        status,
        applied_by,
        statement_digest,
        previous_migration_receipt_digest
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string()),
        schema_mutated,
    );
    PayloadDigest::sha256(preimage.as_bytes())
}

fn parse_prefixed_digest(value: &str) -> PayloadDigest {
    let digest = value
        .strip_prefix("sha256:")
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string());

    PayloadDigest {
        algorithm: "sha256",
        value: digest,
    }
}

fn validate_sqlite_metadata(field: &'static str, value: &str) -> io::Result<()> {
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("sqlite migration {field} cannot be empty"),
        ));
    }
    if value.trim() != value {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("sqlite migration {field} cannot have surrounding whitespace"),
        ));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("sqlite migration {field} cannot contain control characters"),
        ));
    }
    Ok(())
}

fn sqlite_i64_to_u32(value: i64) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|_| {
        rusqlite::Error::IntegralValueOutOfRange(std::mem::size_of::<u32>(), value)
    })
}

fn sqlite_i64_to_u64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| {
        rusqlite::Error::IntegralValueOutOfRange(std::mem::size_of::<u64>(), value)
    })
}

fn hex_word(value: Option<usize>) -> Option<String> {
    value.map(|word| format!("{word:#x}"))
}

fn usize_to_i64(value: usize) -> io::Result<i64> {
    i64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("usize value does not fit into sqlite INTEGER: {value}"),
        )
    })
}

fn u64_to_i64(value: u64) -> io::Result<i64> {
    i64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("u64 value does not fit into sqlite INTEGER: {value}"),
        )
    })
}
