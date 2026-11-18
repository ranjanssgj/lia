//! Authorization boundary for typed presentation intents.
//!
//! This module is deliberately separate from IPC framing. The transport can
//! produce only [`UntrustedUserIntent`]; a trusted UI registry and the current
//! deterministic policy must both accept an intent before it becomes an
//! [`AuthorizedPresentationIntent`].
//!
//! Architectural Invariants:
//! 1. [`AuthorizedPresentationIntent`] is strictly a validated presentation intent,
//!    never an OS execution permit or tool authorization grant.
//! 2. Downstream OS and desktop operations must be authorized by deterministic
//!    authorities and executed by the Rust device agent, never by the renderer.
//! 3. The presentation process (Godot) owns rendering and user event dispatch only.
//!    It cannot claim operation outcomes or manufacture authorization.
//! 4. Unrecognized controls, mismatched event kinds, and unauthorized actions fail closed.

use crate::ipc::{UntrustedUserIntent, UserEventKind};
use std::collections::HashMap;
use std::fmt;

/// Risk tiers defined by canonical security and policy contracts (R0 least consequential to R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskTier {
    /// Non-consequential UI control (e.g. toggle pin, toggle lock, hide panel, pause/resume).
    R0,
    /// Interactive conversation input (e.g. text submission).
    R1,
    /// Direct OS observation or safe query.
    R2,
    /// Reversible OS modification requiring authorization.
    R3,
    /// Consequential or destructive operation requiring explicit consent.
    R4,
}

impl fmt::Display for RiskTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::R0 => write!(f, "R0"),
            Self::R1 => write!(f, "R1"),
            Self::R2 => write!(f, "R2"),
            Self::R3 => write!(f, "R3"),
            Self::R4 => write!(f, "R4"),
        }
    }
}

/// Metadata and validation rules for an allowlisted UI presentation control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiControlDef {
    pub control_id: String,
    pub action_key: String,
    pub allowed_events: Vec<UserEventKind>,
    pub risk_tier: RiskTier,
    pub requires_active_session: bool,
    pub max_text_bytes: Option<usize>,
}

/// Resolves a UI-owned control identifier through an agent-owned registry.
/// Implementations must derive this mapping from the active renderer profile,
/// never from data supplied in the IPC frame beyond its control identifier.
pub trait UiActionRegistry {
    fn resolve_control(&self, control_id: &str) -> Option<String>;

    fn control_definition(&self, _control_id: &str) -> Option<&UiControlDef> {
        None
    }
}

/// Trusted UI control registry for production presentation controls.
/// Pre-populated with canonical AC-010 controls (hide, pin, lock, reposition, pause, resume)
/// and conversational interaction controls (input, stop/interrupt).
#[derive(Clone, Debug, Default)]
pub struct ProductionUiRegistry {
    controls: HashMap<String, UiControlDef>,
}

