use super::{PersistenceError, SqliteStore, decode, encode};
use execution::ProcessResult;
use fs2::FileExt;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::min,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const REDACTION_MARKER: &[u8] = b"[REDACTED]";
const ARTIFACT_COLUMNS: &str = "id, changeset_id, run_id, kind, created_at_unix_ms, body, status, stored_bytes, updated_at_unix_ms, expires_at_unix_ms";
const DEFAULT_SEGMENT_BYTES: usize = 256 * 1024;
const DEFAULT_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactPolicy {
    pub segment_bytes: usize,
    pub per_run_bytes: usize,
    pub total_bytes: usize,
    pub retention: Duration,
}

impl Default for ArtifactPolicy {
    fn default() -> Self {
        Self {
            segment_bytes: DEFAULT_SEGMENT_BYTES,
            per_run_bytes: domain::limits::MAX_CAPTURED_STREAM_BYTES,
            total_bytes: domain::limits::MAX_LOCAL_ARTIFACT_BYTES,
            retention: DEFAULT_RETENTION,
        }
    }
}

impl ArtifactPolicy {
    fn validate(self) -> Result<Self, PersistenceError> {
        if self.segment_bytes == 0
            || self.per_run_bytes == 0
            || self.total_bytes == 0
            || self.segment_bytes > self.per_run_bytes
            || self.per_run_bytes > self.total_bytes
            || self.retention.is_zero()
            || i64::try_from(self.retention.as_millis()).is_err()
        {
            return Err(PersistenceError::InvalidArtifactPolicy);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStream {
    Stdout,
    Stderr,
}

impl ArtifactStream {
    const fn kind(self) -> &'static str {
        match self {
            Self::Stdout => "provider_stdout",
            Self::Stderr => "provider_stderr",
        }
    }

    fn parse(value: &str) -> Result<Self, PersistenceError> {
        match value {
            "provider_stdout" => Ok(Self::Stdout),
            "provider_stderr" => Ok(Self::Stderr),
            _ => Err(PersistenceError::ArtifactMetadataCorrupt),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Writing,
    Complete,
    Deleting,
}

impl ArtifactState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Writing => "writing",
            Self::Complete => "complete",
            Self::Deleting => "deleting",
        }
    }

    fn parse(value: &str) -> Result<Self, PersistenceError> {
        match value {
            "writing" => Ok(Self::Writing),
            "complete" => Ok(Self::Complete),
            "deleting" => Ok(Self::Deleting),
            _ => Err(PersistenceError::ArtifactMetadataCorrupt),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSegment {
    pub sequence: u32,
    pub stored_bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunArtifact {
    pub id: Uuid,
    pub changeset_id: Uuid,
    pub run_id: Uuid,
    pub supervision_id: Uuid,
    pub stream: ArtifactStream,
    pub state: ArtifactState,
    pub source_bytes: usize,
    pub stored_bytes: usize,
    pub segments: Vec<ArtifactSegment>,
    pub sha256: String,
    pub redacted: bool,
    pub process_truncated: bool,
    pub quota_limited: bool,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedArtifactSegment {
    pub artifact: RunArtifact,
    pub segment: ArtifactSegment,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LocalArtifactStore {
    store: SqliteStore,
    root: PathBuf,
    policy: ArtifactPolicy,
}

struct ArtifactOperationLock(File);

struct ArtifactReservation<'a> {
    run_id: Uuid,
    changeset_id: Uuid,
    supervision_id: Uuid,
    stream: ArtifactStream,
    source_bytes: usize,
    content: &'a [u8],
    process_truncated: bool,
    quota_limited: bool,
    now_unix_ms: i64,
}

impl Drop for ArtifactOperationLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

impl LocalArtifactStore {
    pub fn new(
        store: SqliteStore,
        root: impl AsRef<Path>,
        policy: ArtifactPolicy,
    ) -> Result<Self, PersistenceError> {
        if !root.as_ref().is_absolute() {
            return Err(PersistenceError::InvalidArtifactRoot);
        }
        let policy = policy.validate()?;
        create_private_directory(root.as_ref(), true)?;
        let root = fs::canonicalize(root.as_ref())?;
        let temporary = root.join(".tmp");
        create_private_directory(&temporary, false)?;
        Ok(Self {
            store,
            root,
            policy,
        })
    }

    pub fn capture_process_result(
        &self,
        run_id: Uuid,
        changeset_id: Uuid,
        supervision_id: Uuid,
        result: &ProcessResult,
        secrets: &[Vec<u8>],
        now_unix_ms: i64,
    ) -> Result<Vec<RunArtifact>, PersistenceError> {
        let _operation_lock = self.lock_operation()?;
        let streams = [
            (
                ArtifactStream::Stdout,
                result.stdout.as_slice(),
                result.stdout_truncated,
            ),
            (
                ArtifactStream::Stderr,
                result.stderr.as_slice(),
                result.stderr_truncated,
            ),
        ];
        let mut artifacts = Vec::new();
        for (stream, source, process_truncated) in streams {
            if source.is_empty() && !process_truncated {
                continue;
            }
            let available = self.available_capacity(run_id)?;
            if available == 0 {
                continue;
            }
            let (redacted, redaction_limited) =
                redact_bounded(source, secrets, available, process_truncated);
            let Some(mut artifact) = self.reserve(ArtifactReservation {
                run_id,
                changeset_id,
                supervision_id,
                stream,
                source_bytes: source.len(),
                content: &redacted,
                process_truncated,
                quota_limited: redaction_limited,
                now_unix_ms,
            })?
            else {
                continue;
            };
            if let Err(error) = self.write_artifact(&artifact, &redacted[..artifact.stored_bytes]) {
                let _ = self.remove_artifact_files(artifact.id);
                let _ = self.delete_manifest(artifact.id);
                return Err(error);
            }
            artifact.state = ArtifactState::Complete;
            artifact.updated_at_unix_ms = now_unix_ms;
            self.update_manifest(&artifact)?;
            artifacts.push(artifact);
        }
        Ok(artifacts)
    }

    pub fn artifacts_for_run(&self, run_id: Uuid) -> Result<Vec<RunArtifact>, PersistenceError> {
        self.store.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE run_id = ?1 ORDER BY created_at_unix_ms, rowid"
            ))?;
            let rows = statement.query_map([run_id.to_string()], artifact_row)?;
            rows.map(|row| validate_artifact_row(row?)).collect()
        })
    }

    pub fn complete_artifacts_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<RunArtifact>, PersistenceError> {
        let _operation_lock = self.lock_shared_operation()?;
        let artifacts = self.store.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE run_id = ?1 AND status = ?2 ORDER BY created_at_unix_ms, rowid"
            ))?;
            let rows = statement.query_map(
                params![run_id.to_string(), ArtifactState::Complete.as_str()],
                artifact_row,
            )?;
            rows.map(|row| validate_artifact_row(row?))
                .collect::<Result<Vec<_>, PersistenceError>>()
        })?;
        for artifact in &artifacts {
            validate_artifact_layout(artifact)?;
        }
        Ok(artifacts)
    }

    pub fn read_verified_segment(
        &self,
        run_id: Uuid,
        artifact_id: Uuid,
        segment_sequence: u32,
    ) -> Result<VerifiedArtifactSegment, PersistenceError> {
        let _operation_lock = self.lock_shared_operation()?;
        let artifact = self
            .store
            .with_connection(|connection| {
                connection
                    .query_row(
                        &format!(
                            "SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE run_id = ?1 AND id = ?2 AND status = ?3"
                        ),
                        params![
                            run_id.to_string(),
                            artifact_id.to_string(),
                            ArtifactState::Complete.as_str()
                        ],
                        artifact_row,
                    )
                    .optional()?
                    .map(validate_artifact_row)
                    .transpose()
            })?
            .ok_or(PersistenceError::NotFound("artifact"))?;
        validate_artifact_layout(&artifact)?;
        let requested = artifact
            .segments
            .iter()
            .find(|segment| segment.sequence == segment_sequence)
            .cloned()
            .ok_or(PersistenceError::NotFound("artifact segment"))?;
        let bytes = self.verify_artifact_content(&artifact, segment_sequence)?;
        Ok(VerifiedArtifactSegment {
            artifact,
            segment: requested,
            bytes,
        })
    }

    pub fn delete_run_artifacts(&self, run_id: Uuid) -> Result<usize, PersistenceError> {
        let _operation_lock = self.lock_operation()?;
        let artifacts = self.artifacts_for_run(run_id)?;
        for artifact in &artifacts {
            self.delete_artifact(artifact)?;
        }
        Ok(artifacts.len())
    }

    pub fn prune_expired(&self, now_unix_ms: i64) -> Result<usize, PersistenceError> {
        let _operation_lock = self.lock_operation()?;
        let artifacts = self.store.with_connection(|connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE status = ?1 AND expires_at_unix_ms <= ?2 ORDER BY created_at_unix_ms, rowid"
            ))?;
            let rows = statement.query_map(
                params![ArtifactState::Complete.as_str(), now_unix_ms],
                artifact_row,
            )?;
            rows.map(|row| validate_artifact_row(row?)).collect::<Result<Vec<_>, PersistenceError>>()
        })?;
        for artifact in &artifacts {
            self.delete_artifact(artifact)?;
        }
        Ok(artifacts.len())
    }

    pub fn recover_incomplete(&self) -> Result<usize, PersistenceError> {
        let _operation_lock = self.lock_operation()?;
        let ids = self.store.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id FROM artifacts WHERE status IN ('writing', 'deleting') ORDER BY created_at_unix_ms, id",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.map(|row| parse_uuid(&row?, "artifact id"))
                .collect::<Result<Vec<_>, PersistenceError>>()
        })?;
        for id in &ids {
            self.remove_artifact_files(*id)?;
            self.delete_manifest(*id)?;
        }
        self.remove_temporary_directories()?;
        Ok(ids.len())
    }

    fn available_capacity(&self, run_id: Uuid) -> Result<usize, PersistenceError> {
        self.store.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let total_used = stored_usage(&transaction, None)?;
            let run_used = stored_usage(&transaction, Some(run_id))?;
            let available = min(
                self.policy.total_bytes.saturating_sub(total_used),
                self.policy.per_run_bytes.saturating_sub(run_used),
            );
            transaction.commit()?;
            Ok(available)
        })
    }

    fn lock_operation(&self) -> Result<ArtifactOperationLock, PersistenceError> {
        let file = open_lock_file(&self.root.join(".artifact.lock"))?;
        file.lock_exclusive()?;
        Ok(ArtifactOperationLock(file))
    }

    fn lock_shared_operation(&self) -> Result<ArtifactOperationLock, PersistenceError> {
        let file = open_lock_file(&self.root.join(".artifact.lock"))?;
        FileExt::lock_shared(&file)?;
        Ok(ArtifactOperationLock(file))
    }

    fn delete_artifact(&self, artifact: &RunArtifact) -> Result<(), PersistenceError> {
        self.mark_deleting(artifact)?;
        self.remove_artifact_files(artifact.id)?;
        self.delete_manifest(artifact.id)
    }

    fn reserve(
        &self,
        reservation: ArtifactReservation<'_>,
    ) -> Result<Option<RunArtifact>, PersistenceError> {
        let ArtifactReservation {
            run_id,
            changeset_id,
            supervision_id,
            stream,
            source_bytes,
            content,
            process_truncated,
            quota_limited,
            now_unix_ms,
        } = reservation;
        self.store.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let total_used = stored_usage(&transaction, None)?;
            let run_used = stored_usage(&transaction, Some(run_id))?;
            let available = min(
                self.policy.total_bytes.saturating_sub(total_used),
                self.policy.per_run_bytes.saturating_sub(run_used),
            );
            let stored_bytes = min(content.len(), available);
            if stored_bytes == 0 {
                transaction.commit()?;
                return Ok(None);
            }
            let id = Uuid::new_v4();
            let segments = segment_metadata(&content[..stored_bytes], self.policy.segment_bytes)?;
            let retention_ms = i64::try_from(self.policy.retention.as_millis())
                .map_err(|_| PersistenceError::InvalidArtifactPolicy)?;
            let artifact = RunArtifact {
                id,
                changeset_id,
                run_id,
                supervision_id,
                stream,
                state: ArtifactState::Writing,
                source_bytes,
                stored_bytes,
                segments,
                sha256: digest(&content[..stored_bytes]),
                redacted: true,
                process_truncated,
                quota_limited: quota_limited || stored_bytes < content.len(),
                created_at_unix_ms: now_unix_ms,
                updated_at_unix_ms: now_unix_ms,
                expires_at_unix_ms: now_unix_ms.saturating_add(retention_ms),
            };
            transaction.execute(
                "INSERT INTO artifacts (id, changeset_id, run_id, kind, created_at_unix_ms, body, status, stored_bytes, updated_at_unix_ms, expires_at_unix_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    artifact.id.to_string(),
                    artifact.changeset_id.to_string(),
                    artifact.run_id.to_string(),
                    artifact.stream.kind(),
                    artifact.created_at_unix_ms,
                    encode(&artifact)?,
                    artifact.state.as_str(),
                    to_i64(artifact.stored_bytes)?,
                    artifact.updated_at_unix_ms,
                    artifact.expires_at_unix_ms,
                ],
            )?;
            transaction.commit()?;
            Ok(Some(artifact))
        })
    }

    fn verify_artifact_content(
        &self,
        artifact: &RunArtifact,
        requested_sequence: u32,
    ) -> Result<Vec<u8>, PersistenceError> {
        let directory = self.root.join(artifact.id.to_string());
        validate_artifact_directory(&directory)?;
        let mut whole_digest = Sha256::new();
        let mut total_bytes = 0_usize;
        let mut requested_bytes = None;
        let mut buffer = [0_u8; 8192];

        for segment in &artifact.segments {
            let requested = segment.sequence == requested_sequence;
            if requested {
                requested_bytes = Some(Vec::with_capacity(segment.stored_bytes));
            }
            let path = directory.join(segment_name(segment.sequence));
            let mut file = open_artifact_segment(&path, segment.stored_bytes)?;
            let mut segment_digest = Sha256::new();
            let mut remaining = segment.stored_bytes;
            while remaining > 0 {
                let read_limit = min(remaining, buffer.len());
                let read = file
                    .read(&mut buffer[..read_limit])
                    .map_err(|_| PersistenceError::ArtifactIntegrityMismatch)?;
                if read == 0 {
                    return Err(PersistenceError::ArtifactIntegrityMismatch);
                }
                let bytes = &buffer[..read];
                segment_digest.update(bytes);
                whole_digest.update(bytes);
                if requested {
                    requested_bytes
                        .as_mut()
                        .ok_or(PersistenceError::ArtifactIntegrityMismatch)?
                        .extend_from_slice(bytes);
                }
                total_bytes = total_bytes
                    .checked_add(read)
                    .ok_or(PersistenceError::ArtifactMetadataCorrupt)?;
                remaining -= read;
            }
            let mut trailing = [0_u8; 1];
            if file
                .read(&mut trailing)
                .map_err(|_| PersistenceError::ArtifactIntegrityMismatch)?
                != 0
                || format!("{:x}", segment_digest.finalize()) != segment.sha256
            {
                return Err(PersistenceError::ArtifactIntegrityMismatch);
            }
        }

        if total_bytes != artifact.stored_bytes
            || format!("{:x}", whole_digest.finalize()) != artifact.sha256
        {
            return Err(PersistenceError::ArtifactIntegrityMismatch);
        }
        requested_bytes.ok_or(PersistenceError::NotFound("artifact segment"))
    }

    fn write_artifact(
        &self,
        artifact: &RunArtifact,
        content: &[u8],
    ) -> Result<(), PersistenceError> {
        let temporary = self.root.join(".tmp").join(artifact.id.to_string());
        let final_path = self.root.join(artifact.id.to_string());
        create_private_directory(&temporary, false)?;
        for segment in &artifact.segments {
            let start = usize::try_from(segment.sequence)
                .map_err(|_| PersistenceError::ArtifactMetadataCorrupt)?
                .checked_mul(self.policy.segment_bytes)
                .ok_or(PersistenceError::ArtifactMetadataCorrupt)?;
            let end = start
                .checked_add(segment.stored_bytes)
                .ok_or(PersistenceError::ArtifactMetadataCorrupt)?;
            let bytes = content
                .get(start..end)
                .ok_or(PersistenceError::ArtifactMetadataCorrupt)?;
            let path = temporary.join(segment_name(segment.sequence));
            let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        sync_directory(&temporary)?;
        fs::rename(&temporary, &final_path)?;
        sync_directory(&self.root)?;
        Ok(())
    }

    fn update_manifest(&self, artifact: &RunArtifact) -> Result<(), PersistenceError> {
        self.store.with_connection(|connection| {
            let updated = connection.execute(
                "UPDATE artifacts SET body = ?2, status = ?3, stored_bytes = ?4, updated_at_unix_ms = ?5, expires_at_unix_ms = ?6 WHERE id = ?1",
                params![
                    artifact.id.to_string(),
                    encode(artifact)?,
                    artifact.state.as_str(),
                    to_i64(artifact.stored_bytes)?,
                    artifact.updated_at_unix_ms,
                    artifact.expires_at_unix_ms,
                ],
            )?;
            if updated != 1 {
                return Err(PersistenceError::NotFound("artifact"));
            }
            Ok(())
        })
    }

    fn mark_deleting(&self, artifact: &RunArtifact) -> Result<(), PersistenceError> {
        let mut deleting = artifact.clone();
        deleting.state = ArtifactState::Deleting;
        self.update_manifest(&deleting)
    }

    fn delete_manifest(&self, artifact_id: Uuid) -> Result<(), PersistenceError> {
        self.store.with_connection(|connection| {
            connection.execute(
                "DELETE FROM artifacts WHERE id = ?1",
                [artifact_id.to_string()],
            )?;
            Ok(())
        })
    }

    fn remove_artifact_files(&self, artifact_id: Uuid) -> Result<(), PersistenceError> {
        remove_directory_if_present(&self.root.join(artifact_id.to_string()))?;
        remove_directory_if_present(&self.root.join(".tmp").join(artifact_id.to_string()))?;
        Ok(())
    }

    fn remove_temporary_directories(&self) -> Result<(), PersistenceError> {
        let temporary = self.root.join(".tmp");
        validate_private_directory(&temporary)?;
        for entry in fs::read_dir(temporary)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}

type ArtifactRow = (
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
    i64,
    i64,
    Option<i64>,
);

fn artifact_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn validate_artifact_row(row: ArtifactRow) -> Result<RunArtifact, PersistenceError> {
    let (
        id,
        changeset_id,
        run_id,
        kind,
        created_at_unix_ms,
        body,
        status,
        stored_bytes,
        updated_at_unix_ms,
        expires_at_unix_ms,
    ) = row;
    let artifact: RunArtifact = decode(&body)?;
    let matches_scalars = artifact.id == parse_uuid(&id, "artifact id")?
        && artifact.changeset_id == parse_uuid(&changeset_id, "artifact changeset id")?
        && artifact.run_id == parse_uuid(&run_id, "artifact run id")?
        && artifact.stream == ArtifactStream::parse(&kind)?
        && artifact.state == ArtifactState::parse(&status)?
        && artifact.created_at_unix_ms == created_at_unix_ms
        && artifact.stored_bytes
            == usize::try_from(stored_bytes)
                .map_err(|_| PersistenceError::ArtifactMetadataCorrupt)?
        && artifact.updated_at_unix_ms == updated_at_unix_ms
        && Some(artifact.expires_at_unix_ms) == expires_at_unix_ms;
    if !matches_scalars {
        return Err(PersistenceError::ArtifactMetadataCorrupt);
    }
    Ok(artifact)
}

fn validate_artifact_layout(artifact: &RunArtifact) -> Result<(), PersistenceError> {
    if artifact.state != ArtifactState::Complete
        || artifact.stored_bytes == 0
        || artifact.stored_bytes > domain::limits::MAX_CAPTURED_STREAM_BYTES
        || artifact.segments.is_empty()
        || !is_sha256(&artifact.sha256)
    {
        return Err(PersistenceError::ArtifactMetadataCorrupt);
    }

    let mut stored_bytes = 0_usize;
    for (index, segment) in artifact.segments.iter().enumerate() {
        let expected_sequence =
            u32::try_from(index).map_err(|_| PersistenceError::ArtifactMetadataCorrupt)?;
        if segment.sequence != expected_sequence
            || segment.stored_bytes == 0
            || segment.stored_bytes > domain::limits::MAX_CAPTURED_STREAM_BYTES
            || !is_sha256(&segment.sha256)
        {
            return Err(PersistenceError::ArtifactMetadataCorrupt);
        }
        stored_bytes = stored_bytes
            .checked_add(segment.stored_bytes)
            .ok_or(PersistenceError::ArtifactMetadataCorrupt)?;
    }
    if stored_bytes != artifact.stored_bytes {
        return Err(PersistenceError::ArtifactMetadataCorrupt);
    }
    Ok(())
}

fn validate_artifact_directory(path: &Path) -> Result<(), PersistenceError> {
    validate_private_directory(path).map_err(|_| PersistenceError::ArtifactIntegrityMismatch)
}

fn open_artifact_segment(path: &Path, expected_bytes: usize) -> Result<File, PersistenceError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| PersistenceError::ArtifactIntegrityMismatch)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || usize::try_from(metadata.len()).ok() != Some(expected_bytes)
    {
        return Err(PersistenceError::ArtifactIntegrityMismatch);
    }
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| PersistenceError::ArtifactIntegrityMismatch)?;
    let opened = file
        .metadata()
        .map_err(|_| PersistenceError::ArtifactIntegrityMismatch)?;
    if !opened.is_file() || usize::try_from(opened.len()).ok() != Some(expected_bytes) {
        return Err(PersistenceError::ArtifactIntegrityMismatch);
    }
    Ok(file)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn stored_usage(
    transaction: &rusqlite::Transaction<'_>,
    run_id: Option<Uuid>,
) -> Result<usize, PersistenceError> {
    let stored: i64 = match run_id {
        Some(run_id) => transaction.query_row(
            "SELECT COALESCE(SUM(stored_bytes), 0) FROM artifacts WHERE run_id = ?1",
            [run_id.to_string()],
            |row| row.get(0),
        )?,
        None => transaction.query_row(
            "SELECT COALESCE(SUM(stored_bytes), 0) FROM artifacts",
            [],
            |row| row.get(0),
        )?,
    };
    usize::try_from(stored).map_err(|_| PersistenceError::ArtifactMetadataCorrupt)
}

