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

## 2026-11-12 — P0 0.0.5 : demande et état Accessibilité

L'utilisateur indique que la case autorisée n'est pas reconnue et que la
permission est redemandée. Deux problèmes concrets dans le parcours précédent :
le bouton appelait à chaque clic l'API avec prompt, et interprétait son retour
immédiat alors que le prompt est asynchrone. Demande unique par session,
revérifications sans prompt, actualisation à l'ouverture du menu et sur l'action,
et suivi temporaire borné à 15 contrôles / 30 secondes après demande.

Audit ABI : les fonctions AX/CF déclarées avec Apple `Boolean` passent maintenant
par `u8` (UInt8), puis comparaison explicite à zéro. Les autres API CG en C `bool`
restent inchangées. Cette correction de signature n'est pas déclarée cause
confirmée du problème terrain.

Ajout d'un menu Finder pour repérer la copie en cours, d'un contrôle des autres
instances au lancement, et d'un diagnostic AX explicite en lecture seule :
valeurs brutes des deux précontrôles, bundle cible et code `AXWindows` / compte.
Les logs ajoutent PID, autres instances et indicateur AppTranslocation.
Pas de lecture de TCC, pas de modification d'autorisations, pas d'API privée,
pas d'affaiblissement des exigences de signature. La signature ad hoc reste une
limite réelle entre builds ; une case cochée ne suffit pas à conclure à la
confiance du processus courant. Attendre les nouveaux logs si le refus persiste.

Un test de régression vérifie demande unique, arrêt au grant et budget épuisé.
Le parcours TCC interactif, Finder et garde multi-instance restent à essayer
sur le Mac ; aucune promesse de permission désormais accordée. P0 uniquement.