impl ProductionUiRegistry {
    /// Creates a production UI registry populated with canonical AC-010 controls.
    pub fn new() -> Self {
        let mut registry = Self {
            controls: HashMap::new(),
        };

        // AC-010: hide, pin, lock, reposition, pause, resume
        registry.register(UiControlDef {
            control_id: "panel.hide".into(),
            action_key: "ui.panel.hide".into(),
            allowed_events: vec![UserEventKind::Clicked, UserEventKind::Dismissed],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry.register(UiControlDef {
            control_id: "panel.pin".into(),
            action_key: "ui.panel.pin".into(),
            allowed_events: vec![UserEventKind::Clicked],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry.register(UiControlDef {
            control_id: "panel.lock".into(),
            action_key: "ui.panel.lock".into(),
            allowed_events: vec![UserEventKind::Clicked],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry.register(UiControlDef {
            control_id: "panel.reposition".into(),
            action_key: "ui.panel.reposition".into(),
            allowed_events: vec![UserEventKind::Clicked],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry.register(UiControlDef {
            control_id: "presentation.pause".into(),
            action_key: "ui.presentation.pause".into(),
            allowed_events: vec![UserEventKind::Clicked],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry.register(UiControlDef {
            control_id: "presentation.resume".into(),
            action_key: "ui.presentation.resume".into(),
            allowed_events: vec![UserEventKind::Clicked],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        // Conversational interaction and stop controls
        registry.register(UiControlDef {
            control_id: "chat.input".into(),
            action_key: "ui.chat.submit".into(),
            allowed_events: vec![UserEventKind::TextSubmitted],
            risk_tier: RiskTier::R1,
            requires_active_session: true,
            max_text_bytes: Some(8_192),
        });

        registry.register(UiControlDef {
            control_id: "chat.stop".into(),
            action_key: "ui.chat.interrupt".into(),
            allowed_events: vec![UserEventKind::Clicked, UserEventKind::InterruptionRequested],
            risk_tier: RiskTier::R0,
            requires_active_session: true,
            max_text_bytes: None,
        });

        registry
    }

    /// Registers or overrides a UI control definition.
    pub fn register(&mut self, def: UiControlDef) {
        self.controls.insert(def.control_id.clone(), def);
    }

    /// Retrieves the control definition by control ID.
    pub fn get(&self, control_id: &str) -> Option<&UiControlDef> {
        self.controls.get(control_id)
    }

    /// Number of registered controls.
    pub fn len(&self) -> usize {
        self.controls.len()
    }

    /// Returns true if no controls are registered.
    pub fn is_empty(&self) -> bool {
        self.controls.is_empty()
    }
}

impl UiActionRegistry for ProductionUiRegistry {
    fn resolve_control(&self, control_id: &str) -> Option<String> {
        self.controls
            .get(control_id)
            .map(|def| def.action_key.clone())
    }

    fn control_definition(&self, control_id: &str) -> Option<&UiControlDef> {
        self.controls.get(control_id)
    }
}

/// Evaluates the current deterministic policy for a registry-resolved intent.
/// Implementations own permission, resource, session, and action policy checks.
pub trait PresentationPolicy {
    fn authorize(&self, action_key: &str, intent: &UntrustedUserIntent) -> bool;
}

/// Permission authority trait evaluating whether an action key and risk tier are permitted.
pub trait PermissionAuthority {
    fn has_permission(&self, action_key: &str, risk_tier: RiskTier) -> bool;
}

/// Deterministic presentation policy evaluator combining permission authority
/// and session constraints.
#[derive(Clone, Debug)]
pub struct DeterministicPresentationPolicy<A: PermissionAuthority> {
    authority: A,
    expected_session_id: Option<String>,
}

impl<A: PermissionAuthority> DeterministicPresentationPolicy<A> {
    pub fn new(authority: A) -> Self {
        Self {
            authority,
            expected_session_id: None,
        }
    }

    pub fn with_expected_session(authority: A, session_id: impl Into<String>) -> Self {
        Self {
            authority,
            expected_session_id: Some(session_id.into()),
        }
    }

    pub fn authority(&self) -> &A {
        &self.authority
    }
}

impl<A: PermissionAuthority> PresentationPolicy for DeterministicPresentationPolicy<A> {
    fn authorize(&self, action_key: &str, intent: &UntrustedUserIntent) -> bool {
        if let Some(expected_session) = &self.expected_session_id
            && intent.session_id != *expected_session
        {
            return false;
        }
        // Base authorization against R1 ceiling for presentation layer (UI cannot authorize R2+)
        self.authority.has_permission(action_key, RiskTier::R0)
            || self.authority.has_permission(action_key, RiskTier::R1)
    }
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
    risk_tier: RiskTier,
}

impl AuthorizedPresentationIntent {
    pub fn action_key(&self) -> &str {
        &self.action_key
    }

    pub fn intent(&self) -> &UntrustedUserIntent {
        &self.intent
    }

    pub fn risk_tier(&self) -> RiskTier {
        self.risk_tier
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    UnknownControl,
    InvalidEventKind,
    InvalidPayload,
    InvalidSession,
    PolicyDenied,
}

impl fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownControl => write!(
                f,
                "control_id is not allowlisted in the trusted UI registry"
            ),
            Self::InvalidEventKind => write!(f, "event kind is not permitted for this UI control"),
            Self::InvalidPayload => {
                write!(f, "intent payload exceeds bounds or violates control rules")
            }
            Self::InvalidSession => write!(f, "intent session is missing or invalid"),
            Self::PolicyDenied => write!(f, "deterministic presentation policy denied the action"),
        }
    }
}

impl std::error::Error for AuthorizationError {}

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
    let (action_key, risk_tier) = match registry.control_definition(&intent.body.control_id) {
        Some(def) => {
            if !def.allowed_events.contains(&intent.body.event) {
                return Err(AuthorizationError::InvalidEventKind);
            }
            if def.requires_active_session && intent.session_id.is_empty() {
                return Err(AuthorizationError::InvalidSession);
            }
            if let Some(max_bytes) = def.max_text_bytes
                && let Some(text) = &intent.body.text
                && text.len() > max_bytes
            {
                return Err(AuthorizationError::InvalidPayload);
            }
            (def.action_key.clone(), def.risk_tier)
        }
        None => {
            let action = registry
                .resolve_control(&intent.body.control_id)
                .ok_or(AuthorizationError::UnknownControl)?;
            (action, RiskTier::R0)
        }
    };

    if !policy.authorize(&action_key, &intent) {
        return Err(AuthorizationError::PolicyDenied);
    }

    Ok(AuthorizedPresentationIntent {
        action_key,
        intent,
        risk_tier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{UntrustedUserIntent, UserEventBody, UserEventKind};
    use std::cell::Cell;
    use std::collections::HashSet;

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
        assert_eq!(authorized.risk_tier(), RiskTier::R0);
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

    #[test]
    fn production_ui_registry_resolves_all_canonical_ac010_controls() {
        let reg = ProductionUiRegistry::new();
        assert_eq!(reg.len(), 8);

        let canonical_controls = [
            ("panel.hide", "ui.panel.hide", RiskTier::R0),
            ("panel.pin", "ui.panel.pin", RiskTier::R0),
            ("panel.lock", "ui.panel.lock", RiskTier::R0),
            ("panel.reposition", "ui.panel.reposition", RiskTier::R0),
            ("presentation.pause", "ui.presentation.pause", RiskTier::R0),
            (
                "presentation.resume",
                "ui.presentation.resume",
                RiskTier::R0,
            ),
            ("chat.input", "ui.chat.submit", RiskTier::R1),
            ("chat.stop", "ui.chat.interrupt", RiskTier::R0),
        ];

        for (control_id, expected_action, expected_tier) in canonical_controls {
            let def = reg.get(control_id).expect("canonical control should exist");
            assert_eq!(def.action_key, expected_action);
            assert_eq!(def.risk_tier, expected_tier);
            assert!(def.requires_active_session);
            assert_eq!(
                reg.resolve_control(control_id),
                Some(expected_action.into())
            );
        }
    }

    #[test]
    fn event_kind_mismatch_fails_closed_before_policy() {
        let reg = ProductionUiRegistry::new();
        let policy = TestPolicy {
            allowed: true,
            calls: Cell::new(0),
        };

        // Clicked sent to chat.input (which only allows TextSubmitted)
        let clicked_chat = intent("chat.input");
        assert_eq!(
            authorize_user_intent(clicked_chat, &reg, &policy),
            Err(AuthorizationError::InvalidEventKind)
        );
        assert_eq!(policy.calls.get(), 0);

        // TextSubmitted sent to panel.pin (which only allows Clicked)
        let mut text_pin = intent("panel.pin");
        text_pin.body.event = UserEventKind::TextSubmitted;
        text_pin.body.text = Some("pin-command".into());
        assert_eq!(
            authorize_user_intent(text_pin, &reg, &policy),
            Err(AuthorizationError::InvalidEventKind)
        );
        assert_eq!(policy.calls.get(), 0);
    }

    #[test]
    fn chat_input_text_submission_validates_and_authorizes() {
        let reg = ProductionUiRegistry::new();
        let policy = TestPolicy {
            allowed: true,
            calls: Cell::new(0),
        };

        let mut chat_intent = intent("chat.input");
        chat_intent.body.event = UserEventKind::TextSubmitted;
        chat_intent.body.text = Some("Hello LIA".into());

        let authorized = authorize_user_intent(chat_intent, &reg, &policy).unwrap();
        assert_eq!(authorized.action_key(), "ui.chat.submit");
        assert_eq!(authorized.risk_tier(), RiskTier::R1);
        assert_eq!(authorized.intent().body.text.as_deref(), Some("Hello LIA"));
        assert_eq!(policy.calls.get(), 1);
    }

    #[test]
    fn missing_session_fails_closed() {
        let reg = ProductionUiRegistry::new();
        let policy = TestPolicy {
            allowed: true,
            calls: Cell::new(0),
        };

        let mut no_session = intent("panel.hide");
        no_session.session_id = "".into();

        assert_eq!(
            authorize_user_intent(no_session, &reg, &policy),
            Err(AuthorizationError::InvalidSession)
        );
        assert_eq!(policy.calls.get(), 0);
    }

    struct MockPermissionAuthority {
        allowed_actions: HashSet<String>,
        max_tier: RiskTier,
    }

    impl PermissionAuthority for MockPermissionAuthority {
        fn has_permission(&self, action_key: &str, risk_tier: RiskTier) -> bool {
            risk_tier <= self.max_tier && self.allowed_actions.contains(action_key)
        }
    }

    #[test]
    fn deterministic_presentation_policy_evaluates_authority_and_session() {
        let session = "00000000-0000-4000-8000-000000000001";
        let authority = MockPermissionAuthority {
            allowed_actions: HashSet::from(["ui.panel.hide".into(), "ui.chat.submit".into()]),
            max_tier: RiskTier::R1,
        };
        let policy = DeterministicPresentationPolicy::with_expected_session(authority, session);
        let reg = ProductionUiRegistry::new();

        // Allowed panel.hide
        let authorized = authorize_user_intent(intent("panel.hide"), &reg, &policy).unwrap();
        assert_eq!(authorized.action_key(), "ui.panel.hide");

        // Stale / mismatched session rejected
        let mut wrong_session = intent("panel.hide");
        wrong_session.session_id = "00000000-0000-4000-8000-000000000002".into();
        assert_eq!(
            authorize_user_intent(wrong_session, &reg, &policy),
            Err(AuthorizationError::PolicyDenied)
        );

        // Action not permitted in authority
        assert_eq!(
            authorize_user_intent(intent("panel.pin"), &reg, &policy),
            Err(AuthorizationError::PolicyDenied)
        );
    }
}
