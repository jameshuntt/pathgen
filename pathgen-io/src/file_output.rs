use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use pathgen_core::{ArtifactEndpoint, Env, PathMutability, PathgenError, ResolvedArtifactOutput};

use crate::{ArtifactOutputPlan, PayloadDigest};

/// Clock used when creating artifact write receipts.
///
/// The default [`SystemArtifactWriteClock`] records wall-clock Unix time in
/// milliseconds. Tests and deterministic runners can inject their own clock so
/// receipt IDs/digests are reproducible.
pub trait ArtifactWriteClock {
    fn now_unix_ms(&self) -> Result<u64, PathgenError>;
}

/// System wall-clock implementation for artifact write receipts.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemArtifactWriteClock;

impl ArtifactWriteClock for SystemArtifactWriteClock {
    fn now_unix_ms(&self) -> Result<u64, PathgenError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| {
                PathgenError::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("system clock is before Unix epoch: {err}"),
                ))
            })?;

        Ok(duration.as_millis() as u64)
    }
}

/// Deterministic clock for tests, dry-runs, and reproducible receipt fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedArtifactWriteClock {
    unix_ms: u64,
}

impl FixedArtifactWriteClock {
    pub const fn new(unix_ms: u64) -> Self {
        Self { unix_ms }
    }

    pub const fn unix_ms(self) -> u64 {
        self.unix_ms
    }
}

impl ArtifactWriteClock for FixedArtifactWriteClock {
    fn now_unix_ms(&self) -> Result<u64, PathgenError> {
        Ok(self.unix_ms)
    }
}

/// Outcome recorded by an artifact write receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactWriteStatus {
    Written,
}

impl ArtifactWriteStatus {
    pub const fn label(self) -> &'static str {
        match self {
            ArtifactWriteStatus::Written => "written",
        }
    }
}

impl std::fmt::Display for ArtifactWriteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// File-write execution strategy recorded by a write receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFileWriteStrategy {
    /// Historical direct write path using `fs::write` semantics.
    Direct,
    /// Write a sibling temp file, then rename it into place.
    AtomicTempRename,
}

impl ArtifactFileWriteStrategy {
    pub const fn label(self) -> &'static str {
        match self {
            ArtifactFileWriteStrategy::Direct => "direct",
            ArtifactFileWriteStrategy::AtomicTempRename => "atomic_temp_rename",
        }
    }
}

impl std::fmt::Display for ArtifactFileWriteStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Target-conflict behavior for a file write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFileConflictPolicy {
    /// Existing targets are rejected before write execution.
    RejectExisting,
    /// Existing targets may be replaced when the resolved artifact mutability allows it.
    AllowOverwrite,
}

impl ArtifactFileConflictPolicy {
    pub const fn label(self) -> &'static str {
        match self {
            ArtifactFileConflictPolicy::RejectExisting => "reject_existing",
            ArtifactFileConflictPolicy::AllowOverwrite => "allow_overwrite",
        }
    }
}

impl std::fmt::Display for ArtifactFileConflictPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Best-effort local lock behavior for a file write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFileLockPolicy {
    /// No lock file is created.
    NoLock,
    /// A sibling sidecar lock file is created with `create_new` and removed on drop.
    SidecarLock,
}

impl ArtifactFileLockPolicy {
    pub const fn label(self) -> &'static str {
        match self {
            ArtifactFileLockPolicy::NoLock => "no_lock",
            ArtifactFileLockPolicy::SidecarLock => "sidecar_lock",
        }
    }
}

impl std::fmt::Display for ArtifactFileLockPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Local file write policy used to harden actual file writes.
///
/// This policy remains filesystem-local. It does not introduce a daemon, a
/// global lock manager, or distributed locking. `SidecarLock` is an explicit
/// best-effort local coordination mechanism based on `create_new`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFileWritePolicy {
    pub strategy: ArtifactFileWriteStrategy,
    pub conflict_policy: ArtifactFileConflictPolicy,
    pub lock_policy: ArtifactFileLockPolicy,
    pub temp_suffix: String,
    pub lock_suffix: String,
}

impl ArtifactFileWritePolicy {
    pub fn direct_overwrite() -> Self {
        Self {
            strategy: ArtifactFileWriteStrategy::Direct,
            conflict_policy: ArtifactFileConflictPolicy::AllowOverwrite,
            lock_policy: ArtifactFileLockPolicy::NoLock,
            temp_suffix: ".pathgen.tmp".to_string(),
            lock_suffix: ".pathgen.lock".to_string(),
        }
    }