Validation 0.0.5 : fmt/check et 4 tests hôte réussis ; 6 tests sur runner macOS
Intel (dont la régression demande unique), Release Intel/ARM liées, signature
ad hoc/plist/lipo et intégrité ZIP vérifiés. [Run 43254778878](https://github.com/Stem0794/kigi-windows/actions/runs/43254778878),
source du package : a11b3eeb8c2455ca33bedbefef9e28e8185567c2.
Les manipulations TCC, le rafraîchissement après grant, Finder et le blocage des
copies multiples n'ont pas été validés interactivement sur le Mac utilisateur.

## 2026-11-12 — P0 0.0.6 : restauration à gauche avant retour 100 %

L'utilisateur confirme que les fenêtres se déplacent et que le geste haut-gauche
fonctionne. Il observe exactement : 100 %, puis moitié droite, puis moitié gauche,
puis 100 % après trois appuis. Le code précédent demandait la taille avant la
position ; agrandir depuis le bord droit peut être limité à la largeur restante.
Le snapshot était conservé après ce résultat partiel, permettant au troisième
appui d'achever la restauration depuis la gauche. C'est l'explication cohérente
avec le code et le retour, à vérifier avec les frames réelles si cela persiste.

Correctif ciblé : position cible, taille cible, position cible réappliquée.
Ce dernier placement couvre aussi le cas d'une fenêtre encore large déplacée
vers la moitié droite avant réduction. Trois appels bornés, aucune boucle de
retry, aucune nouvelle abstraction/dépendance. Ordre inscrit dans les logs,
géométrie finale toujours relue et snapshot conservé si résultat partiel.
Pas de nouveau test simulant macOS : les six tests existants et la compilation
seront exécutés ; essai fonctionnel requis sur le Mac : 100 % / droite / 100 %
avec deux appuis. Le geste et ses seuils ne sont pas modifiés. P0 uniquement.

Validation 0.0.6 : fmt/check, 4 tests hôte et 6 tests sur runner macOS Intel
réussis ; Release Intel/ARM, signature ad hoc/plist/lipo et intégrité ZIP
vérifiés. [Run 43259628603](https://github.com/Stem0794/kigi-windows/actions/runs/43259628603),
source du package : 7533eeb39f227f513d92aa7caabca8e2222629cd.
L'efficacité du nouvel ordre contre la restauration partielle reste à confirmer
sur le Mac utilisateur ; aucun test unitaire ne simule les contraintes AX.

## 2026-11-12 — P1 0.1.0 : raccourcis manuels autorisés

Retour utilisateur : « oui c'est corrigé, continue l'implémentation ». La
restauration directe 0.0.6 est validée sur son Mac ; il a aussi confirmé le
diagnostic trackpad. Cela ouvre P1, sans valider d'autres Mac ou macOS.

Implémenté : Teams ⌥T, Chrome/maximiser ⌥B, URL ChatGPT ⌥C, maximiser
⌘⇧↑, sélection de deux fenêtres ⌘⇧←/→ conforme à la référence (ordre
et focus du second choix). Pas de ⌘⇧↓ inventé. Carbon enregistre chaque
touche indépendamment et journalise son conflit ; réenregistrement et suspension
personnelle depuis le menu. ⌃⌥T/restauration et diagnostic trackpad conservés.

Un seul crate/processus ; ajout de block2 pour les callbacks publics NSWorkspace.
Les callbacks transmettent seulement leur résultat au thread principal via un
canal standard ; contrôle borné de lancement/fenêtre pendant 3 s, aucune boucle
au repos. AX réutilise l'ordre position/taille/position validé. Titres uniquement
dans les deux sélecteurs, exclus des diagnostics. Annulation avant le second
choix : aucune écriture. Les erreurs/placements partiels restent visibles.

Limites : pas de sortie forcée de fullscreen, pas de réutilisation de l'onglet
ChatGPT ni recherche dans le sélecteur. P2 gestes actifs et automatisation Teams
non commencés. p95/RSS et interactions nouvelles à mesurer/tester sur Mac ;
aucun résultat inventé pour les fenêtres, TCC, Intel physique ou macOS 11.

Vérifications locales : fmt, check hôte, cinq tests hôte (dont mapping unique
et géométrie gauche/droite), checks macOS Intel/ARM cible 11 réussis. Le test
Carbon est étendu à l'indépendance de deux touches maintenues. Packaging et
tests natifs macOS : résultat du run à consigner après compilation.
Prochain sous-ticket : validation manuelle P1 sur Mac puis activation opt-in
du geste dans P2, sans migrer les réunions automatiques par anticipation.

Validation native P1 : [run 43262879975](https://github.com/Stem0794/kigi-windows/actions/runs/43262879975)
réussi ; fmt/check et **7 tests macOS Intel** réussis, Release Intel/ARM liées,
assemblage universel, plist et signature ad hoc vérifiés par le runner.
Source du package : 9f09073622ab9c229e406d41e3406d4db0120ad9. ZIP téléchargé,
CRC de l'artefact et du ZIP applicatif contrôlés, version/bundle ID et en-tête
Mach-O Intel/ARM relus. Aucun test interactif P1 ni mesure de performances
réalisé : ils restent le prochain essai sur le Mac pilote.

## 2026-11-12 — Correctif ChatGPT et diagnostic P1

Les logs 0.1.0 confirment réception des 7 raccourcis, mais AX=false/-25211 :
les flèches sont bloquées par permission, pas par routage. Ensuite l'utilisateur
confirme « tout est ok sauf option C » et demande correction + étape suivante.

⌥C utilise maintenant un script fixe NSAppleScript dans le processus principal,
à la demande uniquement, avec NSAppleEventsUsageDescription. Cherche les vrais
hôtes chatgpt.com/chat.openai.com dans les fenêtres Chrome ; active l'onglet
existant sans navigation/rechargement, en crée un uniquement après recherche
complète sans correspondance. Aucun fallback URL après erreur/permission
refusée ; erreurs numériques seules dans les logs, aucun titre/URL d'onglet.
Le timeout AppleScript de 3 s porte sur les requêtes Apple Events, pas sur une
borne totale de dialogue de consentement. Aucun osascript/shell/runtime ajouté.

Le refus AX est expliqué une fois par session pour toutes les actions de
placement, avec chemin de copie active ; aucun contournement ou changement TCC.
Typage macOS Intel/ARM réussi ; interaction Chrome/TCC à valider sur le Mac.