fn segment_metadata(
    content: &[u8],
    segment_bytes: usize,
) -> Result<Vec<ArtifactSegment>, PersistenceError> {
    content
        .chunks(segment_bytes)
        .enumerate()
        .map(|(sequence, bytes)| {
            Ok(ArtifactSegment {
                sequence: u32::try_from(sequence)
                    .map_err(|_| PersistenceError::ArtifactMetadataCorrupt)?,
                stored_bytes: bytes.len(),
                sha256: digest(bytes),
            })
        })
        .collect()
}

fn redact_bounded(
    input: &[u8],
    secrets: &[Vec<u8>],
    limit: usize,
    input_truncated: bool,
) -> (Vec<u8>, bool) {
    let mut secrets = secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    secrets.sort_unstable_by_key(|secret| std::cmp::Reverse(secret.len()));
    secrets.dedup();

    let trailing_secret_prefix = input_truncated
        .then(|| longest_trailing_secret_prefix(input, &secrets))
        .flatten()
        .unwrap_or(0);
    let complete_input = &input[..input.len().saturating_sub(trailing_secret_prefix)];
    let mut output = Vec::with_capacity(min(input.len(), limit));
    let mut offset = 0;
    let mut limited = false;
    while offset < complete_input.len() {
        let matched = secrets
            .iter()
            .find(|secret| complete_input[offset..].starts_with(secret));
        let (replacement, consumed) = match matched {
            Some(secret) => (REDACTION_MARKER, secret.len()),
            None => (&complete_input[offset..offset + 1], 1),
        };
        let remaining = limit.saturating_sub(output.len());
        output.extend_from_slice(&replacement[..min(replacement.len(), remaining)]);
        offset += consumed;
        if output.len() == limit && (offset < complete_input.len() || trailing_secret_prefix > 0) {
            limited = true;
            break;
        }
    }
    if trailing_secret_prefix > 0 && output.len() < limit {
        let remaining = limit - output.len();
        output.extend_from_slice(&REDACTION_MARKER[..min(REDACTION_MARKER.len(), remaining)]);
        limited |= remaining < REDACTION_MARKER.len();
    }
    (output, limited)
}

