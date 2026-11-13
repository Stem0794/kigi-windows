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

### Validation phase2

Run43423531866, sourceff974dbee, macOS15.7.9 Intel :27 tests, quatre cas de focus,
chargement/DOM Studio, parcours éditeur isolé et relecture après relance passent.
Le parcours couvre conversion explicite schema1 libre→split35/65/schema2, sauvegarde,
annulation49→35, incrément clavier, brouillon à la navigation/fermeture, duplication,
suppression confirmée et conflit sans écrasement. Les assertions ont également
repéré puis protégé la réactivation des contrôles après Actualiser.

Glissement physique, vrai NSOpenPanel et VoiceOver restent des essais Mac pilote ;
les événements clavier du test sont des événements DOM locaux, pas une injection
système ou un essai de geste global. Le test de fermeture appelle le même point
JS fixe que le delegate natif ; le bouton de titre macOS reste à essayer physiquement.
Application/restauration depuis Studio et parité système non incluses dans cette
phase. Le package0.6.1/build21 n’est pas fusionné sur main.

## Phase3 — application/restauration, 0.6.2

Branche ui/studio-apply basée sur la phase2 encore en revue. Nouveau spaces.apply
avec profileId + expectedRevision, et spaces.restore sans paramètres. Le backend
refuse les brouillons sales, permissions absentes, dialogue occupé et snapshot déjà
actif. Le profil doit correspondre aux octets de la dernière lecture, contrôlés
avant l’inventaire puis après l’éventuel sélecteur natif, avant tout placement.
Apps absentes, choix annulé ou fenêtres ambiguës : pas de fenêtres de secours.
Le sélecteur existant est partagé avec les réglages classiques. Il peut rester
ouvert volontairement ; le client ne lui impose pas le timeout ordinaire de8s.

Kernel partagé apply_saved_layout/Meeting et LayoutSnapshot : issue typée, normalisée
sur la zone visible mémorisée, requested/actual nullable, restored/ignored/pending,
restoreAvailable et activationConfirmed distinct de la réussite géométrique.
Lecture AX après les deux écritures ; tolérance historique8pt, aucune supposition
basée sur un booléen AppKit. La fenêtre choisie est relevée explicitement ; aucun
choix implicite d’une autre fenêtre de la même app. L’activation non confirmée est
mentionnée même lorsque les positions sont confirmées. Ratios demandés inchangés.

Restauration : même politique existante restore_decision, aucune pile d’undo ;
ignorer une fenêtre fermée ou déplacée manuellement. Les erreurs/incertitudes gardent
le snapshot. Le snapshot n’est effacé qu’après résolution de toutes les entrées.
Le bouton Restaurer est global et fonctionne même si le profil a été supprimé.
Les modifications du brouillon ne sont pas effacées par une restauration de fenêtres.
Actualiser relit l’état partagé après une action classique ; le backend reste l’autorité.

UI : Appliquer propose Enregistrer et appliquer/Annuler sur un brouillon sale.
Les boutons empêchent les opérations simultanées ; le backend refait les contrôles.
Le retour montre le profil concerné, les largeurs demandées/constatées et un résultat
partiel si nécessaire. Les données absentes restent Non mesuré, jamais0 ou un succès.
Les appels AX restent ceux du moteur principal, synchrones et bornés par appel ;
le dialogue volontaire est modal. Aucun nouveau thread de contrôle ou backend parallèle.

Validation ciblée prévue : deux fixtures AppKit dédiées, cas normal35/65 et cas
contentMinSize55 % pour une demande35 %. Appliquer, refuser un second snapshot,
restaurer ; comparer les vrais CGWindowBounds après restauration aux positions
initiales. Sauvegarde+application explicite et annulation préalable dans le cas normal,
aucune écriture de config dans le cas contraint. Les autres contrôles natifs restent.

### Validation phase3

Source1b91c64, run43434129431 réussi :27 tests natifs et tous contrôles antérieurs.
Deux vrais cas AppKit :35/65 ; et minimum55 % donnant55/65/partial pour la demande
35/65. Les CGWindowBounds des deux fixtures ont été observés pendant le placement,
comparés aux largeurs affichées, puis comparés à leur position initiale après Restore.
Le catalogue conserve35/65, sans write dans le cas partial. Le second Apply est
refusé tant que le retour est actif. Annuler puis Enregistrer et appliquer sont testés
via la vraie WebView et le bridge. Aucune frappe système, capture ou titre exporté.

Les cas ambigus, annulation du sélecteur natif, mouvement manuel de fenêtre et
changement d’écran restent des vérifications Mac pilote ; pas d’extrapolation
à Chrome/Teams ou macOS26.7.2 ARM depuis les fixtures macOS15.7.9 Intel.
Le paquet0.6.2/build22 est Intel/ARM ; le temps total mémoire des auxiliaires WebKit
reste non mesuré. Les gestes et leurs seuils n’ont pas été modifiés.

## Phase4 — réglages système, 0.6.3

Branche ui/studio-system basée sur ui/studio-apply. Sept pages Studio : Accueil,
Espaces, Raccourcis, Trackpad, Souris, Autorisations et Diagnostics. Fallback et
ouverture AppKit par défaut conservés ; cutover et previewApply restent distincts.

Adaptateurs du Controller dans src/studio/controller.rs, services existants seuls
propriétaires. UI locale supplémentaire system.js, aucune librairie frontend.
Commandes système typées dans studio/system.rs : pas de shell, URL libre ou JSON
arbitraire exécuté. Réglages Système ouvert par son bundle installé en allowlist ;
la section validée sert au guidage, sans prétendre ouvrir un lien profond non public.

