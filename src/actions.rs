//! Fixed personal actions for P1; no config framework until scenes need one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Action {
    Test = 0,
    Teams,
    Browser,
    ChatGpt,
    Maximize,
    PairLeft,
    PairRight,
}
impl Action {
    pub const ALL: [Self; 7] = [
        Self::Test,
        Self::Teams,
        Self::Browser,
        Self::ChatGpt,
        Self::Maximize,
        Self::PairLeft,
        Self::PairRight,
    ];
    #[cfg(test)]
    pub fn selector(self) -> &'static str {
        match self {
            Self::Test => "testWindow:",
            Self::Teams => "focusTeams:",
            Self::Browser => "focusBrowser:",
            Self::ChatGpt => "openChatGpt:",
            Self::Maximize => "maximizeWindow:",
            Self::PairLeft => "pairLeft:",
            Self::PairRight => "pairRight:",
        }
    }
    #[cfg(test)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Test => "⌃⌥T — test / restauration",
            Self::Teams => "⌥T — Teams",
            Self::Browser => "⌥B — Chrome à 100 %",
            Self::ChatGpt => "⌥C — retrouver / ouvrir ChatGPT",
            Self::Maximize => "⌘⇧↑ — maximiser",
            Self::PairLeft => "⌘⇧← — choisir gauche puis droite",
            Self::PairRight => "⌘⇧→ — choisir droite puis gauche",
        }
    }
    pub fn binding(self) -> (u32, u32) {
        let option = 1 << 11;
        let command_shift = (1 << 8) | (1 << 9);
        // Carbon virtual key codes refer to physical positions on macOS keyboards.
        match self {
            Self::Test => (17, option | (1 << 12)),
            Self::Teams => (17, option),
            Self::Browser => (11, option),
            Self::ChatGpt => (8, option),
            Self::Maximize => (126, command_shift),
            Self::PairLeft => (123, command_shift),
            Self::PairRight => (124, command_shift),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationState {Waiting, Active, Cancelled}
#[derive(Default)]
pub struct ActivationGuard {pub origin:Option<i32>, seen_target:bool}
impl ActivationGuard {
    pub fn new(origin:Option<i32>)->Self {Self{origin,seen_target:false}}
    pub fn observe(&mut self,frontmost:Option<i32>,target:i32,own:i32)->ActivationState {
        if frontmost==Some(target) {self.seen_target=true;return ActivationState::Active;}
        if frontmost.is_none(){return ActivationState::Waiting;}
        if self.seen_target {return ActivationState::Cancelled;}
        if self.origin.is_none(){self.origin=frontmost;}
        match frontmost {
            None=>ActivationState::Waiting,
            Some(pid) if Some(pid)==self.origin || pid==own=>ActivationState::Waiting,
            _=>ActivationState::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_waits_for_a_space_transition_but_never_fights_a_user_switch() {
        let mut guard=ActivationGuard::new(Some(10));
        assert_eq!(guard.observe(Some(10),20,30),ActivationState::Waiting);
        assert_eq!(guard.observe(None,20,30),ActivationState::Waiting);
        assert_eq!(guard.observe(Some(20),20,30),ActivationState::Active);
        assert_eq!(guard.observe(None,20,30),ActivationState::Waiting);
        assert_eq!(guard.observe(Some(10),20,30),ActivationState::Cancelled);
        let mut guard=ActivationGuard::new(Some(10));
        assert_eq!(guard.observe(Some(40),20,30),ActivationState::Cancelled);
    }
    #[test]
    fn bindings_are_unique_and_dispatch_to_distinct_actions() {
        for (i, action) in Action::ALL.iter().enumerate() {
            assert!(!action.label().is_empty());
            for other in &Action::ALL[i + 1..] {
                assert_ne!(action.binding(), other.binding());
                assert_ne!(action.selector(), other.selector());
            }
        }
    }
}