    pub fn atomic_temp_rename() -> Self {
        Self {
            strategy: ArtifactFileWriteStrategy::AtomicTempRename,
            conflict_policy: ArtifactFileConflictPolicy::RejectExisting,
            lock_policy: ArtifactFileLockPolicy::SidecarLock,
            temp_suffix: ".pathgen.tmp".to_string(),
            lock_suffix: ".pathgen.lock".to_string(),
        }
    }

    pub fn reject_existing(mut self) -> Self {
        self.conflict_policy = ArtifactFileConflictPolicy::RejectExisting;
        self
    }

    pub fn allow_overwrite(mut self) -> Self {
        self.conflict_policy = ArtifactFileConflictPolicy::AllowOverwrite;
        self
    }

    pub fn with_sidecar_lock(mut self) -> Self {
        self.lock_policy = ArtifactFileLockPolicy::SidecarLock;
        self
    }

    pub fn without_lock(mut self) -> Self {
        self.lock_policy = ArtifactFileLockPolicy::NoLock;
        self
    }

    pub fn with_temp_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.temp_suffix = suffix.into();
        self
    }

    pub fn with_lock_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.lock_suffix = suffix.into();
        self
    }

    pub fn validate(&self) -> Result<(), PathgenError> {
        validate_suffix("temp_suffix", &self.temp_suffix)?;
        validate_suffix("lock_suffix", &self.lock_suffix)?;

        if self.temp_suffix == self.lock_suffix {
            return Err(PathgenError::InvalidArtifactFileWritePolicy {
                field: "suffix",
                reason: "temp suffix and lock suffix must be different",
            });
        }

        Ok(())
    }

    pub fn temp_path_for(&self, target: &Path) -> PathBuf {
        sibling_with_suffix(target, &self.temp_suffix)
    }

    pub fn lock_path_for(&self, target: &Path) -> PathBuf {
        sibling_with_suffix(target, &self.lock_suffix)
    }

    pub fn audit_line(&self) -> String {
        format!(
            "strategy={} conflict_policy={} lock_policy={} temp_suffix={} lock_suffix={}",
            self.strategy,
            self.conflict_policy,
            self.lock_policy,
            self.temp_suffix,
            self.lock_suffix,
        )
    }
}

fn validate_suffix(field: &'static str, suffix: &str) -> Result<(), PathgenError> {
    if suffix.is_empty() {
        return Err(PathgenError::InvalidArtifactFileWritePolicy {
            field,
            reason: "suffix cannot be empty",
        });
    }

    if suffix.contains('/') || suffix.contains('\\') {
        return Err(PathgenError::InvalidArtifactFileWritePolicy {
            field,
            reason: "suffix cannot contain path separators",
        });
    }

    if suffix == "." || suffix == ".." || suffix.contains("..") {
        return Err(PathgenError::InvalidArtifactFileWritePolicy {
            field,
            reason: "suffix cannot contain parent traversal",
        });
    }

    if suffix.chars().any(char::is_control) {
        return Err(PathgenError::InvalidArtifactFileWritePolicy {
            field,
            reason: "suffix cannot contain control characters",
        });
    }

    Ok(())
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    file_name.push(suffix);
    path.with_file_name(file_name)
}

/// Buildable file write operation for an already-serialized payload.
///
/// Pass 03 made the output plan explicit. Pass 04 hardens write receipts:
/// every write now records a receipt id, timestamp, status, optional previous
/// receipt digest, and a digest over the receipt evidence itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildableFileOutput {
    pub resolved: ResolvedArtifactOutput,
    pub payload: Vec<u8>,
    pub plan: ArtifactOutputPlan,
}

