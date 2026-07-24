use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use fs2::FileExt;
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, backup::Backup, params,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;

const CURRENT_SCHEMA_VERSION: u32 = 1;
const SESSION_BYTES: usize = 32;
const CSRF_BYTES: usize = 24;
const DEFAULT_SESSION_LIFETIME: Duration = Duration::from_secs(60 * 60 * 24 * 30);
const DEFAULT_LAUNCH_TOKEN_LIFETIME: Duration = Duration::from_secs(60);

#[derive(Debug)]
struct StoreInner {
    path: PathBuf,
    _instance_lock: File,
}

#[derive(Debug, Clone)]
pub struct ControlPlaneStore {
    inner: Arc<StoreInner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub disabled_at_ms: Option<i64>,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
    Guest,
}

impl WorkspaceRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }

    fn parse(value: &str) -> Result<Self, ControlPlaneError> {
        match value {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            _ => Err(ControlPlaneError::CorruptData(format!(
                "unknown workspace role {value}"
            ))),
        }
    }

    pub fn permits(self, permission: Permission) -> bool {
        match self {
            Self::Owner => true,
            Self::Admin => !matches!(permission, Permission::DeleteWorkspace),
            Self::Member => matches!(
                permission,
                Permission::Read | Permission::ManageTickets | Permission::Comment
            ),
            Self::Guest => matches!(permission, Permission::Read | Permission::Comment),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Comment,
    ManageTickets,
    ManageProjects,
    ManageMembers,
    DeleteWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_by_id: Uuid,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub repository_identity: Option<String>,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriority {
    None,
    Urgent,
    High,
    Medium,
    Low,
}

impl TicketPriority {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Urgent => "urgent",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: Uuid,
    pub project_id: Uuid,
    pub sequence_number: u64,
    pub title: String,
    pub description: String,
    pub state_id: Option<Uuid>,
    pub priority: TicketPriority,
    pub created_by_id: Uuid,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub ticket_id: Uuid,
    pub digest: String,
    pub file_name: String,
    pub media_type: String,
    pub byte_length: u64,
    pub storage_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterAttachment<'a> {
    pub ticket_id: Uuid,
    pub digest: &'a str,
    pub file_name: &'a str,
    pub media_type: &'a str,
    pub byte_length: u64,
    pub storage_path: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentSeed {
    pub user: User,
    pub workspace: Workspace,
    pub project: Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedSession {
    pub session_id: Uuid,
    pub token: String,
    pub csrf_token: String,
    pub user_id: Uuid,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub session_id: Uuid,
    pub user: User,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedLaunchToken {
    pub token: String,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaLimits {
    pub tickets: u64,
    pub attachment_bytes: u64,
}

impl Default for QuotaLimits {
    fn default() -> Self {
        Self {
            tickets: 100_000,
            attachment_bytes: 10 * 1024 * 1024 * 1024,
        }
    }
}

impl ControlPlaneStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ControlPlaneError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let lock_path = path.with_extension("instance.lock");
        let instance_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        instance_lock
            .try_lock_exclusive()
            .map_err(|_| ControlPlaneError::InstanceAlreadyRunning(lock_path))?;

        let store = Self {
            inner: Arc::new(StoreInner {
                path,
                _instance_lock: instance_lock,
            }),
        };
        store.with_connection(run_migrations)?;
        Ok(store)
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, ControlPlaneError>,
    ) -> Result<T, ControlPlaneError> {
        let mut connection = Connection::open(&self.inner.path)?;
        configure_connection(&connection)?;
        operation(&mut connection)
    }

    pub fn create_user(
        &self,
        email: &str,
        display_name: &str,
        password: &str,
    ) -> Result<User, ControlPlaneError> {
        validate_non_empty("email", email)?;
        validate_non_empty("display name", display_name)?;
        validate_password(password)?;

        let id = Uuid::new_v4();
        let now = now_ms()?;
        let normalized_email = email.trim().to_lowercase();
        let password_hash = hash_password(password)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "INSERT INTO users (
                    id, email, display_name, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![id.to_string(), normalized_email, display_name.trim(), now],
            )?;
            transaction.execute(
                "INSERT INTO password_credentials (user_id, password_hash, updated_at_ms)
                 VALUES (?1, ?2, ?3)",
                params![id.to_string(), password_hash, now],
            )?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(User {
            id,
            email: normalized_email,
            display_name: display_name.trim().to_owned(),
            disabled_at_ms: None,
            version: 1,
        })
    }

    pub fn authenticate_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<IssuedSession, ControlPlaneError> {
        let normalized_email = email.trim().to_lowercase();
        let credential = self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT users.id, password_credentials.password_hash
                     FROM users
                     JOIN password_credentials ON password_credentials.user_id = users.id
                     WHERE users.email = ?1 AND users.disabled_at_ms IS NULL",
                    [normalized_email],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(Into::into)
        })?;
        let Some((user_id, password_hash)) = credential else {
            return Err(ControlPlaneError::InvalidCredentials);
        };
        verify_password(password, &password_hash)?;
        self.issue_session(parse_uuid(&user_id)?, "password", DEFAULT_SESSION_LIFETIME)
    }

    pub fn issue_launch_token(
        &self,
        user_id: Uuid,
    ) -> Result<IssuedLaunchToken, ControlPlaneError> {
        let token = random_token(SESSION_BYTES)?;
        let expires_at_ms = add_duration(now_ms()?, DEFAULT_LAUNCH_TOKEN_LIFETIME)?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO launch_tokens (id, token_hash, user_id, expires_at_ms)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    Uuid::new_v4().to_string(),
                    token_hash(&token).as_slice(),
                    user_id.to_string(),
                    expires_at_ms
                ],
            )?;
            Ok(())
        })?;
        Ok(IssuedLaunchToken {
            token,
            expires_at_ms,
        })
    }

    pub fn exchange_launch_token(&self, token: &str) -> Result<IssuedSession, ControlPlaneError> {
        let now = now_ms()?;
        let user_id = self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let row = transaction
                .query_row(
                    "SELECT id, user_id FROM launch_tokens
                     WHERE token_hash = ?1 AND consumed_at_ms IS NULL AND expires_at_ms > ?2",
                    params![token_hash(token).as_slice(), now],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .ok_or(ControlPlaneError::InvalidLaunchToken)?;
            transaction.execute(
                "UPDATE launch_tokens SET consumed_at_ms = ?1 WHERE id = ?2",
                params![now, row.0],
            )?;
            transaction.commit()?;
            parse_uuid(&row.1)
        })?;
        self.issue_session(user_id, "launch_token", DEFAULT_SESSION_LIFETIME)
    }

    pub fn validate_session(
        &self,
        token: &str,
        csrf_token: Option<&str>,
    ) -> Result<AuthenticatedSession, ControlPlaneError> {
        let now = now_ms()?;
        let token_digest = token_hash(token);
        self.with_connection(|connection| {
            let row = connection
                .query_row(
                    "SELECT sessions.id, sessions.csrf_hash, sessions.expires_at_ms,
                            users.id, users.email, users.display_name,
                            users.disabled_at_ms, users.version
                     FROM sessions
                     JOIN users ON users.id = sessions.user_id
                     WHERE sessions.token_hash = ?1
                       AND sessions.revoked_at_ms IS NULL
                       AND sessions.expires_at_ms > ?2
                       AND users.disabled_at_ms IS NULL",
                    params![token_digest.as_slice(), now],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Vec<u8>>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, Option<i64>>(6)?,
                            row.get::<_, u64>(7)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(ControlPlaneError::InvalidSession)?;
            if let Some(csrf_token) = csrf_token {
                if row.1.as_slice() != token_hash(csrf_token).as_slice() {
                    return Err(ControlPlaneError::InvalidCsrfToken);
                }
            }
            connection.execute(
                "UPDATE sessions SET last_seen_at_ms = ?1 WHERE id = ?2",
                params![now, row.0],
            )?;
            Ok(AuthenticatedSession {
                session_id: parse_uuid(&row.0)?,
                expires_at_ms: row.2,
                user: User {
                    id: parse_uuid(&row.3)?,
                    email: row.4,
                    display_name: row.5,
                    disabled_at_ms: row.6,
                    version: row.7,
                },
            })
        })
    }

    pub fn rotate_session(
        &self,
        token: &str,
        csrf_token: &str,
    ) -> Result<IssuedSession, ControlPlaneError> {
        let authenticated = self.validate_session(token, Some(csrf_token))?;
        let issued = build_session(
            authenticated.user.id,
            add_duration(now_ms()?, DEFAULT_SESSION_LIFETIME)?,
        )?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            insert_session(&transaction, &issued, "password")?;
            let changed = transaction.execute(
                "UPDATE sessions SET revoked_at_ms = ?1, replaced_by_id = ?2
                 WHERE id = ?3 AND revoked_at_ms IS NULL",
                params![
                    now_ms()?,
                    issued.session_id.to_string(),
                    authenticated.session_id.to_string()
                ],
            )?;
            if changed != 1 {
                return Err(ControlPlaneError::InvalidSession);
            }
            transaction.commit()?;
            Ok(())
        })?;
        Ok(issued)
    }

    pub fn revoke_session(&self, session_id: Uuid) -> Result<bool, ControlPlaneError> {
        let now = now_ms()?;
        self.with_connection(|connection| {
            Ok(connection.execute(
                "UPDATE sessions SET revoked_at_ms = ?1
                 WHERE id = ?2 AND revoked_at_ms IS NULL",
                params![now, session_id.to_string()],
            )? == 1)
        })
    }

    pub fn create_workspace(
        &self,
        actor_id: Uuid,
        slug: &str,
        name: &str,
    ) -> Result<Workspace, ControlPlaneError> {
        validate_slug(slug)?;
        validate_non_empty("workspace name", name)?;
        let workspace = Workspace {
            id: Uuid::new_v4(),
            slug: slug.trim().to_lowercase(),
            name: name.trim().to_owned(),
            created_by_id: actor_id,
            version: 1,
        };
        let now = now_ms()?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "INSERT INTO workspaces (
                    id, slug, name, created_by_id, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    workspace.id.to_string(),
                    workspace.slug,
                    workspace.name,
                    actor_id.to_string(),
                    now
                ],
            )?;
            transaction.execute(
                "INSERT INTO workspace_members (workspace_id, user_id, role, joined_at_ms)
                 VALUES (?1, ?2, 'owner', ?3)",
                params![workspace.id.to_string(), actor_id.to_string(), now],
            )?;
            append_event(
                &transaction,
                workspace.id,
                "workspace",
                workspace.id,
                1,
                "workspace.created",
                actor_id,
                None,
                "{}",
                now,
            )?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(workspace)
    }

    pub fn add_workspace_member(
        &self,
        actor_id: Uuid,
        workspace_id: Uuid,
        user_id: Uuid,
        role: WorkspaceRole,
    ) -> Result<(), ControlPlaneError> {
        self.require_permission(actor_id, workspace_id, Permission::ManageMembers)?;
        if role == WorkspaceRole::Owner {
            return Err(ControlPlaneError::InvalidInput(
                "ownership must be transferred explicitly".to_owned(),
            ));
        }
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO workspace_members (workspace_id, user_id, role, joined_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(workspace_id, user_id) DO UPDATE SET role = excluded.role",
                params![
                    workspace_id.to_string(),
                    user_id.to_string(),
                    role.as_str(),
                    now_ms()?
                ],
            )?;
            Ok(())
        })
    }

    pub fn transfer_workspace_ownership(
        &self,
        actor_id: Uuid,
        workspace_id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<(), ControlPlaneError> {
        if self.role(actor_id, workspace_id)? != Some(WorkspaceRole::Owner) {
            return Err(ControlPlaneError::Forbidden);
        }
        if actor_id == new_owner_id {
            return Ok(());
        }
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let target_exists = transaction.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM workspace_members
                    WHERE workspace_id = ?1 AND user_id = ?2
                 )",
                params![workspace_id.to_string(), new_owner_id.to_string()],
                |row| row.get::<_, bool>(0),
            )?;
            if !target_exists {
                return Err(ControlPlaneError::InvalidInput(
                    "new owner must already be a workspace member".to_owned(),
                ));
            }
            transaction.execute(
                "UPDATE workspace_members SET role = 'admin'
                 WHERE workspace_id = ?1 AND user_id = ?2",
                params![workspace_id.to_string(), actor_id.to_string()],
            )?;
            transaction.execute(
                "UPDATE workspace_members SET role = 'owner'
                 WHERE workspace_id = ?1 AND user_id = ?2",
                params![workspace_id.to_string(), new_owner_id.to_string()],
            )?;
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn role(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<Option<WorkspaceRole>, ControlPlaneError> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT role FROM workspace_members
                     WHERE workspace_id = ?1 AND user_id = ?2",
                    params![workspace_id.to_string(), user_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .map(|value| WorkspaceRole::parse(&value))
                .transpose()
        })
    }

    pub fn require_permission(
        &self,
        user_id: Uuid,
        workspace_id: Uuid,
        permission: Permission,
    ) -> Result<WorkspaceRole, ControlPlaneError> {
        let role = self
            .role(user_id, workspace_id)?
            .ok_or(ControlPlaneError::Forbidden)?;
        if !role.permits(permission) {
            return Err(ControlPlaneError::Forbidden);
        }
        Ok(role)
    }

    pub fn create_project(
        &self,
        actor_id: Uuid,
        workspace_id: Uuid,
        identifier: &str,
        name: &str,
        description: &str,
        repository_identity: Option<&str>,
    ) -> Result<Project, ControlPlaneError> {
        self.require_permission(actor_id, workspace_id, Permission::ManageProjects)?;
        validate_identifier(identifier)?;
        validate_non_empty("project name", name)?;
        let project = Project {
            id: Uuid::new_v4(),
            workspace_id,
            identifier: identifier.trim().to_uppercase(),
            name: name.trim().to_owned(),
            description: description.trim().to_owned(),
            repository_identity: repository_identity.map(str::to_owned),
            version: 1,
        };
        let now = now_ms()?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "INSERT INTO projects (
                    id, workspace_id, identifier, name, description,
                    repository_identity, created_by_id, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    project.id.to_string(),
                    workspace_id.to_string(),
                    project.identifier,
                    project.name,
                    project.description,
                    project.repository_identity,
                    actor_id.to_string(),
                    now
                ],
            )?;
            for (name, group, color, position) in [
                ("Backlog", "backlog", "#64748b", 0.0),
                ("Todo", "unstarted", "#94a3b8", 1.0),
                ("In Progress", "started", "#f59e0b", 2.0),
                ("Done", "completed", "#22c55e", 3.0),
                ("Cancelled", "cancelled", "#ef4444", 4.0),
            ] {
                transaction.execute(
                    "INSERT INTO workflow_states (
                        id, project_id, name, state_group, color, position
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        Uuid::new_v4().to_string(),
                        project.id.to_string(),
                        name,
                        group,
                        color,
                        position
                    ],
                )?;
            }
            append_event(
                &transaction,
                workspace_id,
                "project",
                project.id,
                1,
                "project.created",
                actor_id,
                None,
                "{}",
                now,
            )?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(project)
    }

    pub fn create_ticket(
        &self,
        actor_id: Uuid,
        project_id: Uuid,
        title: &str,
        description: &str,
        priority: TicketPriority,
        idempotency_key: Option<&str>,
        limits: QuotaLimits,
    ) -> Result<Ticket, ControlPlaneError> {
        validate_non_empty("ticket title", title)?;
        let workspace_id = self.workspace_for_project(project_id)?;
        self.require_permission(actor_id, workspace_id, Permission::ManageTickets)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            if let Some(key) = idempotency_key {
                if let Some(ticket) = ticket_for_idempotency_key(&transaction, workspace_id, key)? {
                    transaction.commit()?;
                    return Ok(ticket);
                }
            }
            enforce_ticket_quota(&transaction, workspace_id, limits)?;
            let next_sequence = transaction.query_row(
                "SELECT COALESCE(MAX(sequence_number), 0) + 1
                 FROM tickets WHERE project_id = ?1",
                [project_id.to_string()],
                |row| row.get::<_, u64>(0),
            )?;
            let state_id = transaction
                .query_row(
                    "SELECT id FROM workflow_states
                     WHERE project_id = ?1 AND state_group = 'backlog'
                     ORDER BY position LIMIT 1",
                    [project_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .map(|value| parse_uuid(&value))
                .transpose()?;
            let ticket = Ticket {
                id: Uuid::new_v4(),
                project_id,
                sequence_number: next_sequence,
                title: title.trim().to_owned(),
                description: description.trim().to_owned(),
                state_id,
                priority,
                created_by_id: actor_id,
                version: 1,
            };
            let now = now_ms()?;
            transaction.execute(
                "INSERT INTO tickets (
                    id, project_id, sequence_number, title, description, state_id,
                    priority, created_by_id, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![
                    ticket.id.to_string(),
                    project_id.to_string(),
                    ticket.sequence_number,
                    ticket.title,
                    ticket.description,
                    ticket.state_id.map(|id| id.to_string()),
                    ticket.priority.as_str(),
                    actor_id.to_string(),
                    now
                ],
            )?;
            let event_body = format!("{{\"ticket_id\":\"{}\"}}", ticket.id);
            append_event(
                &transaction,
                workspace_id,
                "ticket",
                ticket.id,
                1,
                "ticket.created",
                actor_id,
                idempotency_key,
                &event_body,
                now,
            )?;
            transaction.commit()?;
            Ok(ticket)
        })
    }

    pub fn register_attachment(
        &self,
        actor_id: Uuid,
        input: RegisterAttachment<'_>,
        limits: QuotaLimits,
    ) -> Result<Attachment, ControlPlaneError> {
        validate_non_empty("attachment digest", input.digest)?;
        validate_non_empty("attachment file name", input.file_name)?;
        validate_non_empty("attachment media type", input.media_type)?;
        validate_non_empty("attachment storage path", input.storage_path)?;
        let workspace_id = self.with_connection(|connection| {
            let value = connection
                .query_row(
                    "SELECT projects.workspace_id
                     FROM tickets JOIN projects ON projects.id = tickets.project_id
                     WHERE tickets.id = ?1",
                    [input.ticket_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or(ControlPlaneError::NotFound("ticket"))?;
            parse_uuid(&value)
        })?;
        self.require_permission(actor_id, workspace_id, Permission::Comment)?;
        let attachment = Attachment {
            id: Uuid::new_v4(),
            workspace_id,
            ticket_id: input.ticket_id,
            digest: input.digest.to_owned(),
            file_name: input.file_name.to_owned(),
            media_type: input.media_type.to_owned(),
            byte_length: input.byte_length,
            storage_path: input.storage_path.to_owned(),
        };
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let used = transaction
                .query_row(
                    "SELECT attachment_bytes FROM quota_usage WHERE workspace_id = ?1",
                    [workspace_id.to_string()],
                    |row| row.get::<_, u64>(0),
                )
                .optional()?
                .unwrap_or(0);
            if used.saturating_add(input.byte_length) > limits.attachment_bytes {
                return Err(ControlPlaneError::QuotaExceeded("attachment bytes"));
            }
            transaction.execute(
                "INSERT INTO attachments (
                    id, workspace_id, ticket_id, digest, file_name, media_type,
                    byte_length, storage_path, uploaded_by_id, created_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    attachment.id.to_string(),
                    workspace_id.to_string(),
                    input.ticket_id.to_string(),
                    attachment.digest,
                    attachment.file_name,
                    attachment.media_type,
                    input.byte_length,
                    attachment.storage_path,
                    actor_id.to_string(),
                    now_ms()?
                ],
            )?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(attachment)
    }

    pub fn seed_development(&self, password: &str) -> Result<DevelopmentSeed, ControlPlaneError> {
        validate_password(password)?;
        let user_id = Uuid::from_u128(1);
        let workspace_id = Uuid::from_u128(2);
        let project_id = Uuid::from_u128(3);
        let now = now_ms()?;
        let password_hash = hash_password(password)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute(
                "INSERT OR IGNORE INTO users (
                    id, email, display_name, created_at_ms, updated_at_ms
                 ) VALUES (?1, 'dev@jet-black.local', 'Jet Black Developer', ?2, ?2)",
                params![user_id.to_string(), now],
            )?;
            transaction.execute(
                "INSERT INTO password_credentials (user_id, password_hash, updated_at_ms)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id) DO UPDATE SET
                    password_hash = excluded.password_hash,
                    updated_at_ms = excluded.updated_at_ms",
                params![user_id.to_string(), password_hash, now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO workspaces (
                    id, slug, name, created_by_id, created_at_ms, updated_at_ms
                 ) VALUES (?1, 'jet-black-dev', 'Jet Black Demo', ?2, ?3, ?3)",
                params![workspace_id.to_string(), user_id.to_string(), now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO workspace_members (
                    workspace_id, user_id, role, joined_at_ms
                 ) VALUES (?1, ?2, 'owner', ?3)",
                params![workspace_id.to_string(), user_id.to_string(), now],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO projects (
                    id, workspace_id, identifier, name, description,
                    created_by_id, created_at_ms, updated_at_ms
                 ) VALUES (
                    ?1, ?2, 'JB', 'Jet Black', 'Agentic delivery workspace',
                    ?3, ?4, ?4
                 )",
                params![
                    project_id.to_string(),
                    workspace_id.to_string(),
                    user_id.to_string(),
                    now
                ],
            )?;
            let existing_states = transaction.query_row(
                "SELECT COUNT(*) FROM workflow_states WHERE project_id = ?1",
                [project_id.to_string()],
                |row| row.get::<_, u64>(0),
            )?;
            if existing_states == 0 {
                for (offset, name, group, color) in [
                    (10_u128, "Backlog", "backlog", "#64748b"),
                    (11, "Todo", "unstarted", "#94a3b8"),
                    (12, "In Progress", "started", "#f59e0b"),
                    (13, "Done", "completed", "#22c55e"),
                    (14, "Cancelled", "cancelled", "#ef4444"),
                ] {
                    transaction.execute(
                        "INSERT INTO workflow_states (
                            id, project_id, name, state_group, color, position
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            Uuid::from_u128(offset).to_string(),
                            project_id.to_string(),
                            name,
                            group,
                            color,
                            offset as f64
                        ],
                    )?;
                }
            }
            transaction.commit()?;
            Ok(())
        })?;
        Ok(DevelopmentSeed {
            user: User {
                id: user_id,
                email: "dev@jet-black.local".to_owned(),
                display_name: "Jet Black Developer".to_owned(),
                disabled_at_ms: None,
                version: 1,
            },
            workspace: Workspace {
                id: workspace_id,
                slug: "jet-black-dev".to_owned(),
                name: "Jet Black Demo".to_owned(),
                created_by_id: user_id,
                version: 1,
            },
            project: Project {
                id: project_id,
                workspace_id,
                identifier: "JB".to_owned(),
                name: "Jet Black".to_owned(),
                description: "Agentic delivery workspace".to_owned(),
                repository_identity: None,
                version: 1,
            },
        })
    }

    pub fn delete_user(&self, user_id: Uuid) -> Result<(), ControlPlaneError> {
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let owned = transaction.query_row(
                "SELECT COUNT(*) FROM workspace_members
                 WHERE user_id = ?1 AND role = 'owner'",
                [user_id.to_string()],
                |row| row.get::<_, u64>(0),
            )?;
            if owned > 0 {
                return Err(ControlPlaneError::SoleWorkspaceOwner);
            }
            let now = now_ms()?;
            transaction.execute(
                "DELETE FROM workspace_members WHERE user_id = ?1",
                [user_id.to_string()],
            )?;
            transaction.execute(
                "DELETE FROM password_credentials WHERE user_id = ?1",
                [user_id.to_string()],
            )?;
            transaction.execute(
                "DELETE FROM sessions WHERE user_id = ?1",
                [user_id.to_string()],
            )?;
            transaction.execute(
                "DELETE FROM launch_tokens WHERE user_id = ?1",
                [user_id.to_string()],
            )?;
            let changed = transaction.execute(
                "UPDATE users SET
                    email = ?1,
                    display_name = 'Deleted user',
                    disabled_at_ms = ?2,
                    updated_at_ms = ?2,
                    version = version + 1
                 WHERE id = ?3 AND disabled_at_ms IS NULL",
                params![
                    format!("deleted+{}@jet-black.invalid", user_id),
                    now,
                    user_id.to_string()
                ],
            )?;
            if changed == 0 {
                return Err(ControlPlaneError::NotFound("user"));
            }
            transaction.commit()?;
            Ok(())
        })
    }

    pub fn delete_workspace(
        &self,
        actor_id: Uuid,
        workspace_id: Uuid,
    ) -> Result<(), ControlPlaneError> {
        self.require_permission(actor_id, workspace_id, Permission::DeleteWorkspace)?;
        self.with_connection(|connection| {
            let changed = connection.execute(
                "DELETE FROM workspaces WHERE id = ?1",
                [workspace_id.to_string()],
            )?;
            if changed == 0 {
                return Err(ControlPlaneError::NotFound("workspace"));
            }
            Ok(())
        })
    }

    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<(), ControlPlaneError> {
        let source = Connection::open(&self.inner.path)?;
        configure_connection(&source)?;
        let mut destination = Connection::open(destination)?;
        let backup = Backup::new(&source, &mut destination)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<(), ControlPlaneError> {
        self.with_connection(|connection| {
            let result = connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))?;
            if result != "ok" {
                return Err(ControlPlaneError::CorruptData(result));
            }
            Ok(())
        })
    }

    pub fn schema_version(&self) -> Result<u32, ControlPlaneError> {
        self.with_connection(|connection| {
            connection
                .pragma_query_value(None, "user_version", |row| row.get(0))
                .map_err(Into::into)
        })
    }

    fn issue_session(
        &self,
        user_id: Uuid,
        method: &str,
        lifetime: Duration,
    ) -> Result<IssuedSession, ControlPlaneError> {
        let issued = build_session(user_id, add_duration(now_ms()?, lifetime)?)?;
        self.with_connection(|connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            insert_session(&transaction, &issued, method)?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(issued)
    }

    fn workspace_for_project(&self, project_id: Uuid) -> Result<Uuid, ControlPlaneError> {
        self.with_connection(|connection| {
            let value = connection
                .query_row(
                    "SELECT workspace_id FROM projects WHERE id = ?1",
                    [project_id.to_string()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or(ControlPlaneError::NotFound("project"))?;
            parse_uuid(&value)
        })
    }
}

fn configure_connection(connection: &Connection) -> Result<(), ControlPlaneError> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

fn run_migrations(connection: &mut Connection) -> Result<(), ControlPlaneError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(ControlPlaneError::NewerSchema {
            database: version,
            runtime: CURRENT_SCHEMA_VERSION,
        });
    }
    if version < 1 {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(include_str!("../migrations/001_product.sql"))?;
        transaction.pragma_update(None, "user_version", 1)?;
        transaction.commit()?;
    }
    Ok(())
}