fn longest_trailing_secret_prefix(input: &[u8], secrets: &[&[u8]]) -> Option<usize> {
    secrets
        .iter()
        .filter_map(|secret| {
            let maximum = min(input.len(), secret.len().saturating_sub(1));
            (1..=maximum)
                .rev()
                .find(|length| input.ends_with(&secret[..*length]))
        })
        .max()
}

fn digest(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}

fn segment_name(sequence: u32) -> String {
    format!("{sequence:08}.segment")
}

fn to_i64(value: usize) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::ArtifactMetadataCorrupt)
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, PersistenceError> {
    Uuid::parse_str(value).map_err(|_| PersistenceError::CorruptIdentifier(field))
}

fn create_private_directory(path: &Path, recursive: bool) -> Result<(), PersistenceError> {
    let result = if recursive {
        fs::create_dir_all(path)
    } else {
        fs::create_dir(path)
    };
    if let Err(error) = result
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(error.into());
    }
    validate_private_directory(path)?;
    secure_directory(path)
}

fn validate_private_directory(path: &Path) -> Result<(), PersistenceError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PersistenceError::InvalidArtifactRoot);
    }
    Ok(())
}

fn open_lock_file(path: &Path) -> Result<File, PersistenceError> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(PersistenceError::InvalidArtifactRoot);
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(PersistenceError::InvalidArtifactRoot);
    }
    secure_file(&file)?;
    Ok(file)
}

