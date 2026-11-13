# Studio — phases 0 et 1

Base : main `67f0305` (0.5.4), retour utilisateur du 13 novembre : activation
Teams/Chrome et gestes corrigés. Branche `ui/studio-shell`, revue avant fusion.
Contrats de référence : KIGI_STUDIO_01_CAHIER_DES_CHARGES.md et
KIGI_STUDIO_02_PLAN_TECHNIQUE.md (13 novembre 2026, documents utilisateur).

## Branchements

- `Controller::open_main_ui` route Dock/menu vers la dernière UI choisie.
  `open_profiles` et `Preferences` restent le secours et le défaut historique.
- `--studio-ui` ouvre Studio ; `--native-ui` force les réglages classiques.
  « Ouvrir Studio… » est disponible dans les deux menus Kigi. Aucun cutover
  automatique avant les phases d’édition, application et parité.
- `State::studio` conserve une seule fenêtre NSWindow/WKWebView réutilisable.
  Fermer ne détruit aucun hotkey/tap/snapshot ; ⌘Q et fin de boucle nettoient
  le handler, le delegate et la vue. La vue est conservée fermée pour réouverture.
- Le bridge utilise `app.bootstrap`, `spaces.list`, `permissions.get`,
  `diagnostics.summary`, et l’action UI `app.openNativeFallback` uniquement.
  Pas d’actions AX, d’écriture config, de nouveau mapping ou recorder dans Studio.
- Les profils sont relus avec `config::load` (schéma 1/2, validation existante),
  sans conversion ni écriture. Un backup est signalé. Pas de JSON frontend
  persistant. `personal_layout` et `LayoutSnapshot` restent inchangés.

## Frontière WebKit

Une dépendance `objc2-web-kit` 0.3.2 avec features ciblées. Assets `ui/` copiés
sous Resources/studio. Lecture limitée à ce répertoire. Datastore non persistant.
CSP locale sans connexion réseau, frames, objets ou contenu tiers. Toute navigation
est refusée sauf le document d’entrée exact et principal. Le handler vérifie aussi
l’identité de sa WKWebView et l’URL du frame avant parsing.

Enveloppe JSON v1 sérialisée en NSString dans les deux sens ; le petit client JS
la décode et expose une Promise d’objet. Charge ≤64 Kio, ID borné, paramètres
vides stricts, allowlist Rust, erreurs uniformes. Aucun JavaScript fourni par UI
n’est exécuté ; l’unique expression DOM de contrôle est fixe et test-only.
Valeurs variables toujours textContent. Pas de serveur/CDN/localStorage.

## Tranche livrée / suite

Accueil, liste des vrais espaces avec rectangles normalisés fidèles, autorisations
et diagnostics en lecture seule. Sidebar/canvas/inspecteur ; tiroir Propriétés
sur petite fenêtre. Actualisation volontaire, aucun polling permanent. Chargement
impossible/processus WebKit interrompu : secours AppKit. Les réglages/actions non
portés restent accessibles uniquement via les réglages classiques.

Phase 2 suivante : éditeur 1–99 %, drag/clavier, brouillon, sauvegarde atomique et
conflits. Phase 3 : appliquer/restaurer. Pas de faux boutons d’édition ou d’application.

## Mesures et validation

Baseline native 0.5.4 du runner macOS 15.7.9 Intel : RSS 38 174 720 puis
38 113 280 octets (run43407827932). Temps d’ouverture natif non instrumenté ;
ne pas le présenter comme mesuré. Le Mac pilote avait environ92 Mio dans ses
logs0.5.3 : pas une comparaison avant/après Studio sur le même environnement.

Studio relève délai jusqu’au bootstrap et RSS Rust. La mémoire des auxiliaires
WebKit reste explicitement non mesurée. Comparaison M5 physique à demander au
retour d’essai, pas de promesse de coût mémoire.

Tests ciblés : protocole/autorité purs, typage ARM, chargement WKWebView réel,
bootstrap + DOM rendu + fermeture/réouverture sans retirer les hotkeys et sans
changer config.json. Les quatre scénarios de focus 0.5.4 restent conservés.
Le navigateur Chromium du conteneur refuse son socket de démarrage : aucun
résultat visuel Chromium n’est revendiqué ; validation native sur runner prévue.

Validation finale : run43416541318, tentative2 réussie sur le même commit
`1b479adb9db99dbf1b442063c3d9d58ba6adcd90` : 25 tests macOS, deux relances,
WKWebView/bridge/DOM natifs et cycle fermer/réouvrir, configuration inchangée,
quatre cas de focus. RSS Rust : classique37 343 232 octets, Studio42 467 328.
Ce relevé de processus n’inclut pas les auxiliaires WebKit. La première tentative
avait passé Studio mais rencontré un timeout AX250ms pendant une maximisation
plein écran ; la seconde a passé sans changement de source ni assouplissement.
Ce risque temporel reste visible dans l’historique du run et à surveiller sur Mac.

Package0.6.0/build20 : 868101 octets, SHA256
`e39663841f736ec10bde6ab0776dc7aeaeeac20b30494611e687848753411d74`.
Intel/ARM et ressources locales validés. Revue visuelle/VoiceOver et essais M5
physiques restent à réaliser ; aucun résultat Chromium ou terrain inventé.

## Phase2 — éditeur, 0.6.1

Branche ui/studio-editor, basée sur la phase1 encore en revue. Édition du nom,
des deux apps via NSOpenPanel, proportions1–99 %, presets et séparateur clavier/
pointer capture. Aucun appel de sauvegarde pendant le drag. Création en brouillon,
renommage, copie et suppression confirmée ; aucun placement AX dans cette tranche.
Les coordonnées libres et anciennes proportions fractionnaires restent intactes
jusqu’au choix explicite de deux colonnes. Schéma2 inchangé : NormalizedRect.

Ajouts bridge typés : spaces.create/update/duplicate/delete/selectApp ; état UI
app.setDraftDirty, app.closeStudio et app.quit. Les mutations portent expectedRevision.
Revision opaque de session associée aux octets exacts du primaire : comparaison
à la lecture puis juste avant remplacement atomique, après backup. Primaire
invalide ou plus récent : pas d’écriture, backup affiché en lecture seule.
Cela ne constitue pas un verrou universel contre un éditeur externe qui écrit
simultanément entre la dernière comparaison et le rename ; aucun verrou imposé
aux autres logiciels. Conflit connu : conserver le brouillon et proposer Actualiser.

Les requêtes de split peuvent porter leftPercent transitoire : Rust valide la
cohérence avec les deux rectangles et les reconstruit sans aller-retour en pixels.
Ce champ n’est jamais persisté dans config.json. Les profils libres n’envoient pas
de proportion et gardent leurs rectangles exacts.

Changer d’espace/actualiser/fermer : Enregistrer/Ignorer/Annuler. La fermeture native
passe par windowShouldClose et un callback JS fixe ; elle est acquittée seulement
après traitement du brouillon. ⌘Q suit le même parcours quand Studio est chargé.
Un NSOpenPanel volontaire ne subit pas le timeout réseau de8s du bridge ; les
autres requêtes restent bornées. Aucun eval fourni par UI, shell ou chemin libre.

Contrôles natifs ciblés : config temporaire isolée uniquement sous le mode interne
--studio-editor-check, vraie WebView/bridge/save/relaunch ; aucun fichier des espaces
du pilote modifié. Tests purs de géométrie, CRUD, conflit, backup, conservation libre.
Le contrôle de focus précède maintenant l’ouverture des processus WebKit : fixtures
indépendantes, mêmes assertions et limite AX250ms, aucune régression masquée.