fn insert_session(
    transaction: &Transaction<'_>,
    issued: &IssuedSession,
    method: &str,
) -> Result<(), ControlPlaneError> {
    let now = now_ms()?;
    transaction.execute(
        "INSERT INTO sessions (
            id, user_id, token_hash, csrf_hash, method, issued_at_ms, expires_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            issued.session_id.to_string(),
            issued.user_id.to_string(),
            token_hash(&issued.token).as_slice(),
            token_hash(&issued.csrf_token).as_slice(),
            method,
            now,
            issued.expires_at_ms
        ],
    )?;
    Ok(())
}

fn append_event(
    transaction: &Transaction<'_>,
    workspace_id: Uuid,
    aggregate_kind: &str,
    aggregate_id: Uuid,
    aggregate_version: u64,
    event_kind: &str,
    actor_id: Uuid,
    idempotency_key: Option<&str>,
    body: &str,
    created_at_ms: i64,
) -> Result<(), ControlPlaneError> {
    transaction.execute(
        "INSERT INTO product_events (
            workspace_id, aggregate_kind, aggregate_id, aggregate_version,
            event_kind, actor_id, idempotency_key, body, created_at_ms
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            workspace_id.to_string(),
            aggregate_kind,
            aggregate_id.to_string(),
            aggregate_version,
            event_kind,
            actor_id.to_string(),
            idempotency_key,
            body,
            created_at_ms
        ],
    )?;
    Ok(())
}

