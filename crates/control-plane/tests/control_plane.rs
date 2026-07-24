use control_plane::{
    ControlPlaneError, ControlPlaneStore, Permission, QuotaLimits, RegisterAttachment,
    TicketPriority, WorkspaceRole,
};
use tempfile::TempDir;

fn store() -> (TempDir, ControlPlaneStore) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let store = ControlPlaneStore::open(directory.path().join("jet-black.sqlite"))
        .expect("open control plane");
    (directory, store)
}

#[test]
fn migrates_reopens_and_prevents_multiple_owners() {
    let (directory, store) = store();
    assert_eq!(store.schema_version().expect("schema version"), 1);
    let clone = store.clone();
    drop(store);
    assert!(matches!(
        ControlPlaneStore::open(directory.path().join("jet-black.sqlite")),
        Err(ControlPlaneError::InstanceAlreadyRunning(_))
    ));
    drop(clone);
    let reopened = ControlPlaneStore::open(directory.path().join("jet-black.sqlite"))
        .expect("reopen after owner dropped");
    reopened.integrity_check().expect("database integrity");
}

#[test]
fn product_and_execution_migrations_share_one_database() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("jet-black.sqlite");
    let execution = persistence::SqliteStore::open(&path).expect("execution store");
    let product = ControlPlaneStore::open(&path).expect("product store");
    let user = product
        .create_user(
            "shared@example.com",
            "Shared database",
            "correct horse battery",
        )
        .expect("product record");
    assert_eq!(product.schema_version().expect("product schema"), 1);
    assert_eq!(product.user(user.id).expect("user lookup"), Some(user));
    assert_eq!(
        execution
            .repository(uuid::Uuid::new_v4())
            .expect("execution query"),
        None
    );
}

#[test]
fn password_sessions_rotate_revoke_and_require_csrf() {
    let (_directory, store) = store();
    let user = store
        .create_user("Ada@Example.com", "Ada", "correct horse battery")
        .expect("create user");
    assert!(matches!(
        store.authenticate_password("ada@example.com", "wrong password"),
        Err(ControlPlaneError::InvalidCredentials)
    ));
    let session = store
        .authenticate_password("ada@example.com", "correct horse battery")
        .expect("authenticate");
    let authenticated = store
        .validate_session(&session.token, Some(&session.csrf_token))
        .expect("validate session");
    assert_eq!(authenticated.user.id, user.id);
    assert!(matches!(
        store.validate_session(&session.token, Some("wrong")),
        Err(ControlPlaneError::InvalidCsrfToken)
    ));

    let rotated = store
        .rotate_session(&session.token, &session.csrf_token)
        .expect("rotate");
    assert!(matches!(
        store.validate_session(&session.token, None),
        Err(ControlPlaneError::InvalidSession)
    ));
    store
        .validate_session(&rotated.token, Some(&rotated.csrf_token))
        .expect("new session works");
    assert!(store.revoke_session(rotated.session_id).expect("revoke"));
    assert!(matches!(
        store.validate_session(&rotated.token, None),
        Err(ControlPlaneError::InvalidSession)
    ));
}

#[test]
fn launch_tokens_are_one_time_and_expiring_session_credentials() {
    let (_directory, store) = store();
    let user = store
        .create_user("launch@example.com", "Launch", "correct horse battery")
        .expect("create user");
    let launch = store.issue_launch_token(user.id).expect("launch token");
    let session = store
        .exchange_launch_token(&launch.token)
        .expect("exchange token");
    assert_eq!(
        store
            .validate_session(&session.token, None)
            .expect("session")
            .user
            .id,
        user.id
    );
    assert!(matches!(
        store.exchange_launch_token(&launch.token),
        Err(ControlPlaneError::InvalidLaunchToken)
    ));
}

#[test]
fn enforces_role_matrix_and_cross_workspace_denials() {
    let (_directory, store) = store();
    let owner = store
        .create_user("owner@example.com", "Owner", "correct horse battery")
        .expect("owner");
    let admin = store
        .create_user("admin@example.com", "Admin", "correct horse battery")
        .expect("admin");
    let member = store
        .create_user("member@example.com", "Member", "correct horse battery")
        .expect("member");
    let guest = store
        .create_user("guest@example.com", "Guest", "correct horse battery")
        .expect("guest");
    let outsider = store
        .create_user("other@example.com", "Other", "correct horse battery")
        .expect("outsider");
    let workspace = store
        .create_workspace(owner.id, "platform", "Platform")
        .expect("workspace");
    store
        .add_workspace_member(owner.id, workspace.id, admin.id, WorkspaceRole::Admin)
        .expect("admin membership");
    store
        .add_workspace_member(owner.id, workspace.id, member.id, WorkspaceRole::Member)
        .expect("member membership");
    store
        .add_workspace_member(owner.id, workspace.id, guest.id, WorkspaceRole::Guest)
        .expect("guest membership");

    for user in [owner.id, admin.id, member.id, guest.id] {
        store
            .require_permission(user, workspace.id, Permission::Read)
            .expect("members can read");
    }
    for user in [owner.id, admin.id, member.id] {
        store
            .require_permission(user, workspace.id, Permission::ManageTickets)
            .expect("non-guests manage tickets");
    }
    assert!(matches!(
        store.require_permission(guest.id, workspace.id, Permission::ManageTickets),
        Err(ControlPlaneError::Forbidden)
    ));
    assert!(matches!(
        store.require_permission(member.id, workspace.id, Permission::ManageProjects),
        Err(ControlPlaneError::Forbidden)
    ));
    assert!(matches!(
        store.require_permission(admin.id, workspace.id, Permission::DeleteWorkspace),
        Err(ControlPlaneError::Forbidden)
    ));
    assert!(matches!(
        store.require_permission(outsider.id, workspace.id, Permission::Read),
        Err(ControlPlaneError::Forbidden)
    ));
}

