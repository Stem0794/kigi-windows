# Handover Codex — gestionnaire macOS de fenêtres, espaces de travail et gestes (Rust-first)

**État :** cahier des charges technique et plan d'exécution, pas une implémentation existante.  
**Date de référence :** 12 novembre 2026.  
**Dépôt de travail UNIQUE :** `https://github.com/Stem0794/kigi-windows` (`main` par défaut ; ne pas créer de nouveau dépôt).  
**Nom de code :** `kigi-windows` (nom du dépôt ; le nom commercial reste à définir).  
**Destination :** Codex, développeurs Rust/macOS, testeur produit (premier utilisateur = commanditaire).  
**Langue des interfaces par défaut :** français, avec architecture prête pour anglais.  
**Licence future :** choix commercial à finaliser ; aucun code Hammerspoon requis dans le produit distribué.

> **Instruction de départ à Codex.** Travailler exclusivement dans le dépôt existant `Stem0794/kigi-windows`, en inspectant ses fichiers avant de les modifier. Ne pas créer d’autre repository ni déplacer le projet. Construire un logiciel macOS natif, autonome, sans Hammerspoon installé, dont le cœur est écrit en Rust. Implémenter **phase par phase**, dans l'ordre et avec les critères de sortie ci-dessous. Ne pas prétendre qu'un geste global du trackpad ou une détection de réunion fonctionne sans l'avoir testé sur macOS. Privilégier la simplicité : un seul crate Rust initial, pas de couches abstraites anticipées, de microservices, ni de grande batterie de smoke tests. Tester uniquement les comportements critiques avec quelques tests unitaires ciblés et des vérifications manuelles pertinentes. Ne pas introduire Electron, navigateur embarqué, service cloud, daemon séparé, serveur HTTP local, télémétrie ou moteur Lua pour le MVP. Ne jamais modifier `~/.hammerspoon/init.lua` ni désinstaller Hammerspoon sans consentement explicite. Ne pas utiliser d'API privées Apple sans accord explicite : le projet doit pouvoir être signé/notarisé et maintenu.

---

## Règles de développement impératives — simplicité d’abord

1. **Dépôt :** travailler uniquement dans `https://github.com/Stem0794/kigi-windows`. Le dépôt est la source de vérité. Examiner l’état de `main` et les modifications existantes avant de coder ; créer ou modifier les fichiers *dans ce dépôt*. Ne pas initialiser un projet dans un autre dépôt puis le recopier.
2. **Un seul crate Rust au départ.** Utiliser `src/` et quelques modules concrets. Ne pas créer de workspace multi-crates, de couches `ports/adapters`, d’injection de dépendances généralisée ou de frameworks internes sans preuve qu’ils deviennent nécessaires.
3. **Implémenter le strict nécessaire de la phase en cours.** Pas de fonctionnalités spéculatives, de compatibilité artificielle, de système de plugins, de moteur de tâches, ni de prématurée optimisation.
4. **Tests proportionnés.** Après un changement, compiler et exécuter les tests *pertinents*. Écrire 1 à 3 tests unitaires concis pour une logique pure présentant un risque réel (géométrie, classification des gestes, configuration, priorités). Vérifier manuellement une fois sur Mac les fonctions dépendant d’Accessibility, des raccourcis et du trackpad. Corriger les régressions constatées. **Ne pas créer 10 000 smoke tests**, de suite E2E gigantesque, de matrices combinatoires, de mocks système complexes, ni répéter en boucle les mêmes essais sans anomalie.
5. **Pas d’infrastructure de test ou de build disproportionnée.** Démarrer avec `cargo fmt`, `cargo check`, `cargo test` et un lancement manuel de `.app`. Ajouter CI, benchmarks automatisés, packaging élaboré et tests multi-machines uniquement lorsqu’ils seront utiles aux versions destinées à d’autres utilisateurs.
6. **Dépendances réduites.** Préférer la bibliothèque standard Rust et les API Apple, puis les crates indispensables ; éviter de multiplier les abstractions et les moteurs concurrents.
7. **S’arrêter après la phase demandée.** Pour chaque phase : petit commit cohérent, brève note dans `docs/progress.md` (fait, essayé, reste à faire) et consigne d’exécution. Aucun rapport fleuve ni documentation redondante.
8. **La fiabilité prime sur les volumes de tests.** Les limites de l’API multitouch, les autorisations macOS et la restauration des fenêtres requièrent des vérifications réelles ; les autres tests restent ciblés. Ne pas prétendre à une compatibilité OS/Intel non vérifiée.

---

## 0. Résultat attendu et limites explicites

### 0.1 Objectif commercial

Créer une app accessible dans la barre de menus, permettant à un utilisateur non technique de :

1. Redimensionner et déplacer les fenêtres par raccourcis.
2. Sauvegarder une disposition de plusieurs fenêtres comme **espace de travail**.
3. Associer à un espace de travail un raccourci clavier **ou un geste trackpad**.
4. Ouvrir/activer les applications et restaurer leurs positions.
5. Définir des règles contextuelles locales (application activée, fenêtre créée, changement de moniteur, réunion reconnue lorsque possible).
6. Restaurer la disposition antérieure après une action temporaire.
7. Configurer tout cela depuis une UI sans script.

**Définition importante :** dans ce produit, un « espace de travail » est une **disposition nommée de fenêtres/applications**, pas un **Space virtuel de Mission Control**. La création, le déplacement et la sélection arbitraire des Spaces macOS ne sont pas des engagements de V1. Ne pas utiliser de méthodes privées pour simuler cette fonctionnalité.

### 0.2 Le périmètre V1

- Fenêtres : reconnaître, sélectionner, activer, positionner, dimensionner, minimiser/restaurer si compatible.
- Écrans : un écran, puis plusieurs écrans avec profils d'affichage.
- Raccourcis : configuration, détection de conflits, activation sans fenêtre au premier plan.
- Espaces de travail : enregistrer, appliquer, restaurer l'état précédent.
- Déclencheurs : raccourci, geste, activation d'app, apparition/disparition d'une fenêtre ; règles de réunion limitées à des signatures vérifiées.
- Trackpad : **au moins un geste de coin à un doigt à valider dans un spike**, en écoute passive sans bloquer les gestes habituels.
- UI : barre de menus + préférences accessibles, édition des scènes/gestes/raccourcis, statut des permissions et erreurs.
- Données : locales, sans compte.

### 0.3 Exclusions V1

Pas d'automatisation générique façon Hammerspoon (shell arbitraire, réseau, Wi‑Fi, audio, calendriers, AppleScript libre, plugins tiers), pas d'IA, pas de synchronisation cloud, pas de collaboration, pas de capture écran, pas d'analyse du contenu des réunions, pas de déplacement automatique de fenêtres entre Spaces par API privée, pas de support Windows/Linux, pas de scripting utilisateur, pas d'extension Chrome obligatoire, pas de daemon root.

### 0.4 Contraintes produit

- Disponible sur **Intel x86_64 et Apple Silicon aarch64**.
- **macOS 11 Big Sur ciblé**, mais seulement revendiqué après essais physiques/VM ; prévoir un ajustement vers macOS 12 ou 13 si dépendances/API impossibles à maintenir. Ne pas choisir silencieusement macOS 13+.
- Une seule app `.app` signable, mode menu bar (`LSUIElement`) avec fenêtre de réglages affichable.
- Fonctionnement hors ligne.
- Les opérations refusées par macOS (ex. fenêtres système ou certaines apps) doivent afficher une erreur explicite, sans blocage.
- Permissions demandées séparément, seulement lorsqu'une fonction le nécessite.
- App toujours utilisable partiellement sans accès au trackpad ; pas de promesse de geste si non validé.

---

## 1. Spécifications héritées de l'usage personnel à migrer

### 1.1 Baseline à reconstruire (sans désinstaller l'existant)

Créer des actions équivalentes aux usages déjà décrits :

