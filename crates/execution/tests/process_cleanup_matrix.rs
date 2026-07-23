use execution::{
    CancellationToken, FilesystemConfinementReport, NetworkConfinementReport, ProcessConfinement,
    ProcessConfinementReport, ProcessSpec, ProcessStartIdentity, ProcessSupervisor,
    SupervisionState, TerminalOutcome, TerminationStatus, supervise,
};
use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};

fn spec(_seconds: u64) -> ProcessSpec {
    ProcessSpec {
        program: "/bin/sh".into(),
        arguments: vec!["-c".into(), "while :; do :; done".into()],
        environment: HashMap::new(),
        sensitive_environment_keys: Vec::new(),
        current_dir: None,
        timeout: Duration::from_millis(200),
        output_limit: 32,
        confinement: ProcessConfinement::Unconfined,
    }
}

#[test]
fn timeout_and_cancellation_have_one_terminal_outcome() {
    let cancellation = CancellationToken::default();
    let result = supervise(&spec(5), &cancellation).unwrap();
    assert_eq!(result.outcome, TerminalOutcome::TimedOut);
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let result = supervise(&spec(5), &cancellation).unwrap();
    assert_eq!(result.outcome, TerminalOutcome::Cancelled);
    let cancellation = CancellationToken::default();
    let worker_token = cancellation.clone();
    let worker = std::thread::spawn(move || supervise(&spec(5), &worker_token).unwrap());
    std::thread::sleep(Duration::from_millis(30));
    cancellation.cancel();
    assert_eq!(worker.join().unwrap().outcome, TerminalOutcome::Cancelled);
}

#[test]
fn output_is_bounded() {
    let mut command = spec(0);
    command.arguments = vec!["-c".into(), "printf '1234567890abcdef'".into()];
    command.output_limit = 4;
    let result = supervise(&command, &CancellationToken::default()).unwrap();
    assert_eq!(result.stdout, b"1234");
    assert!(result.stdout_truncated);
}

#[test]
fn non_zero_exit_is_failed() {
    let mut command = spec(0);
    command.arguments = vec!["-c".into(), "exit 7".into()];
    assert_eq!(
        supervise(&command, &CancellationToken::default())
            .unwrap()
            .outcome,
        TerminalOutcome::Failed
    );
}

#[test]
fn descendant_processes_are_killed_with_the_owned_group() {
    let directory = tempfile::tempdir().unwrap();
    let pid_file = directory.path().join("child.pid");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!(
            "/bin/sh -c 'trap \\\"\\\" TERM; while :; do :; done' & echo $! > {}; wait",
            pid_file.display()
        ),
    ];
    command.timeout = Duration::from_millis(100);
    let result = supervise(&command, &CancellationToken::default()).unwrap();
    assert_eq!(result.outcome, TerminalOutcome::TimedOut);
    let child_pid: i32 = std::fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    for _ in 0..100 {
        let alive = unsafe { libc::kill(child_pid, 0) == 0 };
        if !alive {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("descendant process survived process-group cleanup");
}

#[test]
fn daemonized_descendants_are_killed_by_supervision_token() {
    let setsid = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("setsid"))
        .find(|path| path.is_file())
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let pid_file = directory.path().join("daemon.pid");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!(
            "{} /bin/sh -c 'trap \\\"\\\" TERM; while :; do :; done' & echo $! > {}; exit 0",
            setsid.display(),
            pid_file.display()
        ),
    ];
    assert_eq!(
        supervise(&command, &CancellationToken::default())
            .unwrap()
            .outcome,
        TerminalOutcome::Completed(0)
    );
    let daemon_pid: i32 = std::fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    for _ in 0..100 {
        if unsafe { libc::kill(daemon_pid, 0) } != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("daemonized descendant survived token cleanup");
}

#[test]
fn prepared_child_cannot_act_before_release() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("released");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!("printf released > {}", marker.display()),
    ];
    let supervisor = ProcessSupervisor;

    let (prepared, metadata) = supervisor.prepare(&command).unwrap();
    assert_eq!(metadata.state, SupervisionState::Prepared);
    std::thread::sleep(Duration::from_millis(100));
    assert!(!marker.exists());

    let running = prepared.release().unwrap();
    assert_eq!(running.metadata().state, SupervisionState::Running);
    let result = supervisor
        .wait(running, &CancellationToken::default())
        .unwrap();
    assert_eq!(result.result.outcome, TerminalOutcome::Completed(0));
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "released");
    assert_eq!(result.metadata.state, SupervisionState::Terminated);
}

#[test]
fn dropping_prepared_child_prevents_execution() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("must-not-exist");
    let mut command = spec(0);
    command.arguments = vec!["-c".into(), format!("touch {}", marker.display())];
    let supervisor = ProcessSupervisor;

    let (prepared, _) = supervisor.prepare(&command).unwrap();
    drop(prepared);
    std::thread::sleep(Duration::from_millis(100));

    assert!(!marker.exists());
}

