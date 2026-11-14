//! Review-first Workflow Studio presentation boundary for Spec 063.
//!
//! Desktop maps existing durable outbox/effect-state contracts into a native
//! review surface. It does not create a second task/message/action authority and
//! never turns an `Unknown` effect into an implicit retry.

use medscale_contracts::actions::OutboxEntry;
use medscale_contracts::objects::{DigestSha256, EffectState, OpaqueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepVm {
    pub order: String,
    pub title: String,
    pub detail: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTaskVm {
    pub action_id: String,
    pub title: String,
    pub state: String,
    pub payload_digest: String,
    pub next_step: String,
    pub reconcile_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePreviewVm {
    pub title: String,
    pub body: String,
    pub status: String,
    pub related_action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStudioVm {
    pub synthetic_only: bool,
    pub workflow_name: String,
    pub workflow_state: String,
    pub outbox_summary: String,
    pub action_boundary: String,
    pub task_boundary: String,
    pub message_boundary: String,
    pub steps: Vec<WorkflowStepVm>,
    pub tasks: Vec<ReviewTaskVm>,
    pub messages: Vec<MessagePreviewVm>,
}

impl WorkflowStudioVm {
    #[must_use]
    pub fn from_outbox(entries: &[OutboxEntry]) -> Self {
        let pending = entries
            .iter()
            .filter(|entry| entry.effect_state == EffectState::Pending)
            .count();
        let sent = entries
            .iter()
            .filter(|entry| entry.effect_state == EffectState::Sent)
            .count();
        let confirmed = entries
            .iter()
            .filter(|entry| entry.effect_state == EffectState::Confirmed)
            .count();
        let failed = entries
            .iter()
            .filter(|entry| entry.effect_state == EffectState::Failed)
            .count();
        let unknown = entries
            .iter()
            .filter(|entry| entry.effect_state == EffectState::Unknown)
            .count();

        let tasks = entries.iter().map(task_from_outbox).collect::<Vec<_>>();
        let messages = entries.iter().map(message_from_outbox).collect::<Vec<_>>();

        Self {
            synthetic_only: true,
            workflow_name: "Evidence review → controlled action".to_owned(),
            workflow_state: if unknown > 0 {
                "Blocked by UNKNOWN effect — reconcile before any retry".to_owned()
            } else if failed > 0 {
                "Review failed effects before explicit restart".to_owned()
            } else {
                "Review-first synthetic workflow preview".to_owned()
            },
            outbox_summary: format!(
                "{pending} pending · {sent} sent · {confirmed} confirmed · {failed} failed · {unknown} unknown"
            ),
            action_boundary: "Studio is a read-only/review presentation over existing OutboxEntry + EffectState semantics. Creating or committing an external action remains owned by the Core Host capability/session boundary."
                .to_owned(),
            task_boundary: "Tasks are derived review cues over durable outbox intents; Spec 063 does not invent a second canonical task store."
                .to_owned(),
            message_boundary: "Messages are local review previews only. No external messaging transport, delivery receipt, or clinical communication authority is claimed."
                .to_owned(),
            steps: workflow_steps(),
            tasks,
            messages,
        }
    }

    #[must_use]
    pub fn review_detail(&self, action_id: &str) -> String {
        self.tasks
            .iter()
            .find(|task| task.action_id == action_id)
            .map(|task| {
                format!(
                    "{} · {} · {}. Review only; no action was committed by Desktop.",
                    task.title, task.state, task.next_step
                )
            })
            .unwrap_or_else(|| {
                "Unknown review item. No action was committed by Desktop.".to_owned()
            })
    }

    #[must_use]
    pub fn synthetic_demo() -> Self {
        Self::from_outbox(&synthetic_outbox())
    }
}

fn workflow_steps() -> Vec<WorkflowStepVm> {
    vec![
        WorkflowStepVm {
            order: "01".to_owned(),
            title: "Trusted evidence".to_owned(),
            detail: "Inspect source-linked presentation and unresolved coverage before preparing consequential work."
                .to_owned(),
            state: "Read-only".to_owned(),
        },
        WorkflowStepVm {
            order: "02".to_owned(),
            title: "Explicit review".to_owned(),
            detail: "A human/operator review boundary precedes any durable external-action intent."
                .to_owned(),
            state: "Required".to_owned(),
        },
        WorkflowStepVm {
            order: "03".to_owned(),
            title: "Payload identity".to_owned(),
            detail: "Approved payload identity is bound by DigestSha256 before send-state transitions."
                .to_owned(),
            state: "Bound".to_owned(),
        },
        WorkflowStepVm {
            order: "04".to_owned(),
            title: "Durable outbox".to_owned(),
            detail: "Core Host owns the external-action intent and durable effect-state projection."
                .to_owned(),
            state: "Core Host".to_owned(),
        },
        WorkflowStepVm {
            order: "05".to_owned(),
            title: "Reconcile UNKNOWN".to_owned(),
            detail: "UNKNOWN never retries blindly; reconciliation evidence is required before transition."
                .to_owned(),
            state: "Fail closed".to_owned(),
        },
    ]
}

fn task_from_outbox(entry: &OutboxEntry) -> ReviewTaskVm {
    let (state, next_step, reconcile_required) = match entry.effect_state {
        EffectState::Pending => (
            "Pending".to_owned(),
            "Review the exact payload identity before an authorized send transition.".to_owned(),
            false,
        ),
        EffectState::Sent => (
            "Sent".to_owned(),
            "Await confirmation/failure/unknown evidence; do not duplicate-send.".to_owned(),
            false,
        ),
        EffectState::Confirmed => (
            "Confirmed".to_owned(),
            "No retry; retain the confirmed effect as history.".to_owned(),
            false,
        ),
        EffectState::Failed => (
            "Failed".to_owned(),
            "Review failure evidence before an explicit restart.".to_owned(),
            false,
        ),
        EffectState::Unknown => (
            "UNKNOWN".to_owned(),
            "Reconcile first. Blind retry is forbidden.".to_owned(),
            true,
        ),
    };
    ReviewTaskVm {
        action_id: entry.action_id.as_str().to_owned(),
        title: format!("Review {}", entry.action),
        state,
        payload_digest: entry.payload_digest.to_hex(),
        next_step,
        reconcile_required,
    }
}

fn message_from_outbox(entry: &OutboxEntry) -> MessagePreviewVm {
    let (title, body, status) = match entry.effect_state {
        EffectState::Pending => (
            "Action awaiting review",
            "A durable intent exists, but Desktop has not sent anything.",
            "Local preview",
        ),
        EffectState::Sent => (
            "Awaiting effect confirmation",
            "The effect is marked Sent. Duplicate delivery must not be inferred or retried.",
            "Local status",
        ),
        EffectState::Confirmed => (
            "Effect confirmed",
            "Confirmation is visible as history; this preview is not a transport receipt.",
            "Local status",
        ),
        EffectState::Failed => (
            "Effect failed",
            "Failure requires review before an explicit restart path.",
            "Review required",
        ),
        EffectState::Unknown => (
            "Effect state UNKNOWN",
            "Reconciliation is required before any retry. No external message was sent from this UI.",
            "Reconcile first",
        ),
    };
    MessagePreviewVm {
        title: title.to_owned(),
        body: body.to_owned(),
        status: status.to_owned(),
        related_action_id: entry.action_id.as_str().to_owned(),
    }
}

fn synthetic_outbox() -> Vec<OutboxEntry> {
    [
        (
            "action-demo-pending",
            "prepare-care-plan-share",
            EffectState::Pending,
        ),
        ("action-demo-sent", "send-follow-up", EffectState::Sent),
        (
            "action-demo-confirmed",
            "record-export-disclosure",
            EffectState::Confirmed,
        ),
        ("action-demo-failed", "send-reminder", EffectState::Failed),
        (
            "action-demo-unknown",
            "external-referral",
            EffectState::Unknown,
        ),
    ]
    .into_iter()
    .map(|(id, action, effect_state)| OutboxEntry {
        action_id: OpaqueId::new(id),
        action: action.to_owned(),
        effect_state,
        payload_digest: DigestSha256::of(format!("synthetic-payload:{id}").as_bytes()),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_studio_maps_all_effect_states_without_blind_retry() {
        let vm = WorkflowStudioVm::synthetic_demo();
        assert_eq!(vm.tasks.len(), 5);
        assert!(vm.outbox_summary.contains("1 unknown"));
        let unknown = vm
            .tasks
            .iter()
            .find(|task| task.state == "UNKNOWN")
            .expect("unknown task");
        assert!(unknown.reconcile_required);
        assert!(unknown.next_step.contains("Blind retry is forbidden"));
        assert!(vm.workflow_state.contains("reconcile"));
    }

    #[test]
    fn tasks_and_messages_are_explicitly_non_authoritative_views() {
        let vm = WorkflowStudioVm::synthetic_demo();
        assert!(
            vm.task_boundary
                .contains("does not invent a second canonical task store")
        );
        assert!(vm.message_boundary.contains("local review previews only"));
        assert!(
            vm.action_boundary
                .contains("Core Host capability/session boundary")
        );
    }

    #[test]
    fn review_detail_never_claims_commit() {
        let vm = WorkflowStudioVm::synthetic_demo();
        let detail = vm.review_detail("action-demo-pending");
        assert!(detail.contains("no action was committed by Desktop"));
        let missing = vm.review_detail("missing");
        assert!(missing.contains("No action was committed by Desktop"));
    }
}
