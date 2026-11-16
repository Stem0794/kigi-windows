//! Teams-only AX metadata, bounded polling, and exact-window return point.
use super::*;
use crate::meetings::{Effect, Gate, Settings};

struct Entry {
    window: Element,
    title: String,
}
fn same(a: &Element, b: &Element) -> bool {
    unsafe { CFEqual(a.0, b.0) != 0 }
}
fn normal_or_closed(window: &Element) -> Result<bool, String> {
    if window.confirmed_gone() {
        return Ok(true);
    }
    for name in ["AXMinimized", "AXFullScreen"] {
        let value = window.attribute(name)?;
        if unsafe { CFBooleanGetValue(value.0) } != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}
fn inventory(app: &NSRunningApplication) -> Result<Vec<Entry>, String> {
    let array = Element::app(app.processIdentifier()).attribute("AXWindows")?;
    let count = unsafe { CFArrayGetCount(array.0) };
    if count > 16 {
        return Err("Trop de fenêtres Teams : détection suspendue.".into());
    }
    let started = Instant::now();
    let mut entries = Vec::new();
    for i in 0..count {
        if started.elapsed() > Duration::from_millis(750) {
            return Err("Teams répond lentement : détection suspendue.".into());
        }
        let window = Element(unsafe { CFRetain(CFArrayGetValueAtIndex(array.0, i)) });
        // A failed metadata read is unknown, never evidence of meeting end.
        let role = window
            .text("AXSubrole")
            .ok_or("Métadonnées Teams indisponibles.")?;
        if role == "AXStandardWindow" {
            let title = window
                .text("AXTitle")
                .ok_or("Titre de fenêtre Teams inaccessible.")?;
            entries.push(Entry { window, title });
        }
    }
    Ok(entries)
}
fn selected(entries: &[Entry], settings: &Settings) -> Result<Option<(usize, usize)>, String> {
    crate::meetings::classify(
        &entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
        settings,
    )
}

fn app() -> Result<Option<objc2::rc::Retained<NSRunningApplication>>, String> {
    crate::applications::teams_bundle().map(crate::applications::running)
}
pub fn teams_version() -> Option<String> {
    let url = app().ok()??.bundleURL()?;
    let bundle = objc2_foundation::NSBundle::bundleWithURL(&url)?;
    bundle
        .objectForInfoDictionaryKey(&NSString::from_str("CFBundleShortVersionString"))?
        .downcast::<NSString>()
        .ok()
        .map(|s| s.to_string())
}
pub fn teams_has_meeting(settings: &Settings) -> Result<bool, String> {
    let Some(app) = app()? else {
        return Ok(false);
    };
    if settings.chat_title.trim().is_empty() || settings.meeting_title.trim().is_empty() {
        return Err("Profil Teams à configurer avant le scénario de démarrage.".into());
    }
    Ok(selected(&inventory(&app)?, settings)?.is_some())
}
pub fn teams_probe(settings: &Settings) -> Result<String, String> {
    require_permission()?;
    let app = app()?.ok_or("Ouvrez Teams, son chat et une réunion dans deux fenêtres séparées.")?;
    let entries = inventory(&app)?;
    match selected(&entries, settings)? {
        Some((chat, meeting)) => {
            entries[chat].window.ensure_normal()?;
            entries[meeting].window.ensure_normal()?;
            Ok("Un chat et une réunion distincts reconnus. Vérifiez les fenêtres avec Identifier avant l’activation ; aucun déplacement effectué.".into())
        }
        None => Ok("Aucune réunion correspondant au repère. Aucun déplacement effectué.".into()),
    }
}
pub fn teams_verify(settings: &Settings) -> Result<(), String> {
    require_permission()?;
    let app =
        app()?.ok_or("Ouvrez Teams et une réunion pour valider le profil avant son activation.")?;
    let entries = inventory(&app)?;
    let (chat, meeting) = selected(&entries, settings)?
        .ok_or("Ouvrez une réunion et vérifiez les deux repères avant d’activer le profil.")?;
    entries[chat].window.ensure_normal()?;
    entries[meeting].window.ensure_normal()?;
    Ok(())
}
pub fn teams_choices() -> Result<Vec<Choice>, String> {
    require_permission()?;
    let app = app()?.ok_or("Ouvrez Teams avec un chat et une réunion séparés.")?;
    let bundle_id = app
        .bundleIdentifier()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let entries = inventory(&app)?;
    let mut choices = Vec::new();
    for entry in entries {
        if entry.window.ensure_normal().is_ok() {
            choices.push(Choice {
                label: entry.title,
                bundle_id: bundle_id.clone(),
                window: entry.window,
                app: app.clone(),
            });
        }
    }
    if choices.len() < 2 {
        return Err("Ouvrez le chat et la réunion dans deux fenêtres Teams séparées, hors plein écran et non minimisées.".into());
    }
    Ok(choices)
}
impl Element {
    fn set_teams_frame(&self, target: Rect, display: Rect, right: bool) -> Result<Rect, String> {
        let mut actual = self.set_frame(target)?;
        // Do not resize again: Teams may enforce a width larger than the requested half.
        // Anchor using the read-back size instead of rolling both windows back.
        for _ in 0..2 {
            let anchored = actual
                .aligned_to_edge(display, right)
                .ok_or("La taille imposée par Teams dépasse la zone utilisable de l’écran.")?;
            self.set_pair("AXPosition", 1, [anchored.x, anchored.y])?;
            actual = self.frame()?;
            if actual.aligned_to_edge(display, right).is_some_and(|edge| {
                (actual.x - edge.x).abs() <= 1. && (actual.y - edge.y).abs() <= 1.
            }) {
                diagnostics::record(
                    "teams.frame.anchored",
                    format!(
                        "right={right} requested={target:?} actual={actual:?} exact={}",
                        actual.approximately(target)
                    ),
                );
                return Ok(actual);
            }
        }
        Err("Teams n’a pas confirmé l’alignement au bord de l’écran.".into())
    }
}
struct Session {
    chat: Element,
    meeting: Element,
    before: [Rect; 2],
    applied: [Rect; 2],
    display: Rect,
    placement_message: String,
}
#[derive(Default)]
pub struct TeamsAutomation {
    gate: Gate,
    candidate: Option<[Element; 2]>,
    session: Option<Session>,
    blocker: Option<Element>,
    blocker_missing_since: Option<u64>,
    pub status: String,
}
impl TeamsAutomation {
    pub fn pause_detection(&mut self) {
        self.gate.step(None, 0);
    }
    pub fn tracking(&self) -> bool {
        self.session.is_some()
    }
    pub fn apply_now(
        &mut self,
        mtm: MainThreadMarker,
        settings: &Settings,
        now: u64,
    ) -> Result<String, String> {
        require_permission()?;
        let app = app()?.ok_or("Ouvrez Teams et ses deux fenêtres avant de partager.")?;
        let entries = inventory(&app)?;
        self.apply_selected(mtm, settings, now, &app, &entries)
    }
    fn apply_selected(
        &mut self,
        mtm: MainThreadMarker,
        settings: &Settings,
        now: u64,
        app: &NSRunningApplication,
        entries: &[Entry],
    ) -> Result<String, String> {
        let (chat,meeting)=selected(&entries,settings)?.ok_or("Aucune paire chat/réunion correspondant au profil. Identifiez les fenêtres dans Réunions Teams.")?;
        let before = self
            .session
            .take()
            .filter(|session| {
                same(&session.chat, &entries[chat].window)
                    && same(&session.meeting, &entries[meeting].window)
            })
            .map(|s| s.before);
        self.blocker = None;
        self.candidate = None;
        self.gate = Gate::default();
        self.advance_internal(mtm, settings, now, entries, true);
        if let Some(session) = self.session.as_mut() {
            if let Some(before) = before {
                session.before = before;
            }
            session.chat.raise()?;
            session.meeting.raise()?;
            let _ = crate::applications::request_activation(app);
            focus_window(app, &session.meeting)?;
            Ok(self.status.clone())
        } else {
            Err(self.status.clone())
        }
    }
    pub fn inhibit(&mut self) {
        self.blocker_missing_since = None;
        self.gate.inhibit();
        if let Some(session) = self.session.take() {
            self.blocker = Some(session.meeting);
        } else if let Some([_, meeting]) = self.candidate.take() {
            self.blocker = Some(meeting);
        }
        self.candidate = None;
        self.status =
            "Automatisme suspendu jusqu’à la fermeture de la réunion (priorité à votre action)."
                .into();
    }
    fn unknown(&mut self, now: u64, message: &str) {
        self.gate.step(None, now);
        self.status = message.into();
    }
    pub fn poll(&mut self, mtm: MainThreadMarker, settings: &Settings, now: u64, busy: bool) {
        if !settings.enabled && self.session.is_none() {
            return;
        }
        if busy {
            self.unknown(now, "Action en cours : automatisme en attente.");
            return;
        }
        if !trusted() {
            self.unknown(now, "Accessibilité requise. Aucun déplacement automatique.");
            return;
        }
        let running = match app() {
            Ok(app) => app,
            Err(message) => {
                self.unknown(now, &message);
                return;
            }
        };
        if running.as_ref().is_some_and(|a| a.isHidden()) {
            self.inhibit();
            self.status = "Teams est masqué : automatisme suspendu.".into();
            return;
        }
        let entries = match running.as_ref().map(|a| inventory(a)).transpose() {
            Ok(Some(entries)) => entries,
            Ok(None) => Vec::new(),
            Err(message) => {
                self.unknown(now, &message);
                return;
            }
        };
        let starting = self.session.is_none();
        self.advance(mtm, settings, now, &entries);
        // Bring both existing Teams windows forward only within an already active Teams app.
        if starting && app().ok().flatten().is_some_and(|a| a.isActive()) {
            if let Some(session) = self.session.as_ref() {
                let _ = session.chat.raise();
                let _ = session.meeting.raise();
            }
        }
    }
    fn advance(&mut self, mtm: MainThreadMarker, settings: &Settings, now: u64, entries: &[Entry]) {
        self.advance_internal(mtm, settings, now, entries, false);
    }
    fn advance_internal(
        &mut self,
        mtm: MainThreadMarker,
        settings: &Settings,
        now: u64,
        entries: &[Entry],
        force: bool,
    ) {
        if let Some(blocker) = self.blocker.as_ref() {
            let present = entries.iter().any(|e| same(&e.window, blocker));
            match normal_or_closed(blocker) {
                Ok(normal) if !present && normal => {}
                _ => {
                    self.blocker_missing_since = None;
                    self.status =
                        "Votre disposition est conservée jusqu’à la fermeture de la réunion."
                            .into();
                    return;
                }
            }
            let since = *self.blocker_missing_since.get_or_insert(now);
            if now.saturating_sub(since) < 2000 {
                return;
            }
            self.blocker_missing_since = None;
            self.blocker = None;
            // Do not rearm while a replacement matching meeting still exists.
            if entries.iter().any(|e| settings.meeting_matches(&e.title)) {
                return;
            }
            self.gate.step(Some(false), now);
        }
        if let Some(session) = self.session.as_ref() {
            // The exact meeting remains active even if its title changes or it is minimized.
            let present = entries.iter().any(|e| {
                same(&e.window, &session.meeting)
                    && (!settings.end_on_title_change || settings.meeting_matches(&e.title))
            });
            if !present {
                match normal_or_closed(&session.meeting) {
                    Ok(true) => {}
                    Ok(false) => {
                        self.inhibit();
                        return;
                    }
                    Err(_) => {
                        self.unknown(now, "État de fenêtre inconnu : retour suspendu.");
                        return;
                    }
                }
            }
            let replacements = entries
                .iter()
                .any(|e| !same(&e.window, &session.meeting) && settings.meeting_matches(&e.title));
            if replacements {
                self.inhibit();
                return;
            }
            let chat_frame = session.chat.frame().ok();
            let meeting_frame = if present {
                session.meeting.frame().ok()
            } else {
                None
            };
            let unchanged = chat_frame.is_some_and(|f| f.approximately(session.applied[0]))
                && visible_frame(mtm, session.applied[0])
                    .is_ok_and(|d| d.approximately(session.display))
                && (!present || meeting_frame.is_some_and(|f| f.approximately(session.applied[1])));
            if !unchanged {
                diagnostics::record("teams.geometry.changed",format!("chat={chat_frame:?} meeting={meeting_frame:?} expected={:?} present={present}",session.applied));
            }
            if !unchanged
                || session.chat.ensure_normal().is_err()
                || (present && session.meeting.ensure_normal().is_err())
            {
                self.inhibit();
                return;
            }
            if self.gate.step(Some(present), now) == Effect::End {
                let session = self.session.take().unwrap();
                let target = if settings.maximize_after {
                    session.display
                } else {
                    session.before[0]
                };
                self.status = match session.chat.set_frame(target) {
                    Ok(actual) if actual.approximately(target) => "Réunion terminée : position du chat confirmée.".into(),
                    _ => "Réunion terminée : retour du chat non confirmé. Aucun nouvel essai automatique.".into(),
                };
                diagnostics::record("teams.end", "attempted=true");
            } else {
                self.status = if present {
                    session.placement_message.as_str()
                } else {
                    "Fenêtre réunion fermée : confirmation de fin pendant 2 secondes…"
                }
                .into();
            }
            return;
        }
        let pair = match selected(entries, settings) {
            Ok(pair) => pair,
            Err(message) => {
                self.unknown(now, &message);
                return;
            }
        };
        let Some((chat, meeting)) = pair else {
            self.gate.step(Some(false), now);
            self.candidate = None;
            self.status = "En attente d’une fenêtre réunion correspondant au profil.".into();
            return;
        };
        let chat = &entries[chat].window;
        let meeting = &entries[meeting].window;
        if self
            .candidate
            .as_ref()
            .is_none_or(|p| !same(&p[0], chat) || !same(&p[1], meeting))
        {
            self.gate.reset_candidate();
            self.candidate = Some([
                Element(unsafe { CFRetain(chat.0) }),
                Element(unsafe { CFRetain(meeting.0) }),
            ]);
        }
        if force {
            self.gate.activate();
        }
        if !force && self.gate.step(Some(true), now) != Effect::Start {
            self.status = "Profil reconnu ; attente de stabilité ou de fin de suspension.".into();
            return;
        }
        let result = (|| {
            chat.ensure_normal()?;
            meeting.ensure_normal()?;
            for window in [chat, meeting] {
                window.can_set("AXPosition")?;
                window.can_set("AXSize")?;
            }
            let before = [chat.frame()?, meeting.frame()?];
            let display = visible_frame(mtm, before[1])?;
            let width = display.width * f64::from(settings.chat_percent) / 100.;
            let left = Rect {
                width: if settings.meeting_right {
                    width
                } else {
                    display.width - width
                },
                ..display
            };
            let right = Rect {
                x: display.x + left.width,
                width: display.width - left.width,
                ..display
            };
            let targets = if settings.meeting_right {
                [left, right]
            } else {
                [right, left]
            };
            // Keep a return point before the first write, including partial failure.
            let [chat, meeting] = self.candidate.take().unwrap();
            self.session = Some(Session {
                chat,
                meeting,
                before,
                applied: before,
                display,
                placement_message: String::new(),
            });
            let session = self.session.as_mut().unwrap();
            let chat_result =
                session
                    .chat
                    .set_teams_frame(targets[0], display, !settings.meeting_right);
            session.applied[0] = session.chat.frame().unwrap_or(session.applied[0]);
            chat_result?;
            let meeting_result =
                session
                    .meeting
                    .set_teams_frame(targets[1], display, settings.meeting_right);
            session.applied[1] = session.meeting.frame().unwrap_or(session.applied[1]);
            meeting_result?;
            let exact = session
                .applied
                .iter()
                .zip(targets)
                .all(|(actual, target)| actual.approximately(target));
            session.placement_message = if exact {
                "Chat et réunion placés à la largeur demandée et alignés aux bords.".into()
            } else {
                format!("Teams limite les dimensions : chat {:.0} %, réunion {:.0} %. Bords alignés ; un chevauchement est possible.",session.applied[0].width/display.width*100.,session.applied[1].width/display.width*100.)
            };
            diagnostics::record(
                "teams.placement",
                format!(
                    "exact_requested={exact} chat={:?} meeting={:?}",
                    session.applied[0], session.applied[1]
                ),
            );
            Ok(())
        })();
        if let Err(message) = result {
            if let Some(session) = self.session.take() {
                // Roll back only untouched writes, never a later manual placement.
                for ((window, before), applied) in [session.chat, session.meeting]
                    .iter()
                    .zip(session.before)
                    .zip(session.applied)
                {
                    if window.frame().is_ok_and(|f| f.approximately(applied)) {
                        let _ = window.set_frame(before);
                    }
                }
            }
            self.inhibit();
            self.status = message;
            diagnostics::record("teams.start", "confirmed=false");
        } else {
            self.status = self.session.as_ref().unwrap().placement_message.clone();
            diagnostics::record("teams.start", "confirmed=true");
        }
    }
}

/// Isolated CI command: only the dedicated fixture bundle, never Teams/user windows.
pub fn teams_fixture_check(mtm: MainThreadMarker, manual: bool) -> Result<(), String> {
    if std::env::var_os("KIGI_TEST_TEAMS").is_none() {
        return Err("Fixture explicitement requise.".into());
    }
    require_permission()?;
    let app =
        crate::applications::running("io.kigi.focus.source").ok_or("Fixture Teams absente.")?;
    let minimum = std::env::args().any(|a| a == "--meeting-minimum");
    let reapply = std::env::args().any(|a| a == "--teams-reapply");
    let settings = Settings {
        maximize_after: !reapply,
        enabled: true,
        chat_title: "Kigi chat fixture".into(),
        meeting_title: "Kigi meeting fixture".into(),
        ..Settings::default()
    };
    let mut automation = TeamsAutomation::default();
    let entries = inventory(&app)?;
    let (chat, meeting) = selected(&entries, &settings)?.ok_or("Paire fixture absente.")?;
    let before = entries[chat].window.frame()?;
    automation.advance(mtm, &settings, 0, &entries);
    if automation.session.is_some() {
        return Err("Déclenchement avant stabilité.".into());
    }
    automation.advance(mtm, &settings, 1000, &inventory(&app)?);
    let session = automation
        .session
        .as_ref()
        .ok_or("Disposition automatique non appliquée.")?;
    let chat_frame = session.chat.frame()?;
    let meeting_frame = session.meeting.frame()?;
    if !chat_frame.approximately(session.display.left_half())
        || (meeting_frame.x + meeting_frame.width - session.display.x - session.display.width).abs()
            > 1.
        || (meeting_frame.y - session.display.y).abs() > 1.
        || (!minimum && !meeting_frame.approximately(session.display.right_half()))
        || (minimum && meeting_frame.width <= session.display.width * 0.55)
    {
        return Err("Géométrie automatique ou ancrage droit incorrect.".into());
    }
    if minimum && !session.placement_message.contains("Teams limite") {
        return Err("Dimensions limitées annoncées comme un 50/50 exact.".into());
    }
    if reapply {
        let session = automation.session.as_ref().unwrap();
        session.chat.set_frame(Rect {
            x: session.display.x + 50.,
            width: session.display.width * 0.7,
            ..session.display
        })?;
        automation.apply_selected(mtm, &settings, 1500, &app, &inventory(&app)?)?;
        if !automation
            .session
            .as_ref()
            .is_some_and(|s| s.before[0].approximately(before))
        {
            return Err("La réapplication a écrasé le point de retour initial.".into());
        }
    }
    let session = automation.session.as_ref().unwrap();
    let target = if manual {
        let target = Rect {
            x: session.display.x + 75.,
            y: session.display.y + 60.,
            width: session.display.width * 0.65,
            height: session.display.height * 0.7,
        };
        session.chat.set_frame(target)?;
        automation.advance(mtm, &settings, 2000, &inventory(&app)?);
        if automation.session.is_some() {
            return Err("Le déplacement manuel n’a pas suspendu l’automatisme.".into());
        }
        target
    } else {
        session.display
    };
    let button = entries[meeting].window.attribute("AXCloseButton")?;
    let action = NSString::from_str("AXPress");
    check(unsafe { AXUIElementPerformAction(button.0, name_ref(&action)) })?;
    // Give the separate fixture process time to deliver its own close callback.
    for _ in 0..20 {
        if !inventory(&app)?
            .iter()
            .any(|e| same(&e.window, &entries[meeting].window))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if inventory(&app)?
        .iter()
        .any(|e| same(&e.window, &entries[meeting].window))
    {
        return Err("Fermeture fixture non confirmée.".into());
    }
    automation.advance(mtm, &settings, 3000, &inventory(&app)?);
    if !manual && entries[chat].window.frame()?.approximately(target) {
        return Err("Retour avant le délai de confirmation.".into());
    }
    automation.advance(mtm, &settings, 4999, &inventory(&app)?);
    automation.advance(mtm, &settings, 5000, &inventory(&app)?);
    if !entries[chat].window.frame()?.approximately(target) {
        return Err("Retour final ou priorité manuelle incorrecte.".into());
    }
    entries[chat].window.set_frame(before)?;
    Ok(())
}