fn ticket_for_idempotency_key(
    transaction: &Transaction<'_>,
    workspace_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<Ticket>, ControlPlaneError> {
    let ticket_id = transaction
        .query_row(
            "SELECT aggregate_id FROM product_events
             WHERE workspace_id = ?1 AND idempotency_key = ?2
               AND aggregate_kind = 'ticket' AND event_kind = 'ticket.created'",
            params![workspace_id.to_string(), idempotency_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(ticket_id) = ticket_id else {
        return Ok(None);
    };
    transaction
        .query_row(
            "SELECT id, project_id, sequence_number, title, description,
                    state_id, priority, created_by_id, version
             FROM tickets WHERE id = ?1",
            [ticket_id],
            decode_ticket,
        )
        .optional()?
        .map(ticket_from_row)
        .transpose()
}

type TicketRow = (
    String,
    String,
    u64,
    String,
    String,
    Option<String>,
    String,
    String,
    u64,
);

fn decode_ticket(row: &rusqlite::Row<'_>) -> rusqlite::Result<TicketRow> {
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
    ))
}

fn ticket_from_row(row: TicketRow) -> Result<Ticket, ControlPlaneError> {
    Ok(Ticket {
        id: parse_uuid(&row.0)?,
        project_id: parse_uuid(&row.1)?,
        sequence_number: row.2,
        title: row.3,
        description: row.4,
        state_id: row.5.as_deref().map(parse_uuid).transpose()?,
        priority: match row.6.as_str() {
            "none" => TicketPriority::None,
            "urgent" => TicketPriority::Urgent,
            "high" => TicketPriority::High,
            "medium" => TicketPriority::Medium,
            "low" => TicketPriority::Low,
            value => {
                return Err(ControlPlaneError::CorruptData(format!(
                    "unknown ticket priority {value}"
                )));
            }
        },
        created_by_id: parse_uuid(&row.7)?,
        version: row.8,
    })
}

fn enforce_ticket_quota(
    transaction: &Transaction<'_>,
    workspace_id: Uuid,
    limits: QuotaLimits,
) -> Result<(), ControlPlaneError> {
    let used = transaction
        .query_row(
            "SELECT tickets_count FROM quota_usage WHERE workspace_id = ?1",
            [workspace_id.to_string()],
            |row| row.get::<_, u64>(0),
        )
        .optional()?
        .unwrap_or(0);
    if used >= limits.tickets {
        return Err(ControlPlaneError::QuotaExceeded("tickets"));
    }
    Ok(())
}

fn build_session(user_id: Uuid, expires_at_ms: i64) -> Result<IssuedSession, ControlPlaneError> {
    Ok(IssuedSession {
        session_id: Uuid::new_v4(),
        token: random_token(SESSION_BYTES)?,
        csrf_token: random_token(CSRF_BYTES)?,
        user_id,
        expires_at_ms,
    })
}

fn hash_password(password: &str) -> Result<String, ControlPlaneError> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes)
        .map_err(|error| ControlPlaneError::Random(error.to_string()))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|error| ControlPlaneError::PasswordHash(error.to_string()))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| ControlPlaneError::PasswordHash(error.to_string()))
}