impl BuildableFileOutput {
    pub fn new(
        resolved: ResolvedArtifactOutput,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, PathgenError> {
        let payload = payload.into();
        let plan = ArtifactOutputPlan::file(&resolved, &payload)?;
        Self::from_plan_and_payload(resolved, plan, payload)
    }

    /// Build a file output from a precomputed plan and the exact payload bytes
    /// that produced the plan digest.
    pub fn from_plan(
        resolved: ResolvedArtifactOutput,
        plan: ArtifactOutputPlan,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, PathgenError> {
        Self::from_plan_and_payload(resolved, plan, payload.into())
    }

    fn from_plan_and_payload(
        resolved: ResolvedArtifactOutput,
        plan: ArtifactOutputPlan,
        payload: Vec<u8>,
    ) -> Result<Self, PathgenError> {
        if !resolved.endpoint.is_file() {
            return Err(PathgenError::UnsupportedArtifactEndpoint {
                endpoint: resolved.endpoint.label(),
                reason: "buildable file output requires a file::* endpoint",
            });
        }

        if plan.endpoint != resolved.endpoint {
            return Err(PathgenError::ArtifactOutputPlanMismatch {
                field: "endpoint",
                expected: resolved.endpoint.label().to_string(),
                actual: plan.endpoint.label().to_string(),
            });
        }

        if plan.target_path() != resolved.path.as_path() {
            return Err(PathgenError::ArtifactOutputPlanMismatch {
                field: "path",
                expected: resolved.path.display().to_string(),
                actual: plan.target_path().display().to_string(),
            });
        }

        if plan.bytes_len != payload.len() {
            return Err(PathgenError::ArtifactOutputPlanMismatch {
                field: "bytes_len",
                expected: plan.bytes_len.to_string(),
                actual: payload.len().to_string(),
            });
        }

        if plan.payload_digest != PayloadDigest::sha256(&payload) {
            return Err(PathgenError::ArtifactOutputPlanMismatch {
                field: "payload_digest",
                expected: plan.payload_digest.to_string(),
                actual: PayloadDigest::sha256(&payload).to_string(),
            });
        }

        Ok(Self {
            resolved,
            payload,
            plan,
        })
    }

    pub fn byte_len(&self) -> usize {
        self.payload.len()
    }

    pub fn output_plan(&self) -> &ArtifactOutputPlan {
        &self.plan
    }

    /// Write the planned file output and return an auditable write receipt.
    pub fn write(self) -> Result<ArtifactWriteReceipt, PathgenError> {
        self.write_with_clock(&SystemArtifactWriteClock)
    }

    /// Write with an injected clock and no previous receipt link.
    pub fn write_with_clock(
        self,
        clock: &impl ArtifactWriteClock,
    ) -> Result<ArtifactWriteReceipt, PathgenError> {
        self.write_with_clock_and_previous(clock, None)
    }

    /// Write with an injected clock and an optional previous receipt digest.
    ///
    /// The previous digest gives callers a small receipt-chain primitive without
    /// forcing any particular append log or database layout into this file sink.
    pub fn write_with_clock_and_previous(
        self,
        clock: &impl ArtifactWriteClock,
        previous_receipt_digest: Option<PayloadDigest>,
    ) -> Result<ArtifactWriteReceipt, PathgenError> {
        self.write_with_policy_clock_and_previous(
            ArtifactFileWritePolicy::direct_overwrite(),
            clock,
            previous_receipt_digest,
        )
    }

    /// Write with an explicit local file policy and the system clock.
    pub fn write_with_policy(
        self,
        policy: ArtifactFileWritePolicy,
    ) -> Result<ArtifactWriteReceipt, PathgenError> {
        self.write_with_policy_and_clock(policy, &SystemArtifactWriteClock)
    }

    /// Write with an explicit local file policy and injected clock.
    pub fn write_with_policy_and_clock(
        self,
        policy: ArtifactFileWritePolicy,
        clock: &impl ArtifactWriteClock,
    ) -> Result<ArtifactWriteReceipt, PathgenError> {
        self.write_with_policy_clock_and_previous(policy, clock, None)
    }

    /// Write with an explicit local file policy, injected clock, and optional previous receipt.
    pub fn write_with_policy_clock_and_previous(
        self,
        policy: ArtifactFileWritePolicy,
        clock: &impl ArtifactWriteClock,
        previous_receipt_digest: Option<PayloadDigest>,
    ) -> Result<ArtifactWriteReceipt, PathgenError> {
        policy.validate()?;
        validate_policy_allowed_for_resolved(&self.resolved, &policy)?;

        let parent = self
            .resolved
            .path
            .parent()
            .ok_or_else(|| PathgenError::MissingParent {
                path: self.resolved.path.clone(),
            })?;

        fs::create_dir_all(parent)?;

        let target_path = self.resolved.path.clone();
        let temp_path = (policy.strategy == ArtifactFileWriteStrategy::AtomicTempRename)
            .then(|| policy.temp_path_for(&target_path));
        let lock_path = (policy.lock_policy == ArtifactFileLockPolicy::SidecarLock)
            .then(|| policy.lock_path_for(&target_path));

        let _lock_guard = match lock_path.as_ref() {
            Some(path) => Some(SidecarLockGuard::acquire(path)?),
            None => None,
        };

        if target_path.exists() && policy.conflict_policy == ArtifactFileConflictPolicy::RejectExisting {
            return Err(PathgenError::InvalidArtifactFileWritePolicy {
                field: "target_path",
                reason: "target exists and conflict policy rejects existing files",
            });
        }

        let existing_target_replaced = target_path.exists();

        match policy.strategy {
            ArtifactFileWriteStrategy::Direct => {
                if policy.conflict_policy == ArtifactFileConflictPolicy::RejectExisting {
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&target_path)?;
                    file.write_all(&self.payload)?;
                } else {
                    fs::write(&target_path, &self.payload)?;
                }
            }
            ArtifactFileWriteStrategy::AtomicTempRename => {
                let temp = temp_path.as_ref().expect("temp path exists for atomic strategy");
                if temp.exists() {
                    return Err(PathgenError::InvalidArtifactFileWritePolicy {
                        field: "temp_path",
                        reason: "temp path already exists",
                    });
                }

                {
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(temp)?;
                    file.write_all(&self.payload)?;
                    file.sync_all()?;
                }

                fs::rename(temp, &target_path)?;
            }
        }

        let created_at_unix_ms = clock.now_unix_ms()?;
        let bytes_written = self.payload.len();

        Ok(ArtifactWriteReceipt::from_written_file(
            self.resolved,
            self.plan,
            bytes_written,
            created_at_unix_ms,
            previous_receipt_digest,
            policy,
            temp_path,
            lock_path,
            existing_target_replaced,
        ))
    }
}

fn validate_policy_allowed_for_resolved(
    resolved: &ResolvedArtifactOutput,
    _policy: &ArtifactFileWritePolicy,
) -> Result<(), PathgenError> {
    if resolved.mutability == PathMutability::ReadOnly {
        return Err(PathgenError::InvalidArtifactFileWritePolicy {
            field: "mutability",
            reason: "read-only artifact outputs cannot be written",
        });
    }

    Ok(())
}

struct SidecarLockGuard {
    path: PathBuf,
}

impl SidecarLockGuard {
    fn acquire(path: &Path) -> Result<Self, PathgenError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(b"pathgen-sidecar-lock\n")?;
        file.sync_all()?;
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for SidecarLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactWriteReceipt {
    pub receipt_id: String,
    pub created_at_unix_ms: u64,
    pub status: ArtifactWriteStatus,
    pub app_name: String,
    pub env: Env,
    pub route_id: String,
    pub schema: String,
    pub kind: String,
    pub version: u32,
    pub endpoint: ArtifactEndpoint,
    pub path: PathBuf,
    pub bytes_written: usize,
    pub payload_digest: PayloadDigest,
    pub policy_version: String,
    pub op_word: Option<usize>,
    pub previous_receipt_digest: Option<PayloadDigest>,
    pub write_strategy: ArtifactFileWriteStrategy,
    pub conflict_policy: ArtifactFileConflictPolicy,
    pub lock_policy: ArtifactFileLockPolicy,
    pub temp_path: Option<PathBuf>,
    pub lock_path: Option<PathBuf>,
    pub atomic_rename_used: bool,
    pub existing_target_replaced: bool,
    pub receipt_digest: PayloadDigest,
}

impl ArtifactWriteReceipt {
    #[allow(clippy::too_many_arguments)]
    fn from_written_file(
        resolved: ResolvedArtifactOutput,
        plan: ArtifactOutputPlan,
        bytes_written: usize,
        created_at_unix_ms: u64,
        previous_receipt_digest: Option<PayloadDigest>,
        write_policy: ArtifactFileWritePolicy,
        temp_path: Option<PathBuf>,
        lock_path: Option<PathBuf>,
        existing_target_replaced: bool,
    ) -> Self {
        let status = ArtifactWriteStatus::Written;
        let atomic_rename_used = write_policy.strategy == ArtifactFileWriteStrategy::AtomicTempRename;
        let receipt_digest = receipt_digest_from_fields(ReceiptDigestFields {
            created_at_unix_ms,
            status,
            app_name: &resolved.app_name,
            env: resolved.env,
            route_id: &resolved.route_id,
            schema: &resolved.schema,
            kind: &resolved.kind,
            version: resolved.version,
            endpoint: resolved.endpoint,
            path: &resolved.path,
            bytes_written,
            payload_digest: &plan.payload_digest,
            policy_version: &resolved.policy_version,
            op_word: plan.op_word,
            previous_receipt_digest: previous_receipt_digest.as_ref(),
            write_strategy: write_policy.strategy,
            conflict_policy: write_policy.conflict_policy,
            lock_policy: write_policy.lock_policy,
            temp_path: temp_path.as_deref(),
            lock_path: lock_path.as_deref(),
            atomic_rename_used,
            existing_target_replaced,
        });
        let receipt_id = receipt_id_from_digest(&receipt_digest);

        Self {
            receipt_id,
            created_at_unix_ms,
            status,
            app_name: resolved.app_name,
            env: resolved.env,
            route_id: resolved.route_id,
            schema: resolved.schema,
            kind: resolved.kind,
            version: resolved.version,
            endpoint: resolved.endpoint,
            path: resolved.path,
            bytes_written,
            payload_digest: plan.payload_digest,
            policy_version: resolved.policy_version,
            op_word: plan.op_word,
            previous_receipt_digest,
            write_strategy: write_policy.strategy,
            conflict_policy: write_policy.conflict_policy,
            lock_policy: write_policy.lock_policy,
            temp_path,
            lock_path,
            atomic_rename_used,
            existing_target_replaced,
            receipt_digest,
        }
    }

