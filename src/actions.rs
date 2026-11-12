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
#[cfg(test)]
mod tests {
    use super::*;
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