fn verify_password(password: &str, encoded: &str) -> Result<(), ControlPlaneError> {
    let hash = PasswordHash::new(encoded)
        .map_err(|error| ControlPlaneError::PasswordHash(error.to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| ControlPlaneError::InvalidCredentials)
}

fn random_token(bytes: usize) -> Result<String, ControlPlaneError> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value).map_err(|error| ControlPlaneError::Random(error.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

fn token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn now_ms() -> Result<i64, ControlPlaneError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ControlPlaneError::Clock(error.to_string()))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| ControlPlaneError::Clock("timestamp is out of range".to_owned()))
}

fn add_duration(timestamp_ms: i64, duration: Duration) -> Result<i64, ControlPlaneError> {
    let delta = i64::try_from(duration.as_millis())
        .map_err(|_| ControlPlaneError::Clock("duration is out of range".to_owned()))?;
    timestamp_ms
        .checked_add(delta)
        .ok_or_else(|| ControlPlaneError::Clock("timestamp overflow".to_owned()))
}

fn parse_uuid(value: &str) -> Result<Uuid, ControlPlaneError> {
    Uuid::parse_str(value).map_err(|error| ControlPlaneError::CorruptData(error.to_string()))
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), ControlPlaneError> {
    if value.trim().is_empty() {
        return Err(ControlPlaneError::InvalidInput(format!(
            "{field} cannot be empty"
        )));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), ControlPlaneError> {
    if password.len() < 12 {
        return Err(ControlPlaneError::InvalidInput(
            "password must contain at least 12 characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_slug(slug: &str) -> Result<(), ControlPlaneError> {
    let slug = slug.trim();
    if slug.is_empty()
        || slug.len() > 64
        || !slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return Err(ControlPlaneError::InvalidInput(
            "workspace slug must use lowercase letters, numbers, and hyphens".to_owned(),
        ));
    }
    Ok(())
}

fn validate_identifier(identifier: &str) -> Result<(), ControlPlaneError> {
    let identifier = identifier.trim();
    if identifier.is_empty()
        || identifier.len() > 12
        || !identifier
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err(ControlPlaneError::InvalidInput(
            "project identifier must be 1-12 letters or numbers".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ControlPlaneError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("secure random generation failed: {0}")]
    Random(String),
    #[error("another Jet Black instance owns {0}")]
    InstanceAlreadyRunning(PathBuf),
    #[error("database schema {database} is newer than runtime schema {runtime}")]
    NewerSchema { database: u32, runtime: u32 },
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid or expired session")]
    InvalidSession,
    #[error("invalid CSRF token")]
    InvalidCsrfToken,
    #[error("invalid or expired launch token")]
    InvalidLaunchToken,
    #[error("forbidden")]
    Forbidden,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("user is the owner of one or more workspaces")]
    SoleWorkspaceOwner,
    #[error("{0} quota exceeded")]
    QuotaExceeded(&'static str),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("password hashing failed: {0}")]
    PasswordHash(String),
    #[error("clock error: {0}")]
    Clock(String),
    #[error("database contains invalid data: {0}")]
    CorruptData(String),
}
