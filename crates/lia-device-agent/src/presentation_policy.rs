//! Authorization boundary for typed presentation intents.
//!
//! This module is deliberately separate from IPC framing. The transport can
//! produce only [`UntrustedUserIntent`]; a trusted UI registry and the current
//! deterministic policy must both accept an intent before it becomes an
//! [`AuthorizedPresentationIntent`]. That type is not an OS execution permit.

use crate::ipc::UntrustedUserIntent;

/// Resolves a UI-owned control identifier through an agent-owned registry.
/// Implementations must derive this mapping from the active renderer profile,
/// never from data supplied in the IPC frame beyond its control identifier.
pub trait UiActionRegistry {
    fn resolve_control(&self, control_id: &str) -> Option<String>;
}

/// Evaluates the current deterministic policy for a registry-resolved intent.
/// Implementations own permission, resource, session, and action policy checks.
pub trait PresentationPolicy {
    fn authorize(&self, action_key: &str, intent: &UntrustedUserIntent) -> bool;
}

#[must_use = "authorized intents must be handled explicitly"]
/// An intent that passed registry resolution and the current presentation
/// policy. It does not authorize or perform a desktop/OS operation; downstream
/// execution must still consult the authoritative capability and permission
/// boundary.
#[derive(Debug, PartialEq)]
pub struct AuthorizedPresentationIntent {
    action_key: String,
    intent: UntrustedUserIntent,
}

impl AuthorizedPresentationIntent {
    pub fn action_key(&self) -> &str {
        &self.action_key
    }

    pub fn intent(&self) -> &UntrustedUserIntent {
        &self.intent
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    UnknownControl,
    PolicyDenied,
}

/// Resolve a typed IPC intent and evaluate current policy without dispatching
/// an action. Registry and policy implementations are supplied by trusted Rust
/// authority code; the renderer cannot provide either implementation.
pub fn authorize_user_intent<R, P>(
    intent: UntrustedUserIntent,
    registry: &R,
    policy: &P,
) -> Result<AuthorizedPresentationIntent, AuthorizationError>
where
    R: UiActionRegistry,
    P: PresentationPolicy,
{
    let action_key = registry
        .resolve_control(&intent.body.control_id)
        .ok_or(AuthorizationError::UnknownControl)?;
    if !policy.authorize(&action_key, &intent) {
        return Err(AuthorizationError::PolicyDenied);
    }
    Ok(AuthorizedPresentationIntent { action_key, intent })
}

#[cfg(test)]
mod tests {
    use super::{AuthorizationError, PresentationPolicy, UiActionRegistry, authorize_user_intent};
    use crate::ipc::{UntrustedUserIntent, UserEventBody, UserEventKind};
    use std::cell::Cell;
    use std::collections::HashMap;

    struct TestRegistry(HashMap<String, String>);

    impl UiActionRegistry for TestRegistry {
        fn resolve_control(&self, control_id: &str) -> Option<String> {
            self.0.get(control_id).cloned()
        }
    }

    struct TestPolicy {
        allowed: bool,
        calls: Cell<usize>,
    }

    impl PresentationPolicy for TestPolicy {
        fn authorize(&self, _action_key: &str, _intent: &UntrustedUserIntent) -> bool {
            self.calls.set(self.calls.get() + 1);
            self.allowed
        }
    }

    fn intent(control_id: &str) -> UntrustedUserIntent {
        UntrustedUserIntent {
            session_id: "00000000-0000-4000-8000-000000000001".into(),
            sequence: 0,
            body: UserEventBody {
                control_id: control_id.into(),
                event: UserEventKind::Clicked,
                text: None,
            },
        }
    }

    fn registry() -> TestRegistry {
        TestRegistry(HashMap::from([(
            "synthetic.control".into(),
            "synthetic.action".into(),
        )]))
    }

    #[test]
    fn registry_resolution_and_policy_are_both_required() {
        let policy = TestPolicy {
            allowed: true,
            calls: Cell::new(0),
        };
        let authorized = authorize_user_intent(intent("synthetic.control"), &registry(), &policy)
            .expect("registered, policy-approved intent should pass the boundary");

        assert_eq!(authorized.action_key(), "synthetic.action");
        assert_eq!(authorized.intent().body.control_id, "synthetic.control");
        assert_eq!(policy.calls.get(), 1);
    }

    #[test]
    fn unknown_control_is_rejected_before_policy_evaluation() {
        let policy = TestPolicy {
            allowed: true,
            calls: Cell::new(0),
        };

        assert_eq!(
            authorize_user_intent(intent("renderer.chosen.action"), &registry(), &policy),
            Err(AuthorizationError::UnknownControl)
        );
        assert_eq!(policy.calls.get(), 0);
    }

    #[test]
    fn registry_match_cannot_override_policy_denial() {
        let policy = TestPolicy {
            allowed: false,
            calls: Cell::new(0),
        };

        assert_eq!(
            authorize_user_intent(intent("synthetic.control"), &registry(), &policy),
            Err(AuthorizationError::PolicyDenied)
        );
        assert_eq!(policy.calls.get(), 1);
    }
}