#[test]
fn metadata_contains_digests_without_raw_arguments_or_environment() {
    let argument_secret = "argument-secret-739e";
    let environment_secret = "environment-secret-a14c";
    let mut command = spec(0);
    command.arguments = vec!["-c".into(), format!("printf %s {argument_secret}")];
    command
        .environment
        .insert("JET_BLACK_TEST_SECRET".into(), environment_secret.into());
    let supervisor = ProcessSupervisor;

    let (prepared, metadata) = supervisor.prepare(&command).unwrap();
    let serialized = serde_json::to_string(&metadata).unwrap();

    assert_eq!(metadata.command_digest.len(), 64);
    assert_eq!(metadata.environment_digest.len(), 64);
    assert!(!serialized.contains(argument_secret));
    assert!(!serialized.contains(environment_secret));
    assert!(!serialized.contains("JET_BLACK_TEST_SECRET"));
    assert!(metadata.executable.path.is_absolute());
    assert_eq!(
        metadata.confinement,
        ProcessConfinementReport {
            filesystem: FilesystemConfinementReport::Unconfined,
            network: NetworkConfinementReport::NotOsConfined,
        }
    );
    drop(prepared);
}

#[test]
fn released_process_tree_can_be_terminated_externally() {
    let setsid = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("setsid"))
        .find(|path| path.is_file())
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let group_pid_file = directory.path().join("group.pid");
    let daemon_pid_file = directory.path().join("daemon.pid");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!(
            "/bin/sh -c 'trap \"\" TERM; while :; do sleep 1; done' & echo $! > {}; {} /bin/sh -c 'trap \"\" TERM; while :; do sleep 1; done' & echo $! > {}; wait",
            group_pid_file.display(),
            setsid.display(),
            daemon_pid_file.display()
        ),
    ];
    command.timeout = Duration::from_secs(10);
    let supervisor = ProcessSupervisor;
    let (prepared, metadata) = supervisor.prepare(&command).unwrap();
    let running = prepared.release().unwrap();
    wait_for_file(&group_pid_file);
    wait_for_file(&daemon_pid_file);
    let group_pid = read_pid(&group_pid_file);
    let daemon_pid = read_pid(&daemon_pid_file);

    let termination = supervisor.terminate(&metadata).unwrap();

    assert_eq!(termination.status, TerminationStatus::Terminated);
    assert_process_gone(group_pid);
    assert_process_gone(daemon_pid);
    drop(running);
}

#[test]
fn exited_parent_still_cleans_token_owned_descendants() {
    let setsid = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("setsid"))
        .find(|path| path.is_file())
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let daemon_pid_file = directory.path().join("orphan.pid");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!(
            "{} /bin/sh -c 'trap \"\" TERM; while :; do :; done' & echo $! > {}; exit 0",
            setsid.display(),
            daemon_pid_file.display()
        ),
    ];
    command.timeout = Duration::from_secs(10);
    let supervisor = ProcessSupervisor;
    let (prepared, metadata) = supervisor.prepare(&command).unwrap();
    let running = prepared.release().unwrap();
    wait_for_file(&daemon_pid_file);
    let daemon_pid = read_pid(&daemon_pid_file);

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(metadata.pid as i32, &mut status, libc::WNOHANG) };
        if waited == metadata.pid as i32 {
            break;
        }
        assert!(Instant::now() < deadline, "parent process did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }

    let termination = supervisor.terminate(&metadata).unwrap();

    assert_eq!(termination.status, TerminationStatus::Terminated);
    assert_process_gone(daemon_pid);
    drop(running);
}

#[test]
fn stale_process_start_identity_refuses_to_signal() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("running");
    let mut command = spec(0);
    command.arguments = vec![
        "-c".into(),
        format!("printf running > {}; while :; do :; done", marker.display()),
    ];
    command.timeout = Duration::from_secs(10);
    let supervisor = ProcessSupervisor;
    let (prepared, metadata) = supervisor.prepare(&command).unwrap();
    let running = prepared.release().unwrap();
    wait_for_file(&marker);
    let mut stale_metadata = metadata.clone();
    stale_metadata.process_start = match &metadata.process_start {
        ProcessStartIdentity::Linux {
            boot_id,
            start_time_ticks,
        } => ProcessStartIdentity::Linux {
            boot_id: boot_id.clone(),
            start_time_ticks: start_time_ticks + 1,
        },
        ProcessStartIdentity::Platform { identity } => ProcessStartIdentity::Platform {
            identity: format!("{identity}-stale"),
        },
    };

    let refused = supervisor.terminate(&stale_metadata).unwrap();

    assert_eq!(refused.status, TerminationStatus::IdentityMismatch);
    assert!(!refused.term_signal_sent);
    assert!(!refused.kill_signal_sent);
    assert!(unsafe { libc::kill(metadata.pid as i32, 0) } == 0);

    assert_eq!(
        supervisor.terminate(&metadata).unwrap().status,
        TerminationStatus::Terminated
    );
    drop(running);
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {}", path.display());
}

fn read_pid(path: &Path) -> i32 {
    std::fs::read_to_string(path)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn assert_process_gone(pid: i32) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if unsafe { libc::kill(pid, 0) } != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("process {pid} survived cleanup");
}
