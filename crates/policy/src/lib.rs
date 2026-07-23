use domain::{Approval, ApprovalScope, DomainError, Id};

pub fn authorize_exact_action(
    approval: &Approval,
    run_id: Id,
    presented: &ApprovalScope,
    now_unix_ms: i64,
) -> Result<(), DomainError> {
    approval.validate_for_run(run_id, presented, now_unix_ms)
}
