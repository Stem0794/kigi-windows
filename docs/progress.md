# Suivi

## 2026-11-12 — P0 implémenté, validation Mac en attente

Dépôt GitHub inspecté : vide, aucun historique. Handover version 2 importé sans
modification : AGENTS, HANDOVER_CODEX, CODEX_START, JSON indicatif.

Livré : un crate Rust, barre de menus AppKit, Accessibilité sur demande,
inventaire ponctuel sans titres, déplacement/redimensionnement à droite et
restauration du snapshot, raccourci Carbon ⌥T au relâchement, diagnostic local
et essai global passif du geste, script de bundle local avec signature ad hoc.
Pas de parsing du JSON futur, aucun P1–P7.

Vérifications sous Linux avec Rust 1.99.1 :

- `cargo fmt --check` : réussi après formatage.
- `cargo check --locked` : réussi (branche hôte sans backend macOS).
- `cargo test --locked` : 3 tests réussis.
- `MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target x86_64-apple-darwin`
  et la même commande pour `aarch64-apple-darwin` : réussies, sans lien natif.
- Syntaxe du script et plist : vérifiées ; création/signature/exécution du
  `.app` impossibles dans cet environnement Linux.

Dépendances résolues et verrouillées : objc2 0.6.5, bindings Apple 0.3.2 ;
features limitées aux API utilisées. Aucun moteur web, runtime async, service,
réseau ou framework de tests. Aucune licence commerciale encore choisie.

Reste à valider, sans annoncer P0 terminé : sur un vrai Mac, construire le
bundle, lancer deux fois au même chemin, tester le menu, permission refusée/
autorisée/révoquée, inventaire Chrome/Teams, un déplacement/restauration avec
⌥T, une fenêtre non redimensionnable, et le spike trackpad. Refaire le test
sur Intel/macOS 11 avant toute promesse de compatibilité. Rust supporte les
cibles Apple Intel/ARM, mais cela ne valide pas les API de cette application.

Prochain sous-ticket : **validation native P0 sur le Mac pilote**, puis relever
les observations dans `trackpad-spike.md`. Ne pas démarrer P1.

## Permissions et limites

Accessibilité pour lire/placer les fenêtres tierces. Aucune Surveillance de
l'entrée pour le raccourci Carbon ; elle est demandée séparément seulement
au démarrage volontaire du tap. Le diagnostic local n'en a pas besoin.
Pas d'Apple Events, d'enregistrement d'écran, de root ou de modification MDM.
Pas d'audit `init.lua` possible ici : le Mac de l'utilisateur n'est pas accessible.

La cible macOS 11 est déclarée dans le bundle et le script pour les deux
architectures ; elle reste une cible, pas une compatibilité terrain validée.
L'inventaire est volontairement borné ; AX et le contrôle de géométrie restent
synchrones, avec délai par appel. Les fenêtres peuvent répondre tardivement ou
imposer leurs dimensions. Le snapshot n'est pas persistant ; restaurer avant
quitter. La signature ad hoc n'est ni notarisation ni garantie de stabilité TCC
à travers les recompilations.

## 2026-11-12 — Package téléchargeable P0