| ID | Déclencheur | Action attendue | Remarques |
|---|---|---|---|
| `focus_teams` | `⌥T` | Activer Teams ; ne pas le dupliquer | Si absent, lancement configurable ; défaut = lancer |
| `focus_browser` | `⌥B` | Activer Chrome et maximiser sa fenêtre principale dans la zone utilisable (100 %) | Garder Teams derrière |
| `open_chatgpt` | `⌥C` | Ouvrir/activer ChatGPT dans Chrome | V1 : ouvrir URL ; ciblage d'un onglet existant = intégration optionnelle soumise à permission Apple Events |
| `right_half` | `⌘⇧↓` | Fenêtre active dans la moitié droite de l'écran courant | Vérifier la correspondance exacte avec l'`init.lua` réellement actif avant de figer la migration |
| `maximize` | `⌘⇧↑` | Fenêtre active maximisée en zone visible | Maximisation géométrique, ne pas basculer en fullscreen Space natif |
| `cycle_tabs` | `⌥Tab` | Faire défiler les onglets/fenêtres suivant le contexte de l'app | Optionnel derrière adapter, tester conflits système et alternatives ; pas de promesse de contrôle générique des onglets |
| `workspace_meeting` | Scène / raccourci | Teams à gauche 50 % + Chrome à droite 50 % | Variante « réunion + navigateur » |
| `workspace_chat` | Scène / raccourci | Fenêtre de réunion Teams à gauche 50 % + chat Teams détaché à droite 50 % | Seulement si deux fenêtres Teams distinctes ; sinon message d'aide, pas de fenêtres inventées |
| `restore_work` | Sortie de réunion ou raccourci | Chrome revient en 100 %, ou restauration exacte du snapshot préalable | Priorité au snapshot si l'utilisateur a activé « restaurer » |
| `trackpad_left` | Glissement descendant du coin supérieur gauche du trackpad | Activer l'espace de travail assigné | Mapping configurable ; défaut suggéré « Teams/Meeting » |
| `trackpad_right` | Glissement descendant du coin supérieur droit | Activer espace « Browser » | Assigner après validation terrain ; défaut peut être désactivé |

**Instruction migration :** au démarrage du développement sur le Mac personnel du commanditaire, lire le `~/.hammerspoon/init.lua` **en lecture seule** et produire `docs/migration-audit.md` avec les raccourcis réellement actifs, les corrections et toute incompatibilité. Les raccourcis listés ci-dessus sont une base de travail ; le fichier actif prévaut. Ne pas ajouter les anciennes règles abandonnées (90 %, minuteur de réunion, Stage Manager).

### 1.2 Parcours de référence

- **État normal :** Chrome occupe 100 % de l'espace visible ; Teams peut rester derrière sans être fermé.
- **Réunion :** activer `Meeting + Browser` → lancer/activer Teams et Chrome, rechercher les bonnes fenêtres, gauche/droite à parts égales ; garantir que la fenêtre de réunion reste visible si trouvée.
- **Réunion + chat :** choisir `Meeting + Chat` → positionner la fenêtre de réunion et une fenêtre de chat Teams *détachée*. Ne pas tenter de redimensionner des panneaux internes comme des fenêtres distinctes.
- **Fin de réunion :** restaurer snapshot précédent si activé, sinon revenir au scénario `Work` de l'utilisateur. Ne pas fermer des applications ou onglets ouverts par l'utilisateur.
- **Geste supérieur gauche vers bas :** déclencher une scène sélectionnée dans les paramètres ; ne pas intercepter le geste système habituel ni déplacer le pointeur.

### 1.3 Variantes à prendre en compte

- Teams non lancé ; Chrome non lancé ; fenêtre maximisée/minimisée ; plusieurs fenêtres de même app ; deux écrans ; écran externe débranché ; fenêtre AX inaccessible ; changement de Space manuel ; réunion déclenchée sans chat « pop out » ; permission refusée ou révoquée ; utilisateur qui déplace une fenêtre après application d'une scène.

---

## 2. Architecture technique et responsabilités

### 2.1 Choix

- **Rust-first** : moteur, modèle, persistence, reconnaissance des gestes, règles, orchestrateur et autant que possible UI AppKit via `objc2`.
- **AppKit** : interface barre de menus et fenêtre de préférences, sans WebView.
- **Accessibility** : `AXUIElement`, `AXObserver`, `AXUIElementCopyAttributeValue`, `AXUIElementSetAttributeValue`, `AXUIElementPerformAction`.
- **App lifecycle** : `NSWorkspace`, `NSRunningApplication`.
- **Écrans** : `NSScreen` et `visibleFrame` (gestion du Dock, barre de menus, écran principal, écran avec coordonnées négatives).
- **Raccourcis** : `global-hotkey` *ou* wrapper minimal de `RegisterEventHotKey` en cas de conflit avec AppKit ; un seul backend pour les raccourcis simples.
- **Trackpad** : `CGEventTap` en mode écoute seule + tentative `NSEvent`/`allTouches`, entièrement encapsulée dans module expérimental. Pas d'API privée sans ADR (Architecture Decision Record) + autorisation.
- **Configuration** : `serde`, `serde_json`, validations strictes, écritures atomiques. Pas de SQLite tant que le besoin n'est pas démontré.
- **Journalisation** : `tracing`, niveau WARN par défaut, opt-in pour diagnostics avancés.
- **Messages internes** : canal borné `std::sync::mpsc` ou `crossbeam-channel` uniquement si nécessaire ; ne pas engager d'architecture async lourde sans justification.

Versions exactes des crates : **Codex doit les vérifier à la création du dépôt**, épingler `Cargo.lock` et activer seulement les features nécessaires. Ne pas copier de numéros de version non vérifiés depuis ce document.

### 2.2 Diagramme logique

```text
                   ┌──────────────────────────┐
                   │ AppKit status bar + UI   │
                   └─────────────┬────────────┘
                                 │ commandes
       ┌─────────────────────────▼─────────────────────────┐
       │              Application Controller               │
       │ permissions, config, transactions, état, erreurs  │
       └───────┬───────────────┬───────────────┬───────────┘
               │               │               │
      ┌────────▼─────┐ ┌───────▼──────┐ ┌──────▼──────────┐
      │ Hotkey input │ │ Gesture FSM  │ │ Event sources   │
      │ native       │ │ touch events │ │ NSWorkspace/AX  │
      └───────┬──────┘ └───────┬──────┘ └──────┬──────────┘
              └───────────────┬─┴──────────────┘
                              ▼
                  ┌──────────────────────┐
                  │ Rule Engine / Router │
                  └──────────┬───────────┘
                             ▼
                  ┌──────────────────────┐
                  │ Workspace Executor   │
                  │ snapshot, match, AX  │
                  └──────────┬───────────┘
                             ▼
                  ┌──────────────────────┐
                  │ macOS public APIs    │
                  └──────────────────────┘
```

### 2.3 Contraintes des threads

1. `NSApplication`, objets `AppKit` et backend de hotkeys demandant une boucle d'événements : **thread principal**. Un `NSStatusBar` `!Send`/`!Sync` ne doit pas être transféré vers un thread worker.
2. Callbacks `CGEventTap` : faire uniquement la conversion minimale et pousser vers un buffer borné ; ne jamais invoquer AX ou ouvrir une fenêtre de préférences dans le callback.
3. AX : opérations sérialisées et appels courts, avec gestion des erreurs/timeouts. Ne créer un service ou un thread dédié que si des blocages concrets sont constatés. Éviter de tenir des mutex pendant des appels AX.
4. UI sur le thread principal. Utiliser un canal uniquement si un worker devient nécessaire ; pas de `unsafe impl Send`/`Sync` artificiel pour les types Objective‑C.
5. Shutdown : arrêter les sources d'événements, libérer les observers et désenregistrer les raccourcis, puis fermer le canal ; pas de callback utilisant de pointeur libéré.

### 2.4 Arborescence du dépôt — commencer petit

Le dépôt officiel est `Stem0794/kigi-windows`. **Ne pas créer un autre dépôt** et ne pas imposer toute cette arborescence dès P0. Ajouter les fichiers uniquement lorsqu’ils ont une utilité concrète.

```text
kigi-windows/
  Cargo.toml
  Cargo.lock
  README.md
  AGENTS.md
  HANDOVER_CODEX.md
  CODEX_START.md
  src/
    main.rs                  # Application macOS et menu bar P0
    windows.rs               # Accessibility, après validation P0
    hotkeys.rs               # Raccourcis globaux
    gestures.rs              # Prototype trackpad, puis classification
    workspaces.rs            # Ajouté en P3, seulement si nécessaire
    config.rs                # Ajouté lors de l’enregistrement des scènes
  resources/
    Info.plist
  examples/
    default-config.json
  docs/
    trackpad-spike.md        # Court rapport de faisabilité P0
    progress.md              # Quelques lignes à chaque phase
```