#[test]
fn creates_projects_and_idempotent_tickets_with_quota_enforcement() {
    let (_directory, store) = store();
    let owner = store
        .create_user("owner@example.com", "Owner", "correct horse battery")
        .expect("owner");
    let workspace = store
        .create_workspace(owner.id, "platform", "Platform")
        .expect("workspace");
    let project = store
        .create_project(
            owner.id,
            workspace.id,
            "JB",
            "Jet Black",
            "Agentic delivery",
            Some("github:example/jet-black"),
        )
        .expect("project");
    let limits = QuotaLimits {
        tickets: 1,
        attachment_bytes: 1024,
    };
    let first = store
        .create_ticket(
            owner.id,
            project.id,
            "Build control plane",
            "Replace Convex",
            TicketPriority::Urgent,
            Some("ticket-1"),
            limits,
        )
        .expect("ticket");
    let replay = store
        .create_ticket(
            owner.id,
            project.id,
            "ignored replay body",
            "",
            TicketPriority::None,
            Some("ticket-1"),
            limits,
        )
        .expect("idempotent replay");
    assert_eq!(replay.id, first.id);
    assert_eq!(replay.sequence_number, 1);
    assert!(matches!(
        store.create_ticket(
            owner.id,
            project.id,
            "Second",
            "",
            TicketPriority::Low,
            Some("ticket-2"),
            limits,
        ),
        Err(ControlPlaneError::QuotaExceeded("tickets"))
    ));
}

#[test]
fn deletion_invariants_and_backup_are_enforced() {
    let (directory, store) = store();
    let owner = store
        .create_user("owner@example.com", "Owner", "correct horse battery")
        .expect("owner");
    let workspace = store
        .create_workspace(owner.id, "platform", "Platform")
        .expect("workspace");
    assert!(matches!(
        store.delete_user(owner.id),
        Err(ControlPlaneError::SoleWorkspaceOwner)
    ));

    let backup_path = directory.path().join("backup.sqlite");
    store.backup_to(&backup_path).expect("backup");
    let backup = ControlPlaneStore::open(&backup_path).expect("open backup");
    assert_eq!(
        backup
            .role(owner.id, workspace.id)
            .expect("role from backup"),
        Some(WorkspaceRole::Owner)
    );
    drop(backup);

    store
        .delete_workspace(owner.id, workspace.id)
        .expect("delete workspace");
    store.delete_user(owner.id).expect("delete user");
}

#[test]
fn ownership_can_be_transferred_before_account_deletion() {
    let (_directory, store) = store();
    let first = store
        .create_user("first@example.com", "First", "correct horse battery")
        .expect("first owner");
    let second = store
        .create_user("second@example.com", "Second", "correct horse battery")
        .expect("second owner");
    let workspace = store
        .create_workspace(first.id, "transfer", "Transfer")
        .expect("workspace");
    store
        .add_workspace_member(first.id, workspace.id, second.id, WorkspaceRole::Admin)
        .expect("member");
    store
        .transfer_workspace_ownership(first.id, workspace.id, second.id)
        .expect("transfer");
    assert_eq!(
        store.role(first.id, workspace.id).expect("old owner role"),
        Some(WorkspaceRole::Admin)
    );
    assert_eq!(
        store.role(second.id, workspace.id).expect("new owner role"),
        Some(WorkspaceRole::Owner)
    );
    store.delete_user(first.id).expect("delete previous owner");
}

#[test]
fn attachment_quotas_and_development_seed_are_deterministic() {
    let (_directory, store) = store();
    let seed = store
        .seed_development("development password")
        .expect("development seed");
    let repeated = store
        .seed_development("development password")
        .expect("repeat seed");
    assert_eq!(seed, repeated);
    let ticket = store
        .create_ticket(
            seed.user.id,
            seed.project.id,
            "Attach diagnostics",
            "",
            TicketPriority::Medium,
            Some("seed-attachment-ticket"),
            QuotaLimits::default(),
        )
        .expect("ticket");
    let limits = QuotaLimits {
        tickets: 10,
        attachment_bytes: 10,
    };
    store
        .register_attachment(
            seed.user.id,
            RegisterAttachment {
                ticket_id: ticket.id,
                digest: "abc123",
                file_name: "trace.txt",
                media_type: "text/plain",
                byte_length: 10,
                storage_path: "ab/abc123",
            },
            limits,
        )
        .expect("attachment at quota");
    assert!(matches!(
        store.register_attachment(
            seed.user.id,
            RegisterAttachment {
                ticket_id: ticket.id,
                digest: "def456",
                file_name: "extra.txt",
                media_type: "text/plain",
                byte_length: 1,
                storage_path: "de/def456",
            },
            limits,
        ),
        Err(ControlPlaneError::QuotaExceeded("attachment bytes"))
    ));
}
