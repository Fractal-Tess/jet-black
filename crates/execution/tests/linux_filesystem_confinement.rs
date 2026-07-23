#![cfg(target_os = "linux")]

use execution::{
    CancellationToken, ExecutionError, FilesystemConfinementReport, LinuxFilesystemConfinement,
    NetworkConfinementReport, ProcessConfinement, ProcessSpec, ProcessSupervisor, TerminalOutcome,
    supervise,
};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tempfile::TempDir;

struct TestLayout {
    _root: TempDir,
    workspace: PathBuf,
    state: PathBuf,
    outside: PathBuf,
}

impl TestLayout {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let state = root.path().join("state");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        Self {
            _root: root,
            workspace,
            state,
            outside,
        }
    }

    fn policy(&self) -> LinuxFilesystemConfinement {
        LinuxFilesystemConfinement {
            workspace: self.workspace.clone(),
            writable_state: self.state.clone(),
            runtime_read_execute: runtime_read_execute_roots(),
            runtime_read_only: Vec::new(),
            runtime_read_write: Vec::new(),
        }
    }
}

fn confined_spec(layout: &TestLayout, script: impl Into<String>) -> ProcessSpec {
    ProcessSpec {
        program: "/bin/sh".to_owned(),
        arguments: vec!["-c".to_owned(), script.into()],
        environment: HashMap::from([
            ("WORKSPACE".to_owned(), path_text(&layout.workspace)),
            ("STATE".to_owned(), path_text(&layout.state)),
            ("OUTSIDE".to_owned(), path_text(&layout.outside)),
        ]),
        sensitive_environment_keys: Vec::new(),
        current_dir: Some(layout.workspace.clone()),
        timeout: Duration::from_secs(5),
        output_limit: 4096,
        confinement: ProcessConfinement::LinuxFilesystem(layout.policy()),
    }
}