    pub fn audit_line(&self) -> String {
        format!(
            "receipt_id={} created_at_unix_ms={} status={} policy={} app={} env={} route={} schema={} kind={} version={} endpoint={} path={} bytes_written={} payload_digest={} previous_receipt_digest={} receipt_digest={} op_word={} write_strategy={} conflict_policy={} lock_policy={} temp_path={} lock_path={} atomic_rename_used={} existing_target_replaced={}",
            self.receipt_id,
            self.created_at_unix_ms,
            self.status,
            self.policy_version,
            self.app_name,
            self.env,
            self.route_id,
            self.schema,
            self.kind,
            self.version,
            self.endpoint,
            self.path.display(),
            self.bytes_written,
            self.payload_digest,
            self.previous_receipt_digest
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            self.receipt_digest,
            self.op_word
                .map(|word| format!("{word:#x}"))
                .unwrap_or_else(|| "none".to_string()),
            self.write_strategy,
            self.conflict_policy,
            self.lock_policy,
            self.temp_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.lock_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.atomic_rename_used,
            self.existing_target_replaced,
        )
    }
}

struct ReceiptDigestFields<'a> {
    created_at_unix_ms: u64,
    status: ArtifactWriteStatus,
    app_name: &'a str,
    env: Env,
    route_id: &'a str,
    schema: &'a str,
    kind: &'a str,
    version: u32,
    endpoint: ArtifactEndpoint,
    path: &'a std::path::Path,
    bytes_written: usize,
    payload_digest: &'a PayloadDigest,
    policy_version: &'a str,
    op_word: Option<usize>,
    previous_receipt_digest: Option<&'a PayloadDigest>,
    write_strategy: ArtifactFileWriteStrategy,
    conflict_policy: ArtifactFileConflictPolicy,
    lock_policy: ArtifactFileLockPolicy,
    temp_path: Option<&'a std::path::Path>,
    lock_path: Option<&'a std::path::Path>,
    atomic_rename_used: bool,
    existing_target_replaced: bool,
}

fn receipt_digest_from_fields(fields: ReceiptDigestFields<'_>) -> PayloadDigest {
    let previous = fields
        .previous_receipt_digest
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());
    let op_word = fields
        .op_word
        .map(|word| format!("{word:#x}"))
        .unwrap_or_else(|| "none".to_string());
    let temp_path = fields
        .temp_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());
    let lock_path = fields
        .lock_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string());

    let preimage = format!(
        "artifact_write_receipt.v2\ncreated_at_unix_ms={}\nstatus={}\npolicy={}\napp={}\nenv={}\nroute={}\nschema={}\nkind={}\nversion={}\nendpoint={}\npath={}\nbytes_written={}\npayload_digest={}\nprevious_receipt_digest={}\nop_word={}\nwrite_strategy={}\nconflict_policy={}\nlock_policy={}\ntemp_path={}\nlock_path={}\natomic_rename_used={}\nexisting_target_replaced={}\n",
        fields.created_at_unix_ms,
        fields.status,
        fields.policy_version,
        fields.app_name,
        fields.env,
        fields.route_id,
        fields.schema,
        fields.kind,
        fields.version,
        fields.endpoint,
        fields.path.display(),
        fields.bytes_written,
        fields.payload_digest,
        previous,
        op_word,
        fields.write_strategy,
        fields.conflict_policy,
        fields.lock_policy,
        temp_path,
        lock_path,
        fields.atomic_rename_used,
        fields.existing_target_replaced,
    );

    PayloadDigest::sha256(preimage.as_bytes())
}

fn receipt_id_from_digest(digest: &PayloadDigest) -> String {
    let prefix_len = digest.value.len().min(16);
    format!("awr_{}", &digest.value[..prefix_len])
}