fn remove_directory_if_present(path: &Path) -> Result<(), PersistenceError> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn sync_directory(path: &Path) -> Result<(), PersistenceError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), PersistenceError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn secure_directory(_: &Path) -> Result<(), PersistenceError> {
    Ok(())
}

#[cfg(unix)]
fn secure_file(file: &File) -> Result<(), PersistenceError> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn secure_file(_: &File) -> Result<(), PersistenceError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Changeset, Repository, Run};
    use execution::TerminalOutcome;
    use tempfile::tempdir;

    fn seed_run(store: &SqliteStore, repository_id: Uuid) -> (Uuid, Uuid) {
        let changeset = Changeset::new(repository_id, "base".to_owned());
        store.save_changeset(&changeset).unwrap();
        let run = Run::new(changeset.id);
        store.save_run(&run).unwrap();
        (changeset.id, run.id)
    }

    fn fixture(
        policy: ArtifactPolicy,
    ) -> (tempfile::TempDir, LocalArtifactStore, Uuid, Uuid, Uuid) {
        let directory = tempdir().unwrap();
        let store = SqliteStore::open(directory.path().join("state.sqlite3")).unwrap();
        let repository = Repository {
            id: Uuid::new_v4(),
            filesystem_identity: "fixture".to_owned(),
            git_directory_identity: "fixture-git".to_owned(),
            canonical_path: directory.path().join("repository"),
            identity: "fixture".to_owned(),
            primary_remote: None,
            default_branch: "main".to_owned(),
            base_sha: "base".to_owned(),
            version: 0,
        };
        store.save_repository(&repository).unwrap();
        let (changeset_id, run_id) = seed_run(&store, repository.id);
        let artifacts =
            LocalArtifactStore::new(store, directory.path().join("artifacts"), policy).unwrap();
        (directory, artifacts, repository.id, changeset_id, run_id)
    }

    fn process_result(stdout: &[u8], stderr: &[u8]) -> ProcessResult {
        ProcessResult {
            outcome: TerminalOutcome::Completed(0),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    fn artifact_content(store: &LocalArtifactStore, artifact: &RunArtifact) -> Vec<u8> {
        let mut content = Vec::new();
        for segment in &artifact.segments {
            content.extend(
                fs::read(
                    store
                        .root
                        .join(artifact.id.to_string())
                        .join(segment_name(segment.sequence)),
                )
                .unwrap(),
            );
        }
        content
    }

    #[test]
    fn capture_redacts_segments_and_deletes_without_exposing_secrets() {
        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 128,
            total_bytes: 128,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let result = process_result(b"token=super-secret;super-secret", b"warning");
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &result,
                &[b"super-secret".to_vec()],
                100,
            )
            .unwrap();

        assert_eq!(artifacts.len(), 2);
        let stdout = artifacts
            .iter()
            .find(|artifact| artifact.stream == ArtifactStream::Stdout)
            .unwrap();
        let content = artifact_content(&store, stdout);
        assert!(
            !content
                .windows(b"super-secret".len())
                .any(|bytes| bytes == b"super-secret")
        );
        assert!(
            content
                .windows(REDACTION_MARKER.len())
                .any(|bytes| bytes == REDACTION_MARKER)
        );
        assert_eq!(stdout.sha256, digest(&content));
        assert!(stdout.segments.len() > 1);
        assert_eq!(store.artifacts_for_run(run_id).unwrap(), artifacts);

        assert_eq!(store.delete_run_artifacts(run_id).unwrap(), 2);
        assert!(store.artifacts_for_run(run_id).unwrap().is_empty());
        for artifact in artifacts {
            assert!(!store.root.join(artifact.id.to_string()).exists());
        }
    }

    #[test]
    fn verified_segment_reads_are_bounded_and_verify_the_whole_artifact() {
        let policy = ArtifactPolicy {
            segment_bytes: 4,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefghij", b""),
                &[],
                100,
            )
            .unwrap();
        let artifact = &artifacts[0];

        assert_eq!(store.complete_artifacts_for_run(run_id).unwrap(), artifacts);
        let verified = store.read_verified_segment(run_id, artifact.id, 1).unwrap();
        assert_eq!(verified.segment, artifact.segments[1]);
        assert_eq!(verified.bytes, b"efgh");
        assert!(verified.bytes.len() <= policy.segment_bytes);
        assert!(matches!(
            store.read_verified_segment(run_id, artifact.id, 99),
            Err(PersistenceError::NotFound("artifact segment"))
        ));

        fs::write(
            store
                .root
                .join(artifact.id.to_string())
                .join(segment_name(2)),
            b"zz",
        )
        .unwrap();
        assert!(matches!(
            store.read_verified_segment(run_id, artifact.id, 0),
            Err(PersistenceError::ArtifactIntegrityMismatch)
        ));
    }

    #[test]
    fn verified_reads_accept_artifacts_created_under_an_older_policy() {
        let original_policy = ArtifactPolicy {
            segment_bytes: 4,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (directory, store, _repository_id, changeset_id, run_id) = fixture(original_policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefghij", b""),
                &[],
                100,
            )
            .unwrap();
        let reopened = LocalArtifactStore::new(
            store.store.clone(),
            directory.path().join("artifacts"),
            ArtifactPolicy {
                segment_bytes: 8,
                per_run_bytes: 128,
                total_bytes: 128,
                retention: Duration::from_secs(120),
            },
        )
        .unwrap();

        assert_eq!(
            reopened
                .read_verified_segment(run_id, artifacts[0].id, 1)
                .unwrap()
                .bytes,
            b"efgh"
        );
    }

    #[test]
    fn verified_segment_rejects_wrong_run_and_whole_digest_corruption() {
        let policy = ArtifactPolicy {
            segment_bytes: 4,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefgh", b""),
                &[],
                100,
            )
            .unwrap();
        let artifact = &artifacts[0];
        let (_other_changeset_id, other_run_id) = seed_run(&store.store, repository_id);

        assert!(matches!(
            store.read_verified_segment(other_run_id, artifact.id, 0),
            Err(PersistenceError::NotFound("artifact"))
        ));

        let mut corrupt = artifact.clone();
        corrupt.sha256 = digest(b"different content");
        store
            .store
            .with_connection(|connection| {
                connection.execute(
                    "UPDATE artifacts SET body = ?2 WHERE id = ?1",
                    params![artifact.id.to_string(), encode(&corrupt)?],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(matches!(
            store.read_verified_segment(run_id, artifact.id, 0),
            Err(PersistenceError::ArtifactIntegrityMismatch)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn verified_segment_rejects_symlink_replacement() {
        use std::os::unix::fs::symlink;

        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"stdout", b""),
                &[],
                100,
            )
            .unwrap();
        let artifact = &artifacts[0];
        let segment_path = store
            .root
            .join(artifact.id.to_string())
            .join(segment_name(0));
        let outside = directory.path().join("outside");
        fs::write(&outside, b"outside").unwrap();
        fs::remove_file(&segment_path).unwrap();
        symlink(&outside, &segment_path).unwrap();

        assert!(matches!(
            store.read_verified_segment(run_id, artifact.id, 0),
            Err(PersistenceError::ArtifactIntegrityMismatch)
        ));
        assert_eq!(fs::read(outside).unwrap(), b"outside");
    }

    #[test]
    fn combined_run_and_global_quotas_bound_persisted_bytes() {
        let policy = ArtifactPolicy {
            segment_bytes: 4,
            per_run_bytes: 12,
            total_bytes: 12,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefgh", b"ijklmnop"),
                &[],
                100,
            )
            .unwrap();

        assert_eq!(
            artifacts
                .iter()
                .map(|artifact| artifact.stored_bytes)
                .sum::<usize>(),
            12
        );
        assert!(artifacts.iter().any(|artifact| artifact.quota_limited));
        assert_eq!(artifact_content(&store, &artifacts[0]), b"abcdefgh");
        assert_eq!(artifact_content(&store, &artifacts[1]), b"ijkl");
    }

    #[test]
    fn global_quota_is_shared_across_runs_without_empty_manifests() {
        let policy = ArtifactPolicy {
            segment_bytes: 2,
            per_run_bytes: 8,
            total_bytes: 10,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, repository_id, changeset_id, run_id) = fixture(policy);
        let first = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefgh", b""),
                &[],
                100,
            )
            .unwrap();
        let (second_changeset_id, second_run_id) = seed_run(&store.store, repository_id);
        let second = store
            .capture_process_result(
                second_run_id,
                second_changeset_id,
                Uuid::new_v4(),
                &process_result(b"ijklmnop", b""),
                &[],
                101,
            )
            .unwrap();
        let third = store
            .capture_process_result(
                second_run_id,
                second_changeset_id,
                Uuid::new_v4(),
                &process_result(b"more", b""),
                &[],
                102,
            )
            .unwrap();

        assert_eq!(first[0].stored_bytes, 8);
        assert_eq!(second[0].stored_bytes, 2);
        assert!(second[0].quota_limited);
        assert!(third.is_empty());
        assert_eq!(store.artifacts_for_run(second_run_id).unwrap().len(), 1);
    }

    #[test]
    fn truncated_secret_prefix_is_redacted() {
        let (redacted, limited) =
            redact_bounded(b"token=super-secr", &[b"super-secret".to_vec()], 128, true);

        assert_eq!(redacted, b"token=[REDACTED]");
        assert!(!limited);
    }

    #[test]
    fn deleting_artifacts_continue_to_count_against_quota() {
        let policy = ArtifactPolicy {
            segment_bytes: 4,
            per_run_bytes: 8,
            total_bytes: 8,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"abcdefgh", b""),
                &[],
                100,
            )
            .unwrap();

        store.mark_deleting(&artifacts[0]).unwrap();
        assert_eq!(store.available_capacity(run_id).unwrap(), 0);
        assert_eq!(store.recover_incomplete().unwrap(), 1);
    }

    #[test]
    fn pruning_removes_only_expired_artifacts() {
        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_millis(10),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let expired = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"expired", b""),
                &[],
                100,
            )
            .unwrap();
        let current = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"current", b""),
                &[],
                200,
            )
            .unwrap();

        assert_eq!(store.prune_expired(150).unwrap(), 1);
        assert!(!store.root.join(expired[0].id.to_string()).exists());
        assert_eq!(store.artifacts_for_run(run_id).unwrap(), current);
    }

    #[test]
    fn scalar_manifest_divergence_is_rejected() {
        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"stdout", b""),
                &[],
                100,
            )
            .unwrap();
        store
            .store
            .with_connection(|connection| {
                connection.execute(
                    "UPDATE artifacts SET stored_bytes = stored_bytes + 1 WHERE id = ?1",
                    [artifacts[0].id.to_string()],
                )?;
                Ok(())
            })
            .unwrap();

        assert!(matches!(
            store.artifacts_for_run(run_id),
            Err(PersistenceError::ArtifactMetadataCorrupt)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn recovery_rejects_a_replaced_temporary_directory() {
        use std::os::unix::fs::symlink;

        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_secs(60),
        };
        let (directory, store, _repository_id, _changeset_id, _run_id) = fixture(policy);
        let outside = directory.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("preserved"), "safe").unwrap();
        fs::remove_dir(store.root.join(".tmp")).unwrap();
        symlink(&outside, store.root.join(".tmp")).unwrap();

        assert!(matches!(
            store.recover_incomplete(),
            Err(PersistenceError::InvalidArtifactRoot)
        ));
        assert_eq!(
            fs::read_to_string(outside.join("preserved")).unwrap(),
            "safe"
        );
    }

    #[test]
    fn recovery_and_retention_remove_incomplete_or_expired_artifacts() {
        let policy = ArtifactPolicy {
            segment_bytes: 8,
            per_run_bytes: 64,
            total_bytes: 64,
            retention: Duration::from_millis(1),
        };
        let (_directory, store, _repository_id, changeset_id, run_id) = fixture(policy);
        let artifacts = store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"stdout", b""),
                &[],
                100,
            )
            .unwrap();
        store
            .store
            .with_connection(|connection| {
                connection.execute(
                    "UPDATE artifacts SET status = 'writing' WHERE id = ?1",
                    [artifacts[0].id.to_string()],
                )?;
                Ok(())
            })
            .unwrap();

        assert_eq!(store.recover_incomplete().unwrap(), 1);
        assert!(store.artifacts_for_run(run_id).unwrap().is_empty());

        store
            .capture_process_result(
                run_id,
                changeset_id,
                Uuid::new_v4(),
                &process_result(b"new stdout", b""),
                &[],
                200,
            )
            .unwrap();
        assert_eq!(store.prune_expired(202).unwrap(), 1);
        assert!(store.artifacts_for_run(run_id).unwrap().is_empty());
    }
}