fn runtime_read_execute_roots() -> Vec<PathBuf> {
    let shell = std::fs::canonicalize("/bin/sh").unwrap();
    if shell.starts_with("/nix/store") {
        return vec![PathBuf::from("/nix/store")];
    }
    vec![
        PathBuf::from("/bin"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/lib"),
        PathBuf::from("/usr/lib64"),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect()
}

fn command_path(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap()
}

fn path_text(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn run(spec: &ProcessSpec) -> execution::ProcessResult {
    supervise(spec, &CancellationToken::default()).unwrap()
}

#[test]
fn workspace_is_readable_but_not_writable_or_executable() {
    let layout = TestLayout::new();
    std::fs::write(layout.workspace.join("input"), "allowed\n").unwrap();
    let executable = layout.workspace.join("workspace-tool");
    std::fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();
    let spec = confined_spec(
        &layout,
        r#"
IFS= read -r contents < "$WORKSPACE/input" || exit 10
[ "$contents" = allowed ] || exit 11
if printf blocked > "$WORKSPACE/new"; then exit 12; fi
if "$WORKSPACE/workspace-tool"; then exit 13; fi
exit 0
"#,
    );

    let result = run(&spec);

    assert_eq!(result.outcome, TerminalOutcome::Completed(0));
    assert!(!layout.workspace.join("new").exists());
}

#[test]
fn access_outside_the_policy_is_denied() {
    let layout = TestLayout::new();
    std::fs::write(layout.outside.join("secret"), "blocked\n").unwrap();
    let spec = confined_spec(
        &layout,
        r#"
if IFS= read -r contents < "$OUTSIDE/secret"; then exit 20; fi
if printf blocked > "$OUTSIDE/new"; then exit 21; fi
exit 0
"#,
    );

    let result = run(&spec);

    assert_eq!(result.outcome, TerminalOutcome::Completed(0));
    assert!(!layout.outside.join("new").exists());
}

#[test]
fn inherited_file_descriptors_cannot_bypass_confinement() {
    let layout = TestLayout::new();
    let outside_file = layout.outside.join("inherited-write");
    std::fs::write(&outside_file, "unchanged\n").unwrap();
    let inherited = OpenOptions::new().write(true).open(&outside_file).unwrap();
    let inherited_fd = inherited.as_raw_fd();
    assert_eq!(unsafe { libc::fcntl(inherited_fd, libc::F_SETFD, 0) }, 0);
    let mut spec = confined_spec(
        &layout,
        r#"
if eval "printf escaped >&$INHERITED_FD"; then exit 25; fi
exit 0
"#,
    );
    spec.environment
        .insert("INHERITED_FD".to_owned(), inherited_fd.to_string());

    let result = run(&spec);

    assert_eq!(result.outcome, TerminalOutcome::Completed(0));
    assert_eq!(std::fs::read(outside_file).unwrap(), b"unchanged\n");
}

#[test]
fn writable_state_supports_mutation_but_not_execution() {
    let layout = TestLayout::new();
    let state_executable = layout.state.join("state-tool");
    std::fs::write(&state_executable, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(&state_executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&state_executable, permissions).unwrap();
    let move_command = command_path("mv");
    let remove_command = command_path("rm");
    let mut spec = confined_spec(
        &layout,
        r#"
printf created > "$STATE/created" || exit 30
printf truncate > "$STATE/truncated" || exit 31
: > "$STATE/truncated" || exit 32
"$MOVE" "$STATE/created" "$STATE/renamed" || exit 33
"$REMOVE" "$STATE/renamed" || exit 34
if "$STATE/state-tool"; then exit 35; fi
exit 0
"#,
    );
    spec.environment
        .insert("MOVE".to_owned(), path_text(&move_command));
    spec.environment
        .insert("REMOVE".to_owned(), path_text(&remove_command));

    let result = run(&spec);

    assert_eq!(result.outcome, TerminalOutcome::Completed(0));
    assert_eq!(std::fs::read(layout.state.join("truncated")).unwrap(), b"");
    assert!(!layout.state.join("created").exists());
    assert!(!layout.state.join("renamed").exists());
}

#[test]
fn explicitly_writable_runtime_files_support_read_write_access() {
    let layout = TestLayout::new();
    let runtime_file = layout.outside.join("runtime-file");
    std::fs::write(&runtime_file, "longer original contents").unwrap();
    let mut spec = confined_spec(
        &layout,
        r#"
printf 'replacement\n' > "$RUNTIME_FILE" || exit 36
IFS= read -r contents < "$RUNTIME_FILE" || exit 37
[ "$contents" = replacement ] || exit 38
printf discard > /dev/null || exit 39
"#,
    );
    spec.environment
        .insert("RUNTIME_FILE".to_owned(), path_text(&runtime_file));
    let ProcessConfinement::LinuxFilesystem(policy) = &mut spec.confinement else {
        panic!("expected Linux filesystem confinement");
    };
    policy
        .runtime_read_write
        .extend([runtime_file.clone(), PathBuf::from("/dev/null")]);

    let result = run(&spec);

    assert_eq!(
        result.outcome,
        TerminalOutcome::Completed(0),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(runtime_file).unwrap(), b"replacement\n");
}

#[test]
fn descendants_inherit_the_filesystem_policy() {
    let layout = TestLayout::new();
    std::fs::write(layout.outside.join("secret"), "blocked\n").unwrap();
    let spec = confined_spec(
        &layout,
        r#"
/bin/sh -c 'if IFS= read -r contents < "$OUTSIDE/secret"; then exit 40; fi'
"#,
    );

    let result = run(&spec);

    assert_eq!(result.outcome, TerminalOutcome::Completed(0));
}

#[test]
fn unsafe_policies_fail_before_payload_execution() {
    let layout = TestLayout::new();
    let marker = layout.state.join("payload-ran");
    let supervisor = ProcessSupervisor;
    let mut overlap = confined_spec(&layout, format!("printf ran > {}", marker.display()));
    overlap.confinement = ProcessConfinement::LinuxFilesystem(LinuxFilesystemConfinement {
        workspace: layout.workspace.clone(),
        writable_state: layout.workspace.join("nested-state"),
        runtime_read_execute: runtime_read_execute_roots(),
        runtime_read_only: Vec::new(),
        runtime_read_write: Vec::new(),
    });
    std::fs::create_dir_all(layout.workspace.join("nested-state")).unwrap();

    let overlap_error = supervisor.prepare(&overlap).unwrap_err();

    assert!(matches!(
        overlap_error,
        ExecutionError::UnsafeConfinement("workspace and writable state must not overlap")
    ));
    assert!(!marker.exists());

    let mut root = confined_spec(&layout, format!("printf ran > {}", marker.display()));
    root.confinement = ProcessConfinement::LinuxFilesystem(LinuxFilesystemConfinement {
        workspace: PathBuf::from("/"),
        writable_state: layout.state.clone(),
        runtime_read_execute: runtime_read_execute_roots(),
        runtime_read_only: Vec::new(),
        runtime_read_write: Vec::new(),
    });

    let root_error = supervisor.prepare(&root).unwrap_err();

    assert!(matches!(
        root_error,
        ExecutionError::UnsafeConfinement("filesystem root cannot be granted")
    ));
    assert!(!marker.exists());
}

#[test]
fn metadata_reports_landlock_and_policy_changes_command_identity() {
    let first = TestLayout::new();
    let second = TestLayout::new();
    let supervisor = ProcessSupervisor;
    let first_spec = confined_spec(&first, "exit 0");
    let second_spec = confined_spec(&second, "exit 0");

    let (first_prepared, first_metadata) = supervisor.prepare(&first_spec).unwrap();
    let (second_prepared, second_metadata) = supervisor.prepare(&second_spec).unwrap();

    let FilesystemConfinementReport::Landlock { policy_sha256, abi } =
        &first_metadata.confinement.filesystem
    else {
        panic!("expected Landlock confinement report");
    };
    assert_eq!(policy_sha256.len(), 64);
    assert_eq!(*abi, 3);
    assert_eq!(
        first_metadata.confinement.network,
        NetworkConfinementReport::NotOsConfined
    );
    assert_ne!(
        first_metadata.command_digest,
        second_metadata.command_digest
    );
    drop(first_prepared);
    drop(second_prepared);
}