Éviter de créer des modules vides. Garder les modèles et traitements purement Rust séparés des appels AppKit/Accessibility **à l’intérieur du même crate**. Extraire plusieurs crates uniquement si un problème concret apparaît (compilation, réutilisation indépendante, frontières difficiles à maintenir), et documenter brièvement pourquoi.

### 2.5 Interfaces Rust attendues (propositions, à adapter aux bindings)

```rust
pub struct DisplayId(pub String);
pub struct WindowId(pub u64); // ID uniquement runtime, jamais persistant

pub struct Rect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
pub struct NormalizedRect { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }

pub struct WindowDescriptor {
    pub id: WindowId,
    pub pid: i32,
    pub bundle_id: String,
    pub title: Option<String>,
    pub role: Option<String>,
    pub subrole: Option<String>,
    pub frame: Rect,
    pub is_minimized: bool,
    pub is_fullscreen: bool,
    pub display_id: Option<DisplayId>,
}

pub trait WindowBackend {
    fn list(&mut self) -> Result<Vec<WindowDescriptor>, WindowError>;
    fn activate(&mut self, id: WindowId) -> Result<(), WindowError>;
    fn set_frame(&mut self, id: WindowId, frame: Rect) -> Result<Rect, WindowError>;
    fn minimize(&mut self, id: WindowId, value: bool) -> Result<(), WindowError>;
}

pub trait AppBackend {
    fn is_running(&self, bundle_id: &str) -> bool;
    fn launch(&mut self, bundle_id: &str) -> Result<(), AppError>;
    fn activate(&mut self, bundle_id: &str) -> Result<(), AppError>;
}

pub enum TriggerEvent {
    Hotkey(String),
    Gesture(String),
    AppActivated(String),
    WindowCreated { bundle_id: String, window: WindowId },
    WindowClosed { bundle_id: String, window: WindowId },
    DisplayConfigurationChanged,
    ManualSceneActivation(String),
}

pub enum Action {
    FocusApp { bundle_id: String },
    OpenUrl { url: String, browser_bundle_id: Option<String> },
    ArrangeWorkspace { workspace_id: String },
    RestorePreviousLayout,
    ResizeActiveWindow { target: NormalizedRect },
}
```

Prévoir des erreurs structurées `WindowError::{PermissionDenied, InvalidElement, Unsupported, Timeout, WindowGone, NoMatch, Other}` et rapports d'application `Applied`, `PartiallyApplied`, `Failed`. Les identifiants AX ne sont pas stables entre redémarrages ; le matching persistant se fait par **bundle ID + critères de fenêtre**, pas par PID ou pointeur AX.

---

## 3. Sous-système fenêtres et displays

### 3.1 Découverte d'applications

- `NSWorkspace.sharedWorkspace.runningApplications` pour initialisation.
- Observations `didLaunch`, `didTerminate`, `didActivate`, `didDeactivate` sur le `notificationCenter` de `NSWorkspace`.
- `bundleIdentifier` est la clé de référence persistante ; ne pas associer uniquement par nom affiché. Gérer les processus sans bundle ID comme non supportés/exception.
- `AXUIElementCreateApplication(pid)` pour l'arbre d'accessibilité, `kAXWindowsAttribute` pour la liste, `kAXFocusedWindowAttribute`/`kAXMainWindowAttribute` selon disponibilité.
- Ne pas examiner l'ensemble des fenêtres en boucle à 60 Hz : mise en cache et rafraîchissements déclenchés par notification, avec resynchronisation ponctuelle seulement en cas de perte d'observer.

### 3.2 Sélection d'une fenêtre

Construire des `WindowMatcher` composables, **avec ordre de choix déterministe** :

1. bundle ID exact requis ;
2. rôle/subrôle si spécifié ;
3. correspondance sur titre avec mode `exact`, `contains`, `regex` (regex optionnelle, bornée) ;
4. fenêtre principale/frontmost si critère « préférée » ;
5. sinon dernière fenêtre utilisée de l'app ;
6. ambiguïté irréductible → interface demandant à l'utilisateur de choisir ou préférences `first / focused / last / all`, sans déplacer au hasard.

La détection « réunion Teams » est un profil d'application, pas une règle universelle basée sur le mot « Meeting » ; mémoriser les signatures constatées et pouvoir les mettre à jour.

### 3.3 Positionnement