À la demande de l'utilisateur, ajout d'un unique workflow de packaging macOS,
sans matrice de tests ni CI fonctionnelle. Le premier runner macos-14 était
retiré ; passage à macos-15. Le run [43232714798](https://github.com/Stem0794/kigi-windows/actions/runs/43232714798)
a construit et lié les deux binaires Release, assemblé l'universel avec lipo,
validé le plist et la signature ad hoc avec codesign, puis créé le ZIP.

Commit du package : 3a346c6bfe5d5248d6e51b6ed71f46f63e266df9.
ZIP applicatif : 365 251 octets ; exécutable universel : 801 072 octets.
Architectures Mach-O vérifiées : x86_64 et arm64. Archive ZIP vérifiée sans
erreur CRC. Aucun Rust nécessaire sur le Mac destinataire.

Cela remplace la limitation de build/packaging évoquée plus haut : ceux-ci
sont désormais vérifiés sur un runner macOS. L'application n'a toutefois pas
été exécutée ni testée avec les permissions, fenêtres ou trackpads réels.
Pas de signature Developer ID ni de notarisation. Les validations natives
interactives et macOS 11 restent en attente ; aucun démarrage de P1.

## 2026-11-12 — P0 0.0.2 après premier essai utilisateur

Le diagnostic local fonctionne selon le retour utilisateur ; le diagnostic
global a reçu des contacts et trois candidats (voir trackpad-spike.md).
Réduction de la zone de départ à x≤0,10, y≥0,80, commune aux deux diagnostics.
Le test existant vérifie aussi le rejet à x=0,1001 et l'acceptation à x=0,10.

L'utilisateur rapporte que ⌥T ouvre Teams depuis Chrome. Kigi P0 ne contient
aucune action d'ouverture de Teams ; un autre gestionnaire, probablement
Hammerspoon, exécute cette commande. Le raccourci de test Kigi devient
⌃⌥T (Control + Option + T) pour éviter cette combinaison, sans modifier
Hammerspoon ni implémenter l'action Teams de P1. Le conflit n'est pas prouvé
par une inspection du Mac. Relancer Kigi après arrêt d'un concurrent si besoin :
l'enregistrement se fait au lancement. Un échec est affiché dans le menu.

Version du bundle incrémentée à 0.0.2. Vérifications exécutées avec succès sur le runner macOS : fmt, cargo check,
trois tests (3/3) et construction native Release Intel/ARM. ZIP universel signé
ad hoc, plist et lipo vérifiés : [run 43243429859](https://github.com/Stem0794/kigi-windows/actions/runs/43243429859). La nouvelle zone et le nouveau
raccourci restent à essayer sur le Mac utilisateur. P1 non commencé.

## 2026-11-12 — P0 0.0.3 : raccourci sans effet

L'utilisateur confirme « trackpad ok » après 0.0.2, mais Control+Option+T
semble sans effet. Pas de preuve supplémentaire sur les autres Mac/OS.
Le backend précédent écoutait uniquement le relâchement sur la cible Application.
Le correctif écoute appui/relâchement sur la cible Event Dispatcher, déclenche
une fois à l'appui, puis réarme au relâchement. L'action est postée via
performSelectorOnMainThread au lieu d'un sélecteur temporisé en mode par défaut.
Ces modifications ciblent le routage suspecté ; la cause exacte sur le Mac de
l'utilisateur n'est pas établie. Aucun nouveau changement de combinaison.

Ajout d'une ligne permanente avec état d'enregistrement et compteur de
raccourcis reçus, cliquable pour afficher version, permission AX et dernier
résultat. Aucune frappe arbitraire n'est surveillée ou stockée. Un seul test
de régression ajouté pour éviter le double déplacement à l'appui/relâchement
ou pendant une touche maintenue. Vérifications réussies : fmt, check, quatre tests sur macOS (4/4), construction
Release et lien natif Intel/ARM, signature ad hoc et ZIP universel contrôlés.
[Run de validation 43245414635](https://github.com/Stem0794/kigi-windows/actions/runs/43245414635),
source du package : 94f38b6e161212d91fea14859695a9115f350c63.
Fonctionnement interactif du correctif à retester sur le Mac utilisateur.
Trackpad et ses seuils inchangés. P0 uniquement.

## 2026-11-12 — P0 0.0.4 : logs et référence utilisateur

Le diagnostic transmis par l'utilisateur indique 11 raccourcis reçus et
Accessibilité non autorisée : cause observée du refus de déplacement, sans
preuve d'une panne persistante du raccourci. Ajout d'une explication visible
une fois par session en cas de test AX refusé, sans changement des permissions.

Journal structural en mémoire limité à 200 événements / 400 caractères de
contexte par événement. Copie dans le presse-papiers et export `.txt` via
NSSavePanel uniquement sur action utilisateur. Version/OS/architecture, chemin
abrégé, permissions, réception/routage, erreurs AX et géométrie ; pas de
frappes, titres, contenus, URLs, contacts ou coordonnées de doigts. Le callback
CG du trackpad ne journalise pas son flux ; seul son bilan est exporté.
Un test ajouté pour borne mémoire, rotation et détails Unicode sur une ligne.

Fichier Hammerspoon fourni copié sans modification comme référence (455 lignes)
et audité dans migration-audit.md : gestes de bord sur toute la hauteur,
sélecteur de deux fenêtres, chat Teams gauche/réunion droite, retour au chat,
réutilisation d'onglet ChatGPT. Le profil complet n'est pas activé dans ce P0.
Le prochain jalon reste le déplacement/restauration sur le Mac après permission
AX, puis les raccourcis manuels de migration. Aucun init.lua local modifié.

Validation 0.0.4 : fmt/check réussis, 4 tests sous Linux et 5 tests sur le runner
macOS Intel, Release Intel/ARM liées, signature ad hoc/plist/lipo et intégrité du
ZIP vérifiés. [Run 43249225076](https://github.com/Stem0794/kigi-windows/actions/runs/43249225076),
source du package : ef7f831916f9fe3fb881f613157c8fcb5643b75b.
Les commandes presse-papiers/enregistrement et la nouvelle explication AX sont
compilées, mais leur interaction sur le Mac utilisateur reste à vérifier.
