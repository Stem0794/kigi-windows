//! Typed commands for the existing shortcut and device services.
use crate::shortcuts::Binding;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enabled {
    pub enabled: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Upsert {
    pub expected_revision: String,
    pub index: Option<usize>,
    pub binding: Binding,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Indexed {
    pub expected_revision: String,
    pub index: usize,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Delete {
    pub expected_revision: String,
    pub index: usize,
    pub confirmed: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reset {
    pub expected_revision: String,
    pub confirmed: bool,
}
#[derive(Debug, Deserialize)]
pub enum Section {
    Accessibility,
    InputMonitoring,
    Automation,
}
impl Section {
    /// Native-only allowlist; the bridge accepts enum values, never arbitrary URLs.
    pub fn settings_url(&self) -> &'static str {
        match self {
            Self::Accessibility => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            Self::InputMonitoring => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
            }
            Self::Automation => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Automation"
            }
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Accessibility => "Accessibilité",
            Self::InputMonitoring => "Surveillance de l’entrée",
            Self::Automation => "Automatisation",
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub section: Section,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeetingsSave {
    pub expected_revision: String,
    pub settings: crate::meetings::Settings,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingsProbe {
    pub settings: crate::meetings::Settings,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum SystemCommand {
    #[serde(rename = "runtime.setLoginEnabled")]
    Login(Enabled),
    #[serde(rename = "runtime.get")]
    Runtime(Empty),
    #[serde(rename = "runtime.save")]
    RuntimeSave(RuntimeSave),
    #[serde(rename = "meetings.apply")]
    MeetingsApply(Empty),
    #[serde(rename = "meetings.get")]
    Meetings(Empty),
    #[serde(rename = "meetings.save")]
    MeetingsSave(MeetingsSave),
    #[serde(rename = "meetings.identify")]
    MeetingsIdentify(Empty),
    #[serde(rename = "meetings.probe")]
    MeetingsProbe(MeetingsProbe),
    #[serde(rename = "meetings.pause")]
    MeetingsPause(Empty),
    #[serde(rename = "shortcuts.list")]
    ShortcutsList(Empty),
    #[serde(rename = "shortcuts.upsert")]
    Upsert(Upsert),
    #[serde(rename = "shortcuts.delete")]
    Delete(Delete),
    #[serde(rename = "shortcuts.reset")]
    Reset(Reset),
    #[serde(rename = "shortcuts.reload")]
    Reload(Empty),
    #[serde(rename = "shortcuts.setGlobalEnabled")]
    Global(Enabled),
    #[serde(rename = "shortcuts.reregister")]
    Reregister(Empty),
    #[serde(rename = "shortcuts.startCapture")]
    Capture(Empty),
    #[serde(rename = "shortcuts.cancelCapture")]
    CancelCapture(Empty),
    #[serde(rename = "shortcuts.test")]
    Test(Indexed),
    #[serde(rename = "gestures.get")]
    Gestures(Empty),
    #[serde(rename = "gestures.setEnabled")]
    GesturesEnable(Enabled),
    #[serde(rename = "gestures.startLocalProbe")]
    LocalProbe(Empty),
    #[serde(rename = "gestures.startGlobalProbe")]
    GlobalProbe(Empty),
    #[serde(rename = "gestures.stop")]
    StopGestures(Empty),
    #[serde(rename = "mouse.get")]
    Mouse(Empty),
    #[serde(rename = "mouse.setHorizontalBlocked")]
    MouseEnable(Enabled),
    #[serde(rename = "permissions.requestAccessibility")]
    Accessibility(Empty),
    #[serde(rename = "permissions.requestInputMonitoring")]
    Input(Empty),
    #[serde(rename = "permissions.openSettings")]
    OpenSettings(Settings),
    #[serde(rename = "diagnostics.copy")]
    Copy(Empty),
    #[serde(rename = "diagnostics.export")]
    Export(Empty),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSave {
    pub expected_revision: String,
    pub settings: crate::runtime_settings::Settings,
}