- Utiliser `NSScreen.visibleFrame`, pas des dimensions en pixels d'écran physique.
- Conversion séparée entre repère AppKit (origine bas-gauche) et AX (repère écran différent selon API) ; tests unitaires pour écran principal et écran à gauche (coordonnées négatives).
- Calculer le rectangle cible à partir du rectangle normalisé `[0, 1]` **de la zone visible de l'écran cible**.
- `right_half` → `{x:0.5,y:0,w:0.5,h:1}` ; `left_half` → `{x:0,y:0,w:0.5,h:1}` ; `maximize` → `{x:0,y:0,w:1,h:1}`.
- Clamper aux bords, taille non négative, largeur/hauteur mini par application. Si l'application impose une taille min incompatible, renvoyer `PartiallyApplied` avec dimensions réellement obtenues.
- Écriture position/taille via AX (tester ordre position→taille et taille→position, fixer l'ordre meilleur par app). Relire l'état final ; tolérance de position et taille `<= 8 pt` sauf apps limitantes.
- Plein écran natif macOS / fenêtre d'un autre Space / certaines fenêtres Electron ou système : signaler la limitation. Ne pas forcer le passage hors fullscreen sans consentement explicite.
- Ne jamais fermer ou quitter une app juste pour résoudre une disposition.

### 3.4 Plusieurs écrans

- ID d'écran stable **best effort** (identifiant de moniteur ; fallback rôle principal/interne/externe + taille et ordre).
- Résolution et scaling gérés par points AppKit.
- Si écran ciblé absent : repli sur écran principal, recalcul de rectangle et notification discrète.
- Recalcul à la notification de changement d'écrans, avec délai de stabilisation de 300–600 ms (paramétrable) pour éviter plusieurs transitions successives ; garder le snapshot pour restauration.
- Une scène peut dire « écran où se trouve la fenêtre active », « écran principal », ou « écran identifié ». Pas de dépendance à une résolution fixe.

### 3.5 Transactions d'application de scénario

Algorithme :

```text
apply_workspace(workspace_id):
  1. lire et valider la configuration (état immuable)
  2. prendre le verrou exclusif de transition (coalescer actions identiques)
  3. snapshot runtime de l'état des fenêtres concernées
  4. résoudre les apps cibles et démarrer celles qui manquent, si autorisé
  5. attendre événement apparition fenêtre avec timeout borné (3 s par défaut)
  6. sélectionner les fenêtres selon les matchers stables
  7. calculer frames sur displays actuels
  8. appliquer géométrie et focus, puis relire les positions
  9. reporter Applied / PartiallyApplied / Failed et éléments ignorés
 10. publier WorkspaceApplied ; archiver snapshot en mémoire uniquement
 11. libérer le verrou et traiter le dernier événement en attente
```

- Tolérer les fermetures de fenêtre pendant l'opération.
- Ne pas prétendre à une transaction atomique système (macOS ne le permet pas) ; « rollback » best effort sur les fenêtres encore valides.
- Pas de répétition infinie : au plus deux essais pour erreurs transitoires, backoff 100–250 ms.
- Rendre `apply_workspace` **idempotent** : appliquer deux fois la même scène ne crée pas de nouvelle fenêtre et ne change rien si tout est déjà bien placé.
- Pour restore, ne restaurer que les fenêtres qui existent encore et que l'utilisateur n'a pas délibérément déplacées depuis le snapshot. Garder 1 snapshot de retour par scène temporaire ; pas de pile illimitée en V1.

---

## 4. Raccourcis clavier

### 4.1 Fonctionnement

- Back-end basé sur `global-hotkey` ou Carbon `RegisterEventHotKey` (public, peu coûteux, pas de scrutation) pour les raccourcis `modificateurs + touche`.
- Dispatcher `HotkeyId → ActionId`, stocké de façon immuable entre rechargements.
- UI de capture : l'utilisateur appuie physiquement sur la combinaison et voit sa représentation standard macOS.
- Enregistrement en deux étapes : tester la nouvelle combinaison ; si succès, remplacer l'ancienne. Si conflit ou erreur, conserver l'ancienne.
- Détection de collision interne entre raccourcis ; erreur « réservé par une autre application / macOS » lorsque retournée par API. Certains conflits ne sont pas détectables de façon fiable : prévenir et permettre une autre combinaison.
- La répétition d'une touche doit être ignorée pour les actions non répétables.
- `⌥Tab` nécessite test spécifique : interaction éventuelle avec cycle d'app/onglets. Un raccourci ne garantit pas l'accès aux onglets d'autres applications.

### 4.2 Raccourcis d'onglets

Le positionnement des fenêtres ne donne **pas** automatiquement le contrôle du contenu des onglets de Chrome.

- **MVP** `⌥C` : ouvrir `https://chatgpt.com/` dans Chrome (risque d'onglet dupliqué mentionné dans UI) ; option ouvrir l'URL dans le navigateur configuré.
- **Intégration optionnelle** : utiliser Apple Events/AppleScript via module dédié si autorisé par l'utilisateur afin de retrouver/activer une URL Chrome existante ; config `NSAppleEventsUsageDescription`, consentement macOS d'automatisation. Ne pas scanner l'historique ni lire le contenu des pages.
- `⌥Tab` : soit envoyer l'action native de changement d'onglet à l'app sélectionnée si stable, soit marquer « non supporté » pour cette app. Ne pas prétendre qu'il existe une API macOS universelle de sélection d'onglet.

---

## 5. Trackpad — priorité de faisabilité et moteur de gestes

### 5.1 Gate P0 obligatoire : prototype instrumenté

**Objectif :** déterminer si l'app Rust peut observer globalement les contacts d'un trackpad intégré ou Magic Trackpad, en restant en arrière-plan, sur les macOS réellement pris en charge.

Procédure :

1. Créer une `status bar app` minimale signée localement (bundle ID et chemin stables pendant tests TCC).
2. Installer un event tap passif (`kCGSessionEventTap`, `kCGEventTapOptionListenOnly`) sur les types d'événements pertinents ; ne pas utiliser de tap HID root.
3. Pour les événements `gesture`, tester conversion `CGEvent → NSEvent` (équivalent à Hammerspoon) et accès à `allTouches` / `NSTouch.normalizedPosition`, `phase`, `identity`, `device` ; **vérifier les signatures concrètes avec `objc2`** plutôt que présumer qu'elles sont toutes exposées.
4. Capturer seulement, en mémoire, les données de diagnostic minimales : horodatage monotone, device anonyme, id de contact provisoire, phase, x/y normalisés, nombre de doigts ; pas de textes tapés, captures ou identifiants durables.
5. Tester en 1 doigt en déplaçant le pointeur, 2 doigts en scrollant, gestes système 3/4 doigts, après verrouillage/déverrouillage, sortie/retour de veille, et lorsque Chrome/Teams sont au premier plan.
6. Tester avec et sans permissions Accessibilité et Surveillance de l'entrée, demander chaque permission uniquement si macOS l'exige ; noter les différences. Les exigences peuvent varier selon version/catégorie de tap.
7. Mesurer fréquence des événements, éventuelles pertes des phases `began` et `ended`, stabilité du device ID et coût CPU.
8. Tester sur Intel et Apple Silicon si machines disponibles ; noter systématiquement les tests manquants.
9. Documenter résultats dans `docs/trackpad-spike.md` : matrice par OS, matériel, permissions, observations et verdict **GO / PARTIAL / NO-GO**.

**Règle d'honnêteté :** la présence d'`NSTouch` dans une API et le fait que Hammerspoon l'exploite ne garantissent pas que tous les Mac émettent ces événements globalement de la même façon. Aucune feature « geste trackpad à 1 doigt » ne passe `Done` sans vrai test sur matériel.

**Plan de repli si NO-GO :** garder le moteur de reconnaissance pur en Rust et proposer des déclencheurs clavier/hot corner basé sur position du curseur, **explicitement différent du contact physique trackpad**. Ne pas vendre cela comme un geste équivalent. Toute exploration d'API privées (MultitouchSupport.framework) exige un ADR séparé, analyse de risque distribution et accord du propriétaire.

### 5.2 Forme normalisée des événements

```rust
pub enum TouchPhase { Began, Moved, Stationary, Ended, Cancelled }

pub struct TouchSample {
    pub device_key: u64,      // stable uniquement pendant la session
    pub finger_key: u64,      // identité du contact pendant le geste
    pub phase: TouchPhase,
    pub x: f64,               // [0,1], de gauche à droite
    pub y: f64,               // [0,1], de bas en haut
    pub monotonic_ms: u64,
    pub concurrent_fingers: u8,
}
```

**Coordonnées :** `x=0` côté gauche, `x=1` droite ; `y=0` bas, `y=1` haut. Donc un mouvement **vers le bas** doit vérifier `start_y - current_y >= distance`. Ne pas inverser l'axe Y.

### 5.3 Geste initial demandé : haut-gauche → bas

Proposer les valeurs configurables suivantes, **valeurs initiales à calibrer**, et non supposées parfaites :

```json
{
  "id": "swipe_top_left_down",
  "enabled": false,
  "source": "trackpad",
  "fingers": 1,
  "start_zone": { "x_min": 0.00, "x_max": 0.20, "y_min": 0.80, "y_max": 1.00 },
  "direction": "down",
  "min_distance": 0.22,
  "max_perpendicular_drift": 0.12,
  "max_duration_ms": 1500,
  "cooldown_ms": 1000,
  "trigger_policy": "once_per_touch_sequence",
  "action_id": "workspace_meeting"
}
```

- `enabled=false` jusqu'à ce que la calibration/confidence soit confirmée pour éviter les déclenchements accidentels sur le premier Mac.
- Considérer les extrémités x/y comme normalisées et inclusives ; une zone de coin n'est pas équivalente à « toute la bande de gauche ». Prévoir l'option **« bord gauche »** `{x<=0.20, y quelconque}` pour retrouver exactement un éventuel geste Hammerspoon antérieur ; seul le vrai `init.lua` tranche le défaut à migrer.
- Option symétrique `top_right_down` pour workspace navigateur ; paramétrable séparément.
- L'action peut être une scène ou toute action exposée dans le catalogue.

### 5.4 Machine à états d'un geste

```text
IDLE
  ├─ touch began in zone & 1 doigt ─▶ TRACKING
  └─ autre événement ─────────────────▶ IDLE
TRACKING
  ├─ 2e doigt / phase cancelled ──────▶ CANCELLED
  ├─ délai > 1500 ms ─────────────────▶ CANCELLED
  ├─ drift x > 0.12 ──────────────────▶ CANCELLED
  ├─ déplacement y contraire au-delà du seuil de tolérance ─▶ CANCELLED
  ├─ y_start - y_now >= 0.22 ────────▶ FIRED
  └─ ended avant seuil ───────────────▶ CANCELLED
FIRED
  └─ attendre fin de contact ─────────▶ COOLDOWN
COOLDOWN
  └─ après 1000 ms ───────────────────▶ IDLE
CANCELLED
  └─ attendre fin du contact ─────────▶ IDLE
```

Détails techniques :

- Associer `finger_key` + `device_key`, ne pas fusionner les touches d'appareils différents.
- Ne jamais déclencher plus d'une action par séquence de contact.
- Un second doigt, pression/clic ou détection de drag de souris peut annuler si source reconnaissable.
- Les phases initiales manquantes doivent **annuler** le geste, pas être traitées comme nouveau début artificiel.
- Tolérer quelques samples Stationary, x/y bruités et timestamps non strictement croissants ; convertir en horloge monotone dans l'adapter.
- Ne pas bloquer, modifier ou consommer les événements trackpad de macOS ; reconnaissance **passive**.
- Mettre le classifieur dans `src/gestures.rs` (crate unique) et tester seulement les cas à risque.
- Montrer une UI de calibration (« zone détectée », « direction », « action déclenchée »), sans enregistrer tous les déplacements en permanence.

### 5.5 Tests spécifiques gestes

- Déclenche exactement une fois pour mouvement valide de 0.30 vertical et drift 0.05.
- Ne déclenche pas si départ x=0.45, y=0.90.
- Ne déclenche pas si départ y=0.40 (mode coin), même si x=0.1.
- Ne déclenche pas pour mouvement gauche→droite ni bas→haut.
- Ne déclenche pas à 0.19 de descente ; déclenche à 0.23.
- Ne déclenche pas avec 2 doigts, toucher interrompu, plus de 1.5s, ni pendant cooldown.
- Ne double-déclenche pas quand une phase `ended` suit `FIRED`.
- Après retrait/redémarrage d'un trackpad, aucun état d'ancien device ne subsiste.
- Lorsque la session est verrouillée, suspendre les actions ; réarmer correctement après déverrouillage.
- **Validation terrain en P0 :** essayer quelques gestes volontaires et des usages normaux, puis noter les cas réussis/ratés. Avant de déclarer la fonctionnalité fiable pour la vente, élargir les essais (par exemple 15 gestes volontaires et 15 usages courants) si nécessaire. Ne pas en faire une batterie automatisée répétitive.

---

## 6. Scènes/espaces de travail, persistance et édition

### 6.1 Contrat d'un espace de travail

```text
Workspace {
  id: stable UUID,
  name: string,
  description?: string,
  icon?: known-symbol-id,
  is_temporary: bool,
  restore_strategy: "snapshot" | "named_workspace" | "none",
  fallback_workspace_id?: UUID,
  steps: [ WorkspaceStep ],
  enabled: bool,
}

WorkspaceStep {
  target: WindowMatcher,
  on_missing_app: "launch" | "skip" | "error",
  on_missing_window: "skip" | "error" | "wait",
  display: "current" | "primary" | "specific",
  geometry: NormalizedRect,
  focus_at_end?: bool,
}
```

- Appliquer le positionnement sur toutes les fenêtres sélectionnées si explicitement choisi ; autrement une seule fenêtre.
- Lors de la création, l'utilisateur peut sauvegarder la **disposition des fenêtres actuellement visibles** ; ne pas sauvegarder leur texte intérieur ou capture d'écran.
- UI d'édition : liste d'apps, sélecteur de fenêtre, aperçu graphique schématique, zones 50/50, tiers, maximiser, déplacement multi-écrans, aperçu avant application.
- Valider un workspace avant enregistrement : cible inconnue, dimensions hors 0..1, références circulaires, focus final ambigu, scènes supprimées.
- Les fenêtres récemment ouvertes/fermées doivent être re-résolues à chaque activation, jamais stockées via un pointeur AX persistant.

### 6.2 Structure `config.json`

```text
~/Library/Application Support/<BundleIdentifier>/config.json
~/Library/Application Support/<BundleIdentifier>/backups/config-<timestamp>.json
~/Library/Logs/<NomApp>/app.log             # seulement si logs activés
```

- `schema_version=1`, encodage UTF‑8, ID stables ; migration explicite à chaque changement de schéma.
- Écriture temp + `sync_all` + renommage atomique dans même volume, backup de la version précédente ; lecture avec validation avant activation.
- Pas de secrets dans JSON, jamais de token nécessaire pour MVP.
- Au crash de configuration, repli sur dernier backup valide et avertissement ; ne pas écraser le mauvais fichier.
- Import/export JSON : contrôle de schéma, taille max raisonnable, aucun code exécutable ou URL `javascript:`.
- Un seul processus garde le verrou d'édition ; si deuxième lancement, activer préférences de l'instance active au lieu de doubler les raccourcis.

### 6.3 Modèle de configuration d'exemple

Le fichier `examples/default-config.json` joint au handover donne les types et scènes minimales. Codex doit écrire le `serde` correspondant et des tests round-trip lecture/écriture. L'exemple est indicatif ; adapter les UUID et les bundles au Mac réel.

---

## 7. Automatisation contextuelle

### 7.1 Déclencheurs supportés

| Déclencheur | API/Source | Disponibilité |
|---|---|---|
| Raccourci | hotkey manager | MVP |
| Geste trackpad | trackpad adapter si GO | MVP sous gate |
| Ouverture/activation d'application | NSWorkspace | MVP |
| Apparition/disparition de fenêtre | AXObserver si notification supportée | P2 |
| Changement d'écran | NSScreen/app notification | P2 |
| Retour/fin de réunion | machine à états par application | P3 |

Ne pas ajouter de `cron` local : les scénarios événementiels suffisent pour V1.

### 7.2 Règles

Format : `trigger → conditions → actions`. Conditions v1 : `bundle_id`, `window matcher`, `display_count`, `current_workspace`, `is_user_idle` seulement si API stable et autorisée. Actions v1 : `activate_workspace`, `restore_workspace`, `focus_app`, `resize_active_window`, `open_url`. Priorité : commande manuelle explicite > raccourci > geste > règles automatiques, et dernière action manuelle peut inhiber automatiquement une règle pour un délai configuré.

**Anti-loop et anti-flicker :**

- Chaque transition porte `transaction_id` et `origin` (`user`, `hotkey`, `gesture`, `automation`).
- Une notification AX causée par notre propre repositionnement ne doit pas redéclencher la même règle.
- Dédupliquer événements identiques dans 250 ms ; debouncing d'une même fenêtre 300 ms ; expiration 3s si la cible n'apparaît pas.
- Si l'utilisateur bouge manuellement une fenêtre après l'application d'un workspace, ne pas la remettre en place en boucle. Option « respecter les déplacements manuels », activée par défaut.
- Si plusieurs règles se déclenchent, ordre de priorité explicite et journal de décision compréhensible dans l'UI.
- Une règle désactivée ne doit pas laisser d'observers coûteux inutiles ; se désabonner.

### 7.3 Détection de réunion Teams

Ne **jamais** confondre Teams lancé ou au premier plan avec une réunion active.

**V1 manuelle** : scène `Meeting + Browser` / `Meeting + Chat` activée par raccourci/geste, restauration par raccourci/geste.

**P3 expérimentale** :

1. Sur une version précise de Teams, faire un relevé des fenêtres AX avant, pendant, après réunion (métadonnées techniques nécessaires ; pas le contenu des échanges).
2. Identifier un prédicat robuste `MeetingWindowPresent` : bundle Teams + fenêtre meeting avec rôle/subrôle et signature vérifiée. Pas de simple `title.contains("Meeting")` sans preuve.
3. Passer `Idle → Candidate` quand la fenêtre apparaît ; valider après 500–1000 ms + stabilité ; `Candidate → Active`.
4. Au premier `Active`, sauvegarder snapshot et activer scène si l'utilisateur a explicitement opt-in.
5. À disparition de la *fenêtre réunion*, passer `MaybeEnded`, temporiser 2 s, re-vérifier et restaurer si l'utilisateur n'a pas modifié la disposition ou déclenché manuellement une autre scène.
6. Annuler la restauration si une nouvelle réunion apparaît ou si l'utilisateur a désactivé l'automatisation.
7. Si classification incertaine : journaliser `unknown`, **ne pas activer** la scène ; laisser déclenchement manuel.
8. Toute signature Teams doit être documentée et testée sur différentes versions ; options de configuration accessibles dans UI.

**Remarque :** Teams sait ouvrir le chat et les contenus de réunion dans des fenêtres séparées (documentation Microsoft) ; l'app doit sélectionner ces fenêtres existantes. Elle ne promet pas de créer des pop-outs automatiquement sans API publique/chemin UX testé.

### 7.4 Autres applications de réunion

Créer une interface `MeetingDetector` générique mais **implémenter uniquement Teams** avant la V1 commerciale. Zoom/Google Meet (dans navigateur) ensuite, sur preuves. Pas de dépendance à des API réseau d'entreprise pour le cœur du produit.

---

## 8. UI/UX native : spécifications écran par écran

### 8.1 Barre de menus (toujours disponible)

- Icône sobre, statut de permission en cas de blocage.
- Menu : `Appliquer un espace...` (avec les espaces actifs + leurs raccourcis), `Restaurer disposition`, `Déplacer fenêtre`, `Gérer les espaces`, `Raccourcis`, `Gestes`, `Automatisations`, `Diagnostics`, `Ouvrir au démarrage`, `Quitter`.
- `Activer/désactiver temporairement les automatisations` avec reprise explicite.
- Aucune popup lors des actions réussies courantes ; confirmation discrète optionnelle uniquement lors du changement de scène.

### 8.2 Onboarding

1. Écran de principe : l'app pilote les fenêtres et les événements locaux.
2. Bouton Accessibilité ; afficher état et méthode de réessai.
3. Tester déplacement d'une fenêtre de démonstration appartenant à l'app, puis d'une app tierce avec consentement.
4. Configurer premiers raccourcis.
5. Calibrer geste trackpad **uniquement si le spike a réussi et les permissions sont OK**.
6. Créer scénario en enregistrant la disposition courante.
7. Choisir lancement à l'ouverture de session (désactivé par défaut).

### 8.3 Fenêtre « Espaces de travail »

- Colonne gauche : scènes, recherche locale, nouveau, dupliquer, supprimer (confirmation).
- Centre : aperçu zones des écrans et fenêtres (rectangles, labels, pas de screenshot).
- Droite : matcher de fenêtres, app cible, écran, position, comportement si fenêtre absente, focus final.
- Boutons `Tester` (appliquer et proposer `Restaurer`), `Sauvegarder`, `Annuler`.
- Interaction simple : bouton `Enregistrer la disposition actuelle` qui pré-remplit les étapes.

### 8.4 Raccourcis / Gestes

- Tableau : action, raccourci, état conflit, activer/désactiver.
- Gestes : schéma du trackpad, zone de départ redimensionnable, direction, distance, tolérance latérale, nombre de doigts et action ciblée.
- Calibration : flux visuel des coordonnées récentes pendant une courte session **volontaire**, compteur de tentatives, faux positifs; purger à la fermeture.

### 8.5 Automatisations

- Éditeur de règles basique « Quand [événement], si [conditions], alors [scène] » ; activation/désactivation règle par règle.
- `Pourquoi une règle ne s'est pas exécutée ?` → dernier code d'erreur, permission manquante, matcher non résolu, conflit de priorité.
- Ne pas permettre des expressions exécutables, des scripts, ou des boucles généralisées.

### 8.6 Diagnostics

- Permission Accessibilité, permission de surveillance de l'entrée si nécessaire, Apple Events si intégration Chrome activée.
- Versions macOS et app, architecture processeur, nombre d'événements par type, dernière erreur AX (sans titres sensibles par défaut).
- Export diagnostic anonymisé opt-in, bouton `Copier informations de support`.

---

## 9. Permissions, sécurité, confidentialité et distribution

### 9.1 Permissions macOS

| Fonction | Permission / mécanisme | Expérience utilisateur |
|---|---|---|
| Lire/placer fenêtres tierces | Accessibilité via `AXIsProcessTrustedWithOptions` | Demander après action explicite avec explication |
| Raccourcis globaux simples | Carbon/hotkey natif | Ne pas demander automatiquement la permission clavier globale si pas utile |
| Événements globaux trackpad | Vérifier TCC selon type de tap/OS ; peut impliquer Surveillance de l'entrée | Informer, demander seulement après activation du geste |
| Contrôler onglet Chrome via Apple Events | Automatisation + `NSAppleEventsUsageDescription` | Fonction optionnelle désactivée par défaut |
| Démarrage automatique | `SMAppService` sur macOS 13+ ; compatibilité 11/12 à traiter explicitement | Bouton on/off et statut vérifiable |

Ne pas automatiser la modification des permissions système via commandes shell. Ne pas fournir de procédure pour contourner les profils d'entreprise (MDM). Sur un Mac supervisé, tenir compte des restrictions IT. La distribution commerciale doit être testée sur un Mac personnel avec droits appropriés.

### 9.2 Sécurité par défaut

- Aucune connexion réseau nécessaire à l'application de scènes.
- Aucune télémétrie par défaut ; aucune collecte de frappes, contenus de fenêtre, mots de passe, captures écran, messages Teams.
- Les titres de fenêtres ne sont utilisés **qu'en mémoire pour le matching** ; pas de logs de titres sans consentement de diagnostic.
- Validation des imports JSON et noms d'applications ; pas d'exécution de code utilisateur.
- Signatures stables entre versions pour ne pas multiplier les demandes d'autorisation TCC ; pas de builds qui changent constamment de bundle ID.
- Audit des dépendances Rust (`cargo audit`/equivalent actuel vérifié par Codex), notices licence si code tiers repris.

### 9.3 Signature et livraison

- Build `x86_64-apple-darwin` + `aarch64-apple-darwin` ; universel avec `lipo` ou deux distributions identifiées.
- `.app` valide : `Info.plist` (bundle ID, `LSUIElement`, minimum system version réellement supporté, usage descriptions), icône, ressources, exécutable.
- Signature Developer ID, Hardened Runtime, horodatage sécurisé ; notarisation via `xcrun notarytool`, stapling, vérification `codesign --verify`, `spctl --assess`.
- `.dmg` téléchargeable ; versionnement semver ; canal de mise à jour plus tard, pas de service de fond supplémentaire en V0.1.
- App Store hors périmètre initial à cause des contraintes sandbox et de compatibilité des automatisations ; distribution directe prioritaire.
- Commercial : essai 14 jours **si l'on décide de le lancer**, paiement unique envisagé ; ne pas intégrer moteur de licence avant d'avoir un prototype réellement utile.

---

## 10. Qualité, benchmarks, observabilité

### 10.1 Objectifs mesurables, non garantis

| Métrique | Cible V0.1 Release | Méthode |
|---|---:|---|
| RSS au repos après 10 min | idéal ≤40 Mo ; gate souple ≤60 Mo | `ps`, Activity Monitor ; médiane de 3 mesures sur machine de test |
| CPU au repos | moyenne <0,5 % d'un cœur sur 5 min | sampling `top`/Instruments ; aucune boucle polling rapide |
| Déclenchement de raccourci → réception moteur | p95 <50 ms | timestamps monotones avec instrumentation locale |
| Raccourci → position vérifiée | viser <400 ms pour une fenêtre déjà ouverte et coopérative | quelques essais sur Mac pilote, à affiner avant release |
| Apply scene 2 fenêtres déjà ouvertes | viser <700 ms | quelques essais représentatifs sur Mac pilote |
| Démarrage du moteur | <1 s sur Mac test après lancement process | instrumentation ; sans attendre lancement Teams/Chrome |
| Fuite mémoire | pas de croissance évidente pendant l’usage quotidien | Observer le processus ; profiler seulement si suspicion réelle |
| Faux positifs geste | aucun faux positif observé dans un court essai représentatif | validation manuelle terrain |

Ces chiffres ne constituent pas des promesses marketing ; réviser seulement à partir de mesures avec modèle Mac, OS et version de l'app. Les mesures d'un Mac M5 ne suffisent pas à promettre de bonnes performances sur Intel ancien.

### 10.2 Diagnostics techniques

- Utiliser `tracing` avec spans `shortcut`, `rule_evaluation`, `workspace_apply`, `ax_call`, `gesture_classification` ; par défaut contenu sensible masqué.
- Éviter les allocations sur chaque `TouchSample`, limiter la fréquence des updates UI de calibration (par ex. max 30 Hz, seulement pendant calibration).
- Event tap callbacks `O(1)` ou `O(nombre de doigts)`, jamais de matching complet AX.
- Gérer `kCGEventTapDisabledByTimeout` / `kCGEventTapDisabledByUserInput` : possibilité de réactivation contrôlée, afficher état si impossible ; pas de boucle agressive.
- Conserver uniquement les derniers 100 événements **structurels** en mémoire pour diagnostics (pas de keycodes frappés ni de titres par défaut).

### 10.3 Politique de tests minimaliste

Le but est de détecter les erreurs importantes, **pas de maximiser le nombre de tests**.

- `cargo fmt --check` et `cargo check` après modification substantielle ; `cargo test` sur les fonctions pures ajoutées/modifiées.
- **1 à 3 tests ciblés par logique non triviale**, couvrant un cas normal, une limite et éventuellement une régression. Ne pas créer de fichiers de tests vides ou d’énormes tableaux de cas.
- Pour chaque fonction macOS (AX, raccourci, trackpad, scène), **une vérification manuelle représentative sur le vrai Mac**, avec constat factuel. Ajouter des vérifications uniquement en cas de bug ou d’hypothèse à départager.
- Aucun framework E2E, mock générique d’AppKit, suite de smoke tests automatisés, snapshot visuel ni pipeline CI complexe pour P0–P3.
- Avant une version commercialisée : tester sur Intel et Apple Silicon, ainsi que sur chaque macOS officiellement annoncé, sans prétendre couvrir ce qui n’a pas été exécuté.

### 10.4 Mesures de performance proportionnées

Les cibles du §10.1 servent de repères et non de gates automatisés pour chaque commit. Une mesure de mémoire, CPU et latence sur le Mac pilote en version release suffit pour le premier jalon. Répéter les mesures seulement après un changement significatif ou lorsqu’une régression est suspectée. Les campagnes longues ou instruments avancés sont réservés à un problème reproductible ou à la préparation commerciale.

---

## 11. Roadmap d'implémentation (ordre impératif)

Chaque étape doit produire un **commit isolé**, un petit rapport dans `docs/progress.md` et une démonstration utilisable. **Ne pas commencer l'étape suivante si un critère GO bloquant échoue.**

### P0 — Recherche et spikes techniques [bloquant]

**P0.1** Ouvrir/cloner le dépôt existant `Stem0794/kigi-windows` ; initialiser un unique crate Rust minimal dans ce dépôt (sans workspace multi-crates). Cibler macOS 11 et l’architecture de la machine disponible ; AppKit status bar minimale. Vérifier Intel/ARM à une étape de validation ultérieure si le matériel ou la toolchain n’est pas accessible.  
**P0.2** Vérifier `AXIsProcessTrustedWithOptions`, lister Chrome/Teams, lire les positions et déplacer une fenêtre avec restauration.  
**P0.3** Enregistrer un raccourci global `⌥T` depuis le processus agent.  
**P0.4** Créer spike trackpad global décrit §5.1, test manuel réel (go/no-go).  
**P0.5** Tester le packaging `.app`, stabilité TCC sur 2 lancements ; vérifier que l'app fonctionne sans Hammerspoon actif.  
**P0.6** Vérifier que la barre de menus et une action native `objc2 AppKit` fonctionnent ; ne pas construire de prototype UI supplémentaire en P0.  
**Livrables :** `docs/trackpad-spike.md` concis, `docs/progress.md`, démonstration menu bar + raccourci + fenêtre déplacée + geste si GO. Créer la matrice de compatibilité seulement quand plusieurs OS/Mac auront été réellement testés.  
**Done :** les limites techniques sont documentées, aucune feature non testée n'est « validée ».

### P1 — Cœur fenêtres, hotkeys, actions personnelles

**P1.1** Modèles `Rect`, `NormalizedRect`, actions et erreurs dans le crate unique ; quelques tests ciblés de géométrie.  
**P1.2** Adapter AX : apps/fenêtres, sélection, position, taille, activation, min/fullscreen, gestion erreurs.  
**P1.3** Display service : visible frames, conversion AX/AppKit, fallback écran.  
**P1.4** Hotkey manager : registration/deregistration et conflits.  
**P1.5** Implémenter actions personnelles `⌥T`, `⌥B`, `⌘⇧↓`, `⌘⇧↑`; `⌥C` URL basique.  
**P1.6** Menu bar minimal et logs d'erreur UX.  
**Livrable :** remplacement pratique des principaux raccourcis Hammerspoon.  
**Done :** commandes fonctionnent sans Hammerspoon ; jamais de fenêtre dupliquée pour un simple focus ; p95 et RSS mesurés.

### P2 — Gestes et déclencheurs configurables

**P2.1** Adapter trackpad conforme au résultat du spike.  
**P2.2** Module `gestures` avec classifieur simple et quelques tests de transitions critiques.
**P2.3** Zone haut-gauche → bas, mapping à une action ; calibration opt-in.  
**P2.4** Gestion devices, rebond, cooldown, veille, et permission révoquée.  
**P2.5** Gestion de gestes multiples déclaratifs (zones/direction) mais pas de reconnaissance arbitraire ML.
**Livrable :** geste personnel testé et fiable.  
**Done :** gestes essayés manuellement sur le Mac cible, sans déclenchement accidentel observé lors des usages courants ; si NO-GO, désactiver simplement l’option et afficher « non disponible », sans architecture spéciale de fallback.

### P3 — Workspaces et restauration

**P3.1** `Workspace` et matchers, JSON schema versionné + fichiers atomiques.  
**P3.2** Capture d'état présent / sauvegarde de configuration.  
**P3.3** Apply multi-fenêtres avec lancement et attente bornée, transaction et rapports partiels.  
**P3.4** Scènes `Work`, `Meeting+Browser`, `Meeting+Chat` ; ordre de focus et rappel.  
**P3.5** Restore de snapshot et respect des modifications manuelles.  
**P3.6** Idempotence, repli écran, erreur app absente.  
**Livrable :** workflows Teams/Chrome utilisables tous les jours.  
**Done :** quelques cycles manuels représentatifs d’application/restauration réussissent sans duplication ni clignotement gênant ; un bug constaté reçoit un test de régression ciblé.

### P4 — UI utilisable sans code

**P4.1** Préférences AppKit : onglets Spaces/Raccourcis/Gestes/Automations/Permissions.  
**P4.2** Éditeur visuel des scènes : composants rectangles et drag/resize ; préférer contrôles natifs.  
**P4.3** Bouton « Enregistrer disposition actuelle », édition, duplication, suppression, aperçu.  
**P4.4** Capture des raccourcis et détection conflits.  
**P4.5** Onboarding permissions et calibration gestes.  
**P4.6** Rendu localisé FR/EN si infrastructure simple ; anglais non bloquant pour V0.2.  
**Livrable :** un utilisateur non technique peut créer une scène, l'assigner à un raccourci et la déclencher.

### P5 — Automatisations contextuelles

**P5.1** Observer NSWorkspace et AX seulement pour les apps et règles pertinentes.  
**P5.2** Rule engine, priorités, anti-boucles, suspension manuelle, logs décisionnels.  
**P5.3** Activation sur ouverture/activation app, apparition fenêtre.  
**P5.4** Test et restauration lors du branchement/débranchement écran.  
**P5.5** Détecteur Teams expérimental (signature vérifiée + FSM) ; conserver activation manuelle toujours.  
**Livrable :** règles locales sûres, avec réunion automatique seulement si signature suffisamment robuste.

### P6 — Qualité sur ancien Mac

**P6.1** Mesure RSS/CPU/latence release sur ARM et Intel.  
**P6.2** Profiler idle & gestes ; supprimer polling et allocations évitables.  
**P6.3** Tester macOS 11 ; documenter les API indisponibles et conditionner l'activation plutôt que planter.  
**P6.4** Vérifier 24 h d'utilisation de test, veille/reprise, second lancement, crash/restart.  
**P6.5** Corriger P0/P1 de fiabilité avant polish.  
**Livrable :** `docs/performance-report.md`, `docs/compatibility-matrix.md` complétés.

### P7 — Préparation vente

**P7.1** Nom/identité, icônes originales, mentions légales, licences de dépendances.  
**P7.2** `.app` + universal binary, signature Developer ID, hardened runtime, notarisation, `.dmg`.  
**P7.3** Vérification install fraîche sur un Mac sans Hammerspoon/compilateur Rust.  
**P7.4** Documentation utilisateur, aide permissions, guide scenarios et limitations.  
**P7.5** Mécanisme de licence et update seulement après pilote ; ne pas bloquer les tests personnels avec activation payante.  
**P7.6** Landing page et essais, hors scope technique du MVP.  
**Done :** package autonome distribué, vérifié Gatekeeper ; aucune promesse de compatibilité non testée.

---

## 12. Stratégie de versions et gates

| Version | Contenu | Critère de lancement |
|---|---|---|
| `0.0.1` | Spikes P0, status bar, AX, hotkey, trackpad diagnostic | Démo réelle et rapport de faisabilité |
| `0.1.0` | Raccourcis personnels et redimensionnements P1 | Peut remplacer Hammerspoon pour ces usages précis |
| `0.2.0` | Gestes (si GO), scènes/restore P2–P3 | Usage quotidien sans régression des scénarios |
| `0.3.0` | Éditeur visuel P4 | Scénarios créables sans JSON |
| `0.4.0` | Règles contextuelles, Teams expérimental P5 | Pas de réorganisation surprise |
| `0.9.0` | Tests Intel/macOS ancien, perf, signing P6–P7 | Pilote de plusieurs utilisateurs |
| `1.0.0` | Release payante | Support, installation et limitations documentés |

**Important :** `0.2.0` ne doit pas être présentée comme « geste supporté » si le spike est NO-GO ; prioriser une fonctionnalité qui marche plutôt qu'un numéro de version.

---

## 13. Critères d'acceptation finaux / checklists Codex

### Fonctionnels

- [ ] Lancement comme app macOS depuis Finder, icône menu bar, sans terminal ni Hammerspoon.
- [ ] `⌥T` active Teams une seule fois, lancement si absent.
- [ ] `⌥B` ouvre/active Chrome et le positionne sur zone visible 100 %.
- [ ] `⌘⇧↓` positionne la fenêtre active à droite 50 % (sous réserve audit init.lua).
- [ ] `⌘⇧↑` maximise géométriquement sans provoquer fullscreen natif.
- [ ] `⌥C` ouvre ChatGPT dans Chrome ; option « retrouver onglet » explicitement conditionnelle aux permissions, non requise pour MVP.
- [ ] Création et édition d'une scène via UI.
- [ ] Teams/Chrome 50/50 s'applique et se restaure correctement.
- [ ] Deux fenêtres Teams meeting/chat sont sélectionnées seulement si présentes et identifiables.
- [ ] Aucun déplacement arbitraire en cas d'ambiguïté du matcher.
- [ ] Une scène ne ferme jamais les fenêtres non ciblées.
- [ ] Le geste haut-gauche vers bas active une scène **si spike GO**, sans bloquer les gestes standards.
- [ ] Aucun double-déclenchement de gestes et pas de réorganisation en boucle.
- [ ] Suspensions/règles contextuelles respectent actions manuelles.
- [ ] Un second lancement ne crée pas un deuxième gestionnaire de hotkeys.

### Fiabilité et conformité

- [ ] Permission refusée → message clair, app ne plante pas.
- [ ] Changement de moniteur → fenêtres contraintes à l'écran disponible.
- [ ] Veille/retour → rebranche observers et hotkeys si besoin.
- [ ] App tierce inaccessible → `PartiallyApplied` ou `Failed` documenté.
- [ ] Les quelques tests utiles de géométrie, gestes, règles et persistance passent avec `cargo test` ; CI facultative tant que non justifiée.
- [ ] Aucun réseau ni télémétrie implicite.
- [ ] Intel + ARM construits, min macOS vérifié.
- [ ] Benchmark signé Release documenté avec modèle/os/date.
- [ ] `.dmg` notarisé, Gatekeeper vérifié.
- [ ] Hammerspoon reste installé tant que la parité n'est pas démontrée.

### Clôture de chaque ticket

Codex doit systématiquement fournir : `1) fichiers modifiés`, `2) comportement observable`, `3) tests lancés + résultats`, `4) limitations non résolues`, `5) prochain sous-ticket`, sans ajouter de fonctionnalités hors scope.

---

## 14. Risques classés et mitigations

| Risque | Gravité | Mesure de réduction |
|---|---|---|
| Geste 1 doigt global inaccessible ou peu fiable | Très élevée | Spike P0, gate GO, fallback explicite et non équivalent |
| Application Teams change sa structure de fenêtres | Élevée | Adapter avec signature versionnée, détection incertaine désactivée, mode manuel |
| Ancien macOS incompatible avec un crate/API | Élevée | Matrice P0 et feature flags, tests précoce sur Intel/macOS 11 |
| Autorisations TCC déroutantes | Élevée | Bundle ID stable, onboarding explicatif, diagnostics, signature cohérente |
| Interférences avec Hammerspoon pendant migration | Moyenne | Désactiver temporairement hotkeys de l'un des deux lors du test ; jamais modifier sans accord |
| Gestes involontaires lors d'usage normal | Élevée | Coin étroit, mouvement directionnel, 1 seul déclenchement, calibration, disabled par défaut |
| AX bloquant/ralentissant | Moyenne | Timeouts, queue série, aucune opération AX dans callback événement |
| Redimensionnement limité par app | Moyenne | Relire frames, retour partiel, pas de retry infini |
| UI AppKit 100 % Rust coûteuse à maintenir | Moyenne | Spike P0 ; documenter recours éventuel à petite couche SwiftUI uniquement si Rust-first devient disproportionné, sans changer moteur Rust |
| Capture de contenus sensibles accidentelle | Élevée | Aucun screenshot, pas de logs titres par défaut, tests confidentialité |
| Écrans/Spaces hétérogènes | Élevée | Workspaces = dispositions, pas Mission Control ; modes de fallback, pas d'API privée |

---

## 15. Sources techniques à consulter au début de l'implémentation

1. Hammerspoon MIT et code de la conversion événement → `NSEvent` → touches : https://github.com/Hammerspoon/hammerspoon ; https://github.com/Hammerspoon/hammerspoon/blob/master/extensions/eventtap/libeventtap_event.m
2. Référence Hammerspoon Touch : https://www.hammerspoon.org/docs/hs.eventtap.event.html
3. Apple AX : https://developer.apple.com/documentation/applicationservices/axuielement_h
4. Apple set AX frame : https://developer.apple.com/documentation/applicationservices/1460434-axuielementsetattributevalue
5. Apple permission AX : https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions
6. Apple events tap : https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate%28tap%3Aplace%3Aoptions%3Aeventsofinterest%3Acallback%3Auserinfo%3A%29
7. Apple `NSEvent` touches : https://developer.apple.com/documentation/appkit/nsevent/touches%28matching%3Ain%3A%29
8. Apple screen `visibleFrame` : https://developer.apple.com/documentation/appkit/nsscreen/visibleframe
9. Apple `NSWorkspace` lifecycle : https://developer.apple.com/documentation/appkit/nsworkspace/didactivateapplicationnotification
10. Rust `objc2-app-kit` : https://docs.rs/objc2-app-kit/latest/objc2_app_kit/
11. Rust `global-hotkey` : https://docs.rs/global-hotkey/latest/global_hotkey/
12. Apple macOS startup : https://developer.apple.com/documentation/servicemanagement/smappservice
13. Apple notarisation : https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
14. Apple Apple Events usage description : https://developer.apple.com/documentation/bundleresources/information-property-list/nsappleeventsusagedescription
15. Microsoft Teams meeting pop-outs : https://support.microsoft.com/en-us/teams/meetings/multitask-during-a-microsoft-teams-meeting

**Instruction :** traiter les docs et les signatures crate comme source de vérité au moment de coder. Si une hypothèse du handover est contredite par l'API réelle, enregistrer un ADR avec explication et une solution simple, ne pas bricoler une API privée sans accord.

---

## 16. Premier prompt à donner à Codex

```text
Dépôt UNIQUE : https://github.com/Stem0794/kigi-windows
Travaille dans ce dépôt existant, sur son arborescence réelle, et n’en crée aucun autre.
Lis AGENTS.md, HANDOVER_CODEX.md et examples/default-config.json.

EXIGENCE PRINCIPALE : implémentation simple, directe et minimaliste.
Ne fais PAS 10 000 smoke tests ou une vaste infrastructure E2E. Pas
sur-ingénierie, pas de multi-crates initial, pas de frameworks, workers,
services ou abstractions anticipés. Un seul crate Rust, quelques modules,
le minimum de dépendances. Tests unitaires ciblés uniquement pour les
logiques à risque ; vérifications manuelles courtes des API macOS.

Travaille sur P0 UNIQUEMENT :
1) app macOS native barre de menus, Rust + objc2/AppKit ;
2) permission Accessibility, lecture et déplacement d'une fenêtre ;
3) un raccourci clavier global ;
4) prototype en lecture seule du geste trackpad haut-gauche vers bas ;
5) court rapport factuel GO/PARTIAL/NO-GO sur le trackpad.

Cible macOS 11, Intel + Apple Silicon à terme, sans annoncer une
compatibilité non testée. Ne modifie jamais ~/.hammerspoon/init.lua.
À la fin, donne le diff/commit, les commandes réellement exécutées,
les tests utiles et résultats, les limites et le prochain petit pas.
Ne lance pas P1.
```

**Fin du handover.**