Raccourcis : CRUD, choix d’app via sélecteur natif existant, test de commande enregistrée,
activation globale, reset confirmé, relecture volontaire et réenregistrement. Revision
opaque associée aux octets exacts et aux réglages réellement chargés. Fichier externe
modifié/invalide : sauvegarde bloquée, moteur conservé ; relecture explicite avant
édition. Writer atomique existant renforcé d’un contrôle exact avant rename pour Studio.
Enregistrement Carbon candidat avant écriture ; rollback sur échec d’enregistrement
ou d’écriture. Les anciens handlers sont retirés/restaurés selon la même transaction
que l’interface classique. Global off n’est pas assimilé à une entrée active.

Capture : même Recorder AppKit dans une petite NSWindow modale volontaire, premier
répondant explicite, anciens hotkeys suspendus, Échap/perte de focus/fermeture/15s
annulent. Aucun moniteur clavier global ou keydown JS. La combinaison n’est persistée
qu’au clic Enregistrer. Capture terminée : restitution des hotkeys ; un refus Carbon
est signalé au lieu d’un succès inventé. Brouillons raccourcis et espaces sont gardés
par le même Save/Discard/Cancel, y compris fermeture et ⌘Q.

Gestes : activation/désactivation, fenêtre de test local réelle, diagnostic global
passif, compteurs réels et bilan. Mode expérimental et seuils10 % inchangés, pas de
mapping configuré fictif. UI : suivi demandé13s au maximum, arrêt à la navigation/
fermeture ; aucun polling au repos. Le pilote natif garde sa politique d’interruption
et de réactivation manuelle. Compteurs absents : Non mesuré, pas0.

Souris : préférence/filtre actif séparés, réactivation manuelle et vrai rapport du tap.
Permissions : lecture sans demande ; demandes AX/entrée seulement sur action. Chrome
reste unknown tant qu’aucun appel réel n’a fourni une preuve ; denied seulement pour
-1743, granted après réponse AppleScript ; état de dernier appel, pas une promesse TCC
actuelle. Rapports copier/export proviennent du backend authentique ; NSSavePanel .txt.
Tester une commande ne prétend pas qu’une activation asynchrone a déjà réussi.

Validation ajoutée : un raccourci Carbon réservé par une fixture dédiée, échec candidat
et rollback observés ; vraie fenêtre Recorder ouverte/focalisée, suspension observée,
annulation native puis restitution ; CRUD/UI, pages, rapport dans pbpaste et relecture
après relance. Config/shortcuts temporaires uniquement sous mode de vérification,
NSUserDefaults globaux réels non écrits par cette vérification. Aucune frappe système
synthétique, aucun contact simulé, aucun screenshot ou contenu tiers.

## Corrections0.6.4 et actions complémentaires

Completion et timeout du Recorder programmés explicitement en modes default ET
NSModalPanelRunLoopMode. La capture réussie retourne sans clic Annuler ; UI affiche
la combinaison capturée et demande Enregistrer. Aucun enregistrement implicite.
CI : second essai de capture, NSEvent fixe livré directement à notre Recorder
focalisé, sans injection système ; retour et message UI vérifiés en moins2,5s.

Commandes ajoutées : Work, Meeting, Restore, ApplyProfile{profile_id}. Moteurs et
snapshot existants partagés, pas de commande shell libre. Catalogue relu au
déclenchement ; identifiant stable, pas nom/index. Référence absente : erreur,
aucune substitution. Sélecteurs Studio et AppKit affichent les espaces enregistrés.

Gestes : supprimer cooldown1s qui annulait entièrement un contact commencé trop
tôt. Le doigt doit toujours être levé entre deux actions. Seuils, direction, durée,
multitouch et politique du tap inchangés. Ne corrige pas des contacts non livrés.

##0.6.5 — réarmement sans lever

Après reconnaissance, attente d’un nouveau passage dans une bande de bord10 %.
Le même contact/device réarme ce bord sans déclenchement immédiat, puis le trajet
intérieur10 % est mesuré depuis ce nouveau départ. Retour au bord de l’action
précédente : recul6 % depuis son point de reconnaissance pour filtrer les rebonds.
Un contact commencé au centre peut rejoindre un bord avant son premier geste.
Coordonnées/identité restent transitoires. Interruption multitouch/clic/device ou
contact invalide exige encore un lever ; aucun réarmement depuis le centre.

##0.7.0 — bascule de l’interface

Studio par défaut au lancement, Dock et Ouvrir Kigi. --studio-ui reste compatible ;
--native-ui force le secours. Menus sans doublons, secours dans Diagnostics ou erreur
de chargement. Les anciens services restent natifs ; ancien écran gardé une version.

spaces.capture{expectedRevision} : choix natif de deux fenêtres existantes, capture
AX en lecture seule, catalogue recontrôlé avant/après choix. Retour vers brouillon ;
aucun déplacement, snapshot ou fichier créé avant Enregistrer. Profils libres gardés.
spaces.recoverBackup{expectedRevision,confirmed} : token dédié lecture seule et octets
primaire/backup liés. Récupération refuse toute modification depuis l’affichage ou
tout primaire redevenu valide. Ancien invalide copié et synchronisé dans un fichier
config.invalid-<temps>.json, backup original conservé, primaire remplacé atomiquement.
Contrôles exacts juste avant écriture ; même fenêtre de concurrence résiduelle entre
comparaison finale et rename que le writer existant, pas de verrou universel.
