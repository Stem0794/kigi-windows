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
