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

## 2026-11-12 — P2 pilote 0.2.0 : gestes reliés aux actions

Après confirmation « tout est ok » pour P1, demande explicite de corriger ⌥C
et continuer. P2 pilote, pas de scènes P3 ou réunions automatiques P5.
Un tap public passif réutilisé, modes diagnostic et actions exclusifs. Menu
opt-in non persistant ; gauche 10 % / droite 20 %, toute hauteur de la référence,
vers le bas → Teams / Chrome 100 %. Le coin diagnostique reste séparé.

Classifieur pur : début réel, même doigt/device, un doigt, seuil 0,22/dérive0,12/
1,5s ; une action/contact et cooldown global1s. Second doigt, clic/drag observé,
annulation ou changement de contact bloquent jusqu'au retrait complet. Deux
tests ciblés ajoutés (cooldown/once et interruptions/invalides). Actions postées
sur le runloop après retour du callback CG, jamais AX/Apple Events dans le tap.
Les commandes manuelles effacent le geste en attente ; sélecteurs, menu et
AppleScript ignorent les gestes. Veille Mac/écran, session inactive ou tap
interrompu : arrêt, purge des actions et reprise manuelle. Pas de polling au
repos, API privée, observer périphérique ou daemon ajouté. Révocation vérifiée
avant une action reçue. Verrouillage sans veille et faux positifs restent à tester.

NSAppleScript en main pour respecter l'API ; son retour nullable est traité
explicitement malgré le binding généré non-null de 0.3.2. Consentement peut
attendre l'utilisateur. Compilation du script sur runner si Chrome installé,
sans exécution automatique. ⌥C ne retombe pas vers une ouverture URL après erreur.

Validation : fmt/check, sept tests hôte, types macOS Intel/ARM cible11 à exécuter
sur le contenu final. Packaging natif et essais réels à consigner séparément.
Prochain sous-ticket : valider réutilisation ChatGPT et gestes en usage courant
sur le Mac pilote avant de considérer P2 fiable. Éditeur générique/config JSON,
calibration graphique et scènes restent pour les étapes suivantes.

## 2026-11-12 — P2 0.2.1 : gestes horizontaux depuis les bords à 5 %

À la demande de l'utilisateur : départ sur les 5 % extrêmes gauche/droite
sur toute la hauteur, arrivée au centre x=0,50 (gauche → Teams, droite →
Chrome 100 %). Écart vertical limité à 0,12, recul extérieur toléré de 0,03,
durée ≤1,5 s. Conservation du cooldown global 1 s, de l'annulation
multi-doigts, des permissions et de la réactivation volontaire après interruption.
Le diagnostic vertical haut-gauche reste séparé, sans déclencher d'action.

Tests unitaires ciblés : seuils 5 %, centre, direction, cooldown et annulations.
Pas de nouvelle dépendance ni d'infrastructure. Validation sur Mac nécessaire
pour confirmer la détection réelle et mesurer d'éventuels faux positifs.

## 2026-11-12 — Revue P2 et P3 pilote 0.3.0

La revue a détecté deux échecs successifs du build P2 : formatage des tests
puis version `Cargo.toml`/`Cargo.lock` divergente. Formatage corrigé et
lockfile synchronisé dans 0.2.1 ; le runner a passé fmt/check/tests et la
compilation du script Chrome. Le fonctionnement réel du nouveau profil 5 %
demeure non validé sur Mac.

P3 débuté de façon limitée : trois actions manuelles en barre de menu,
Travail (Chrome 100 %), Réunion (Teams gauche et Chrome droite 50/50) et
Restaurer. Deux fenêtres déjà ouvertes et standard sont requises. Préflight
AX des deux, snapshot mémoire capturé avant écriture et lecture finale des
géométries. Un seul snapshot de retour, pas de pile de scènes. Restaurer
n'écrase pas une fenêtre déplacée manuellement depuis l'application. Une
relance de Réunion n'écrase pas le snapshot existant ; Travail le termine.
Les applications ne sont jamais fermées et aucun onglet n'est créé par P3.

Limites de ce premier pilote : sélection de fenêtre Teams basée sur la fenêtre
standard/focalisée, pas de prédicat prouvé « réunion », pas de lancement d'app
par la scène elle-même ni JSON persistant. La configuration versionnée,
Meeting+Chat et l'éditeur attendent les sous-étapes suivantes. Ne pas activer
de détection de réunion automatique avant audit terrain. Aucun nouveau
crate, dépendance ou service.

Validation native P3 : [run 43274091660](https://github.com/Stem0794/kigi-windows/actions/runs/43274091660)
réussi sur macOS : formatage, vérification Rust, tests unitaires,
compilation du script Chrome, construction universelle Intel/ARM, assemblage
et archive. L'archive applicative extraite a passé le contrôle CRC ; Info.plist
indique 0.3.0 / build 10. Les trois commandes P3 restent à essayer sur le Mac
pilote, tout comme le geste horizontal P2 et la permission Automatisation Chrome.
Aucune preuve de compatibilité matérielle ancienne ni de performance p95/RSS.

## 2026-11-12 — Reprise depuis GitHub, P3 0.3.1

Main récupéré à 8a925d6, changements 0.3.0 conservés. Demande explicite de
continuer depuis GitHub. Gestes horizontaux depuis les 5 % extrêmes vers le
centre inchangés ; pas de réunion automatique/P4 anticipés.

Ajout d'une disposition personnelle persistante de deux fenêtres sur un écran :
sélection explicite, capture normalisée, applications existantes, résolution
par bundle et choix utilisateur si ambigu, application sur écran courant puis
restauration du snapshot. Deux fenêtres de la même app sont possibles sans
persister de titre ou de pointeur. Toute résolution/preflight avant écriture.
Un seul retour temporaire partagé avec Réunion ; pas de pile ou moteur générique.
JSON schema1 strict, 64 Ko max, géométrie/bundles validés, remplacement atomique,
backup valide et relecture de secours sans écrasement d'un fichier invalide.
Pas de daemon, service ou base de données ; serde/serde_json pour ce besoin réel.

Correction trouvée pendant la reprise : 0.3.0 créait le snapshot après écritures,
et utilisait la position initiale comme fallback après échec de lecture. Une
restauration partielle pouvait aussi être prise pour un déplacement manuel au
second essai. Snapshot maintenant assigné avant écriture, read-back optionnel,
suivi des positions intermédiaires pendant restore ; fenêtres traitées
indépendamment, fermetures confirmées distinguées des erreurs transitoires.

Tests ciblés : normalisation/adaptation à l'écran et décisions de restauration ;
roundtrip JSON, backup valide, fichier corrompu et refus de remplacement invalide.
Le runtime local repris ne contenait pas Rust ; toolchain réinstallée pour
reproduire les vérifications, sans reporter d'anciens tests comme nouveaux.
Compilations et résultats natifs à consigner après vérification finale.
Limites : une seule disposition, deux fenêtres déjà ouvertes, un écran à la fois,
retour mémoire seulement. Prochain sous-ticket : essai terrain capture/restart/
application/restauration avant éditeur et scènes multiples.

Vérifications locales 0.3.1 : Rust1.100 installé, fmt/check réussis, 10 tests
hôte réussis ; checks macOS Intel/ARM cible11 réussis sans lien Apple local.
Les deux nouveaux tests couvrent la persistance et la décision de restauration,
sans mock AX. Le choix reste explicite même pour une seule correspondance si
l'inventaire est partiel, pour ne pas prendre une liste tronquée pour un match
unique. Les interactions macOS et le cycle après redémarrage restent à tester.

Validation native 0.3.1 : [run 43283892092](https://github.com/Stem0794/kigi-windows/actions/runs/43283892092)
réussi, fmt/check et **12 tests macOS** réussis, script Chrome compilé,
Release Intel/ARM liées, signature ad hoc/plist et lipo vérifiés. Source du
package : a10d60ae26bb2a7d4d455ed85541f396e84921d1. Artefact téléchargé et
SHA256 vérifié, ZIP applicatif CRC vérifié, Info.plist 0.3.1/build11 et Mach-O
Intel/ARM relus. ZIP 576485 octets. Capture/Finder/redémarrage/placement et
restauration n'ont pas été exécutés interactivement ici : essai Mac nécessaire.

## 2026-11-12 — Option souris 0.3.2

Demande explicite de désactiver en option le scroll horizontal des souris Apple.
Implémentation native opt-in, préférence NSUserDefaults séparée du JSON de
scènes. Tap session modifiable, masque ScrollWheel seulement, indépendant du
tap Gesture passif. Aucun processus, crate, API privée ou boucle au repos ajouté.
Les champs publics horizontaux ligne/fixe/pixel sont annulés, le vertical,
profondeur, phases et modificateurs sont préservés. Les événements purement
horizontaux sans frontière de séquence sont supprimés sans réémission.

MayBegin public permet de préserver le trackpad, Magic Mouse n'émettant pas ce
préambule selon Apple. L'inertie suit uniquement une séquence connue récente ;
inconnus, phases manquantes et séquences périmées passent. Marqueur MayBegin
conservé si les doigts restent posés longtemps avant de scroller. Ce n'est pas
une identification de marque : d'autres souris compatibles sont aussi filtrées,
et les usages simultanés/événements réécrits restent ambigus. Option pilote.

Activation explicite et reprise au lancement/retour de session seulement si
préférence active et permission AX déjà accordée, sinon état inactif visible.
Veille retire le filtre ; tap interrompu laisse passer et demande une relance
manuelle par l'option. Arrêt Kigi/désactivation libèrent port/source/context.
Aucune action AX/I/O ou log de déplacement dans le callback. Compteurs
structurels exportables, ni coordonnées ni identifiants de matériel.

Tests ciblés : trackpad/Mouse/inertie, séquences incomplètes/périmées et marqueur
trackpad après repos. Test natif CG : les trois axes X à zéro avec Y/Z et phases
inchangés, sans installer un tap ni poster d'événements. Fonctionnement matériel
Magic Mouse/Minecraft, préférences natives et reprise après veille à confirmer
sur le Mac pilote. Les scènes et gestes 5 % sont conservés.

Validation 0.3.2 : fmt et 12 tests hôte réussis ; checks de code/tests Intel/ARM
cible11 réussis. [Run macOS 43286890229](https://github.com/Stem0794/kigi-windows/actions/runs/43286890229)
réussi : 15 tests, dont test CG de conservation Y/Z/phase, script Chrome compilé,
Release Intel/ARM liées, signature ad hoc/plist/lipo vérifiés. Source :
5c96f015fc3befb4e46b80af76abe5dc18109c12. Archive téléchargée, SHA256 de
l'artefact et CRC ZIP applicatif vérifiés, version0.3.2/build12 et Mach-O relus.
Aucun essai réel de tap modifiable, Magic Mouse, trackpad, Minecraft,
NSUserDefaults ou veille réalisé ici ; ces interactions restent à confirmer
sur le Mac pilote avant de déclarer la classification fiable.

## 2026-11-12 — P4 pilote 0.4.0 : espaces nommés et interface native

L'utilisateur confirme le fonctionnement de l'option souris 0.3.2 puis demande
la prochaine étape. Retour positif sur son Mac, sans l'étendre à tout matériel.
Main dac219e inspecté et conservé. Première UI P4 AppKit, créée à la demande :
liste, nom, capture/recapture de deux fenêtres, renommage, duplication,
suppression confirmée, positions gauche/droite/100 %, appliquer et messages
visibles. Sous-menu des espaces pour activation directe. Pas de nouveau crate,
serveur, WebView, processus ou framework de réglages.

Catalogue schema2, IDs stables non réutilisés, 32 espaces max/noms60caractères.
Schema1 repris en mémoire comme Disposition personnelle, sans écriture à la
lecture. Premier changement sauvegarde l'ancien fichier et écrit atomiquement
le nouveau. Validation stricte, unicité de noms/IDs, backup/read-only recovery
conservés. Les mutations relisent la configuration avant sauvegarde ; un
formulaire inchangé ne remplace pas le backup. Logs : compteurs/IDs et succès,
aucun nom utilisateur ou titre de fenêtre ajouté au diagnostic.

Les sélections partent de l'application précédant l'ouverture des réglages,
avec repli écran principal, pour éviter de cibler la propre fenêtre Kigi.
Résolution et préflight précèdent tout placement ; annulation ne sauvegarde
rien. Ancien snapshot partagé avec Réunion, positions/restauration conservées.
Le filtre souris, la recherche d'onglet et les gestes5 % sont inchangés.

Tests ciblés : migration v1→v2, roundtrip/backup, fichier invalide conservé,
identités stables pendant renommage/suppression et absence de réutilisation.
13 tests hôte et typage Mac ARM/Intel déjà réussis pendant l'implémentation ;
validation finale et packaging à consigner après compilation. UI interactive
non exécutée ici (Linux), à essayer sur le Mac pilote. Reste P4 : interactions
graphiques avancées, capture de raccourcis et affectation gestes→espaces. P5
n'est pas démarré.

Validation finale 0.4.0 : fmt/check, 13 tests hôte et checks de code/tests Mac
Intel/ARM cible11 réussis. [Run macOS 43299940463](https://github.com/Stem0794/kigi-windows/actions/runs/43299940463)
réussi : 16 tests, script Chrome compilé, Release Intel/ARM liées, signature
ad hoc/plist/lipo contrôlés. Source : 468889f5f1b6a5a79490bb58e500d1e12cd144a1.
Archive téléchargée, SHA256 de l'artefact et CRC applicatif vérifiés ; version
0.4.0/build13, binaire universel et chaînes de la nouvelle UI relus. ZIP 665517
octets. Aucun clic ni rendu
interactif de la fenêtre AppKit exécuté ici ; création/duplication/édition/
application depuis la nouvelle UI restent à essayer sur le Mac pilote.

## 0.4.1 — Interface principale

Fenêtre AppKit au lancement, présence Dock et réouverture de la même fenêtre.
Cinq onglets : Espaces, Raccourcis, Gestes et souris, Autorisations, Diagnostics.
Le menu de statut garde ouverture/espaces/dispositions/restauration/quitter.
Options synchronisées sur l’état réel ; activation des raccourcis personnels
mémorisée. Gestes toujours volontaires et arrêtés après veille. Fermeture de
fenêtre sans arrêt des fonctions, ⌘Q pour quitter. Menus Édition/Fenêtre natifs
pour copier/coller, sélectionner, fermer/réduire. Le contexte de capture garde
le dernier PID externe en mémoire, sans titres ni détection de réunions.
Pas de nouvelle dépendance, seulement les classes AppKit NSTabView(Item).
Validation interactive et présentation à confirmer sur Mac pilote ; aucun
rendu/clic AppKit réalisé sur l’hôte Linux.

Validation finale 0.4.1 : fmt/check, 13 tests hôte, typage code/tests Intel et
ARM réussis. [Run macOS 43304136419](https://github.com/Stem0794/kigi-windows/actions/runs/43304136419)
réussi : 16 tests, script Chrome compilé, Release Intel/ARM et vérifications
signature/plist/lipo. Source d53009c773260490b4a02350c1cc8af1b6d2ee9a.
SHA256 de l’artefact et CRC des deux archives vérifiés ; paquet universel
0.4.1/build14, LSUIElement=false, 681251 octets. L’affichage, la réouverture
Dock et les interactions de la fenêtre restent à confirmer sur le Mac pilote.

## 0.5.0 — Raccourcis personnalisables et retour des gestes

Demande utilisateur : absence de détection 5 %, changer les touches et l’app
cible, améliorer UX/UI. Éditeur natif de 32 raccourcis : ajout/suppression,
activation individuelle/globale, une combinaison saisie volontairement dans un
NSView focalisé, choix `.app` via NSOpenPanel, maximisation facultative, actions
fenêtre ou ChatGPT. Détection des doublons ; essai Carbon avant écriture et
retour aux anciens réglages si refus. Écriture JSON atomique séparée du
catalogue, validation stricte et garde contre modification externe. Les noms
d’apps et caractères saisis ne sont pas ajoutés au rapport des raccourcis.

Annulation par Échap, changement de focus/app, fermeture de fenêtre ou délai
15 s ; réenregistrement des touches après saisie. Aucun moniteur clavier global.
En-tête, hiérarchie typographique et formulaire action/application/combinaison,
état réel visible et onglet Raccourcis ouvert au lancement. Aucune nouvelle
dépendance ; fonctionnalités AppKit existantes complétées.

Trackpad : le diagnostic local utilisait encore le coin vertical ; il partage
désormais le reconnaisseur horizontal 5 % avec le global. Compteurs visibles,
log de première réception, bilan automatique et arrêt après 12 s sans contact
(y compris détection des contacts multi-doigts). Aucun contact reçu n’est plus
confondu avec un geste valide. Réception globale encore NON RÉSOLUE sur le Mac
utilisateur : il faut son nouveau rapport pour localiser la panne. API privées
non ajoutées. Souris, gestes de base Teams/Chrome et espaces conservés.

Trois tests purs ciblés : validation/affectation/doublons, persistance et refus
d’écrasement invalide/externe, limite du diagnostic sans contacts. Vérifications
locales réussies : 16 tests hôte, code/tests macOS ARM/Intel. Validation native
et archive à consigner après build. UI/saisie/trackpad réels à tester sur Mac.

Validation finale 0.5.0 : [run macOS 43309029522](https://github.com/Stem0794/kigi-windows/actions/runs/43309029522)
réussi sur d155b1e276cadfdbeb7903a68b5d8692d447a300 : fmt/check, 19 tests
macOS, script Chrome compilé, Release Intel/ARM, signature/plist/lipo. Un seul
contrôle de démarrage natif : processus resté vivant 4 s, puis terminé par le
runner ; aucune saisie, action AX ou capture écran. Ce contrôle ne valide pas
la saisie interactive ni le trackpad physique. Archive téléchargée : SHA256
artefact/CRC outer-inner vérifiés, Info.plist 0.5.0/build15, Mach-O Intel/ARM,
762111 octets ; SHA256 ZIP bfdd2a89b4814bbddbc966213d140688f37f8f48245d92a51db9362fea0f5ad4.
Retour Mac requis : réaffectation ⌥T à une autre app, changement de combinaison,
annulation/focus/conflit/persistance ; test local 5 % puis global 12 s et logs.

## 0.5.1 — Essai des bords à 10 %

À la demande de l’utilisateur, départ x≤0,10 à gauche ou x≥0,90 à droite,
au lieu de 5 %. Reconnaisseur partagé par actions et diagnostics local/global ;
libellés et logs alignés. Arrivée x=0,50, durée, dérive, annulation et cooldown
conservés. Test des bornes ajusté (incluses 0,10/0,90, exclus 0,10001/0,89999).
Cela élargit la zone de départ sans résoudre une éventuelle absence de contacts.
Essai matériel requis ; aucune API privée ajoutée.

Validation 0.5.1 : 16 tests hôte, checks code/tests Mac Intel/ARM ;
[run macOS 43311517374](https://github.com/Stem0794/kigi-windows/actions/runs/43311517374)
réussi sur 0579d6b564855ae9ae2497a9b215d24372549312 : 19 tests, construction
universelle, signature/plist/lipo et démarrage natif 4 s sans erreur signalée.
SHA256 artefact, CRC ZIP, version 0.5.1/build16 et Mach-O universel vérifiés.
La détection physique au nouveau seuil reste à essayer sur le Mac utilisateur.

## 0.5.2 — Retour aux critères P0/P1

Après revue, demande explicite de traiter P0 et P1. Bilan global de 12 s :
événements/contacts/échantillons à un doigt, Began/Moved, débuts dans les bords,
gestes et échecs de conversion. Diagnostic de l’étape manquante, arrêt si aucun
geste reconnu, aucune coordonnée/identité exportée. Le seuil 10 % est conservé.
Un geste reconnu n’est pas présenté comme preuve de fiabilité ni de GO général.

Comparaison locale entre relances de cette copie (version, AX, surveillance),
écriture uniquement des derniers états, aucun changement de TCC. Test natif
ciblé de deux lancements successifs : fenêtre visible, RSS propre au processus
et flags de permissions comparés. Sans gestes/AX sur app tierce en CI.

P1 : requête de maximisation conserve la fenêtre ciblée ; déminimisation et
sortie du plein écran demandée via AXFullScreenButton/AXPress public, état
système public currentSystemPresentationOptions. Pas d’écriture AXFullScreen
ni de frappe synthétique. Attente asynchrone bornée à 4 s, rectangle stable
200 ms après transition, annulation au changement d’app/commande/veille. Les
sélecteurs/scènes conservent leurs limites pour fenêtres normales.

RSS actuelle via proc_pidinfo du propre PID ; pic RSS via getrusage Darwin
(octets). p95 borné aux 64 derniers succès par catégorie, erreurs séparées,
aucun succès = non mesuré. Géométrie AX, activation, maximisation mesurées
séparément sans inclure le temps de choix utilisateur dans la géométrie. libc
était déjà dans le graphe ; dépendance directe pour ses bindings système.
Pas de benchmark CPU ni résultat p95 inventé. Trois tests ciblés : p95/bornes/
échecs, attente de stabilisation et diagnostic des étapes du trackpad.

P0 trackpad et P1 plein écran nécessitent encore le Mac pilote ; rien n’est
déclaré clos à partir de la seule compilation. Validation finale à consigner.

Validation finale 0.5.2 : [run macOS 43327603804](https://github.com/Stem0794/kigi-windows/actions/runs/43327603804)
réussi sur ddde02d4fcbc81b4c77295b2cf12e91764cf538f : 21 tests macOS,
Release Intel/ARM liées, signature/plist/lipo et archive validés. Deux
lancements du même bundle sous macOS 15.7.9 x86_64 : fenêtre visible à chaque
fois, aucune collision de raccourcis signalée, AX=true et surveillance=true
aux deux relevés ; la seconde session voit la précédente. RSS actuelle et
maximale : 38178816 puis 38084608 octets (36,4 et 36,3 Mio environ). Ces
chiffres décrivent le runner au démarrage, pas le Mac utilisateur ou le repos
prolongé. Aucun test d’action AX sur app tierce ni de geste physique.

Artefact SHA256, CRC des deux ZIP, Info.plist 0.5.2/build17 et Mach-O universel
vérifiés. ZIP applicatif 796552 octets, SHA256
884328e87462cb38e9ab3b45e7b3af4191a687c4e40c035686d80d8a95416b26.

P0/P1 ne sont pas déclarés intégralement clos : sur le Mac pilote, tester la
maximisation depuis un plein écran natif, deux relances sans remplacer le
bundle après autorisation, puis l’essai trackpad 10 % et le rapport. Les p95
d’actions restent non mesurés tant qu’aucune action réelle n’a été exécutée.

## 0.5.3 — Premier plan depuis 100 % et geste court

Retour utilisateur : ⌥T/Chrome échouent depuis une fenêtre à 100 %, ainsi que
les gestes ; l’arrivée au centre n’est pas souhaitée. Le code traitait la
demande d’activation Teams comme une preuve de succès et pouvait abandonner
Chrome avant la fin du passage au premier plan. Cause exacte sur le Mac encore
à confirmer, correctifs ciblés sur ces points observés dans le code.

Activation publique avec toutes les fenêtres et handoff coopératif lorsque
Kigi est actif (macOS 14+, garde de disponibilité pour les anciens OS).
Relevé AX de la fenêtre existante, déminimisation, AXMain si modifiable et
AXRaise. Un seul relevé par demande, attente bornée à 4 s et confirmation du
PID actif ; annulation si une tierce app est choisie. Aucune fenêtre recréée.
Logs structurels de la source plein écran, demande acceptée, relevé et résultat.

Geste : bande 10 % inchangée ; déplacement intérieur ≥0,10 depuis le départ,
plus horizontal que vertical (ratio 1,5), dérive≤0,12 et durée≤1,5 s.
Plus d’arrivée x=0,50 obligatoire. Tests des micro-mouvements, bornes décimales,
diagonales, déclenchement avant le centre et cooldown. Messages et test local
alignés. Deux tests purs ajoutés ; les 20 tests hôte et le typage Mac ont passé
avant interruption de l’environnement local.

Reprise du travail via le connecteur GitHub après indisponibilité de
l’environnement local. Le build Mac vérifiera le format et le contenu final.
Validation native ciblée : fenêtre source à 100 % puis plein écran natif,
activation d’une cible sans puis avec maximisation ; vérifier PID actif et
ordre CGWindowList des fenêtres visibles (PID/layer, sans titre ni screenshot).
Une fixture Swift CI, aucun runtime ajouté au produit Rust. Résultats à
consigner après exécution. Le geste physique reste à essayer sur le Mac pilote.

Validation finale 0.5.3 : [run macOS 43403783033](https://github.com/Stem0794/kigi-windows/actions/runs/43403783033)
réussi sur 8d6203c1ef0e0cf64031b62fe8205f0c97f9ca07. Format vérifié sur
les sources finales, 23 tests macOS, Release Intel/ARM et signature/plist/lipo.
Deux démarrages avec fenêtre visible et précontrôles AX/surveillance à true.

La régression native a passé les deux scénarios : source maximisée à 100 %,
cible relevée sans redimensionnement ; source en plein écran natif, cible
activée puis maximisée. Le PID cible actif ET son ordre devant la fenêtre
source ont été vérifiés via métadonnées CGWindowList, sans screenshot, titre
lu ou frappe synthétique. Il s’agit des fenêtres de fixture sous macOS 15.7.9
Intel, pas d’un essai physique sur le Mac utilisateur ou de ses apps Teams/
Chrome. Le geste physique reste à confirmer.

Archive validée sur le runner : CRC ZIP, version 0.5.3/build18, Mach-O Intel/
ARM ; 803670 octets, SHA256
ae4df0cffb659770593b9aac0d98184ab5693a211fa7f8deb9dcc71618035da9.
Artefact GitHub 14026975889, disponible depuis le run cité. L’environnement
local de fichiers est resté indisponible ; pas de nouveau ZIP copié dans la
conversation ni de remplacement de l’ancienne pièce jointe 0.5.2.

Le premier contrôle à attente fixe de 4 s n’avait pas reçu de rapport avant
arrêt du processus. Il attend désormais le rapport réel (limite15 s) et
vérifie toujours la fenêtre visible ; les deux lancements ont passé. Les
sources ont été formatées à partir de la sortie rustfmt du runner après
reprise GitHub. Aucun résultat d’essai matériel n’est extrapolé de ces tests.

## 2026-11-13 — 0.5.4 : refus AppKit depuis l’arrière-plan

Les logs 0.5.3 du Mac pilote (macOS 26.7.2 ARM64) contiennent 1 856 événements
et 15 gestes reconnus. Les échecs commencent quand AppKit renvoie false depuis
une app tierce : le code quittait avant AXRaise. Même refus pour les raccourcis.
Le tap continue à recevoir et reconnaître les gestes ; aucun changement de seuil
ni de réarmement dans cette correction.

Séparation demande/confirmation : un refus AppKit laisse maintenant poursuivre
la levée AX autorisée. AXFrontmost est tenté si modifiable, puis AXMain/AXRaise ;
le PID réellement au premier plan confirme toujours le succès. Aucune réouverture
d’app ni fenêtre supplémentaire. Permissions et annulation utilisateur conservées.
Deux cas natifs supplémentaires refusent volontairement AppKit dans le seul mode
fixture pour vérifier le chemin AX depuis une fenêtre maximisée et un plein écran.
Validation : format et compilation ARM locale, 20 tests purs ; workflow macOS
[43407827932](https://github.com/Stem0794/kigi-windows/actions/runs/43407827932)
réussi sur macOS 15.7.9 Intel : 23 tests, deux relances, quatre bascules natives
avec fenêtre cible réellement devant la source, dont deux demandes AppKit
volontairement refusées. Bundle 0.5.4/build 19 universel, CRC ZIP et architectures
vérifiés ; 806 932 octets, SHA256
`8d8f47c8cb706cf15c2a3444ab7ded8794e85d7da03dfa0ddceb2c06cb69f0f7`.
Le retour sur macOS 26.7.2 ARM64 et le trackpad physique reste à confirmer.
Ces essais utilisent des fenêtres contrôlées, pas Teams/Chrome sur le Mac pilote.

## 2026-11-13 — Studio 0.6.0, phases 0–1 (branche dédiée)

L’utilisateur confirme la correction 0.5.4 et demande l’étape suivante. Reprise
sur main67f0305 ; contrat Studio utilisateur lu, phases0–1 sur ui/studio-shell.
Audit/branchements/baseline dans docs/studio-contract.md.

Fenêtre redimensionnable locale WKWebView, trois colonnes, Accueil/Espaces/
Autorisations/Diagnostics en lecture seule. Catalogue réel relu sans écriture,
profils libres conservés, fallback signalé, mémoire des auxiliaires inconnue.
Bridge JSON v1 borné/strict/allowlist, document principal local et vue vérifiés,
navigation externe bloquée, assets packagés, CSP sans réseau et valeurs texte.
Aucune modification de la reconnaissance des gestes ou de l’activation0.5.4.

Ancienne UI par défaut ; ouvrir Studio via menu ou --studio-ui, --native-ui force
le secours. Aucun cutover ou merge main dans cette tranche. L’édition des ratios
et la sauvegarde restent la phase2. CI étendue d’un seul essai WKWebView réel,
avec rendu DOM, réouverture et config inchangée ; régressions natives conservées.
Format, typage ARM et22 tests hôte passent. Validation macOS/package en attente.

Validation finale Studio : [run43416541318, tentative2](https://github.com/Stem0794/kigi-windows/actions/runs/43416541318)
réussi au commit1b479adb :25 tests natifs, deux démarrages, bridge/WKWebView/DOM,
fermeture-réouverture, config inchangée et quatre régressions de focus. La première
tentative avait échoué sur un timeout AX250ms à la maximisation plein écran,
après succès Studio ; relance unique sans modification du moteur ou du contrôle.
RSS Rust37 343 232→42 467 328 octets (auxiliaires WebKit non mesurés).
Bundle0.6.0/build20 universel, assets locaux, CRC/plist/architectures vérifiés ;
868101 octets, SHA256e39663841f736ec10bde6ab0776dc7aeaeeac20b30494611e687848753411d74.
[PR#1](https://github.com/Stem0794/kigi-windows/pull/1), main conservée en0.5.4.
Essais physiques Studio/VoiceOver encore requis ; phase2 non commencée.

## 2026-11-13 — Studio0.6.1, phase2 éditeur (branche séparée)

Suite demandée pendant l’essai0.6.0. Base ui/studio-shell66ab813, nouvelle branche
ui/studio-editor. Édition1–99 %, presets, drag/clavier, champs liés, choix d’apps natif,
CRUD et sauvegarde Rust. Brouillons non persistés, garde des changements à la navigation,
fermeture et ⌘Q. Profils libres et snapshots conservés ; aucun Apply/Restore Studio.
Révision opaque + comparaison des octets, backup/écriture atomique et refus d’un primaire
invalide. Le détail de la fenêtre de concurrence résiduelle est dans studio-contract.
Typage ARM, syntaxeJS et24 tests hôte passent. Essais natifs/package en attente.

Validation finale : [run43423531866](https://github.com/Stem0794/kigi-windows/actions/runs/43423531866)
réussi au commitff974dbee08e80619cbb4c4399d50e53b3a593d5 :27 tests macOS15.7.9
Intel, deux relances, quatre cas de focus, bridge/DOM/cycle Studio. Le vrai parcours
éditeur a converti explicitement un ancien profil libre schema1, sauvegardé35/65,
vérifié Annuler, clavier, navigation avec brouillon, copie/suppression, conflit
conservant le brouillon et annulation de fermeture. Seconde instance :35/65 relu
sur schema2. Tout ce parcours écrit uniquement un catalogue temporaire isolé.

Premier run43422385218 : moteur/Studio passent, parcours éditeur en timeout UI.
Correction de la réactivation des boutons après Actualiser et conservation des
valeurs numériques invalides lors d’un rerender ; contrôle de non-régression ajouté.
Le nouveau run passe sans changement de limite AX ni assouplissement d’assertion.
ClippyARM termine avec les seuls5 needless_borrow préexistants dans mouse_scroll.rs.

Package0.6.1/build21 universel Intel/ARM :918563 octets, SHA256
c2f2b65245aff0bbd9d6d1afd0ee16dbd308608347e1deb0b0f96effb89ba6c3.
CRC/plist/architectures et ressources embarquées vérifiés. [PR#2](https://github.com/Stem0794/kigi-windows/pull/2)
empilée sur PR#1 ; main et branche0.6.0 conservées. Choix d’app natif, drag physique,
VoiceOver et coûts WebKit auxiliaires restent à essayer/mesurer sur le Mac pilote.

## 2026-11-13 — Studio0.6.2, phase3 Apply/Restore

Suite demandée pendant l’essai0.6.1. Branche ui/studio-apply basée sur badfb77,
sans modifier les branches d’essai précédentes. Appliquer/Restaurer Studio reliés
au moteur partagé : préflights, sélection des fenêtres, snapshot unique, ratios
normalisés et comparaison AX réelle. Séparation placement/activation, retour typé
applied/partial/restored/canceled et géométrie inconnue nullable. Les retours partiels
restent restaurables ; déplacement manuel ignoré suivant la politique existante.
Révision du profil recontrôlée après le sélecteur. Aucune réécriture des proportions
constatées dans le catalogue. Pas de parité système ni de test brouillon previewApply.
24 tests hôte, typage ARM, syntaxeJS passent ; Clippy ne signale que les5 anciens
needless_borrow dans mouse_scroll.rs. Validation native/package en attente.

Validation finale : [run43434129431](https://github.com/Stem0794/kigi-windows/actions/runs/43434129431)
réussi au commit1b91c64afeb579e3885c83201fd749b529f21a51, macOS15.7.9 Intel.
27 tests natifs, deux relances, quatre cas de focus, chargement/cycle Studio et
parcours éditeur/relecture passent. Nouveau parcours réel Apply/Restore :
35/65 demandé et constaté, puis demande35/65 contrainte à55/65 signalée partial.
Dans les deux cas, observation CGWindowBounds indépendante pendant le placement,
valeurs affichées conformes à cette mesure, retour initial restauré/vérifié.
Annuler l’application sale ne crée pas de retour ; Enregistrer et appliquer garde
35/65 ; un second placement refuse d’écraser le snapshot. Cas contraint : octets
config inchangés après apply/restore. Le premier run43433626306 avait déjà passé
le même code moteur ; le second renforce l’observation indépendante des dimensions.
Aucune limite AX ou assertion de résultat n’a été assouplie.

Package0.6.2/build22 universel :939445 octets, SHA256
99954359d32a62a3c3942734ccdf9ea4d97072d9d190ea8ed9d94e000d64a183.
[PR#3](https://github.com/Stem0794/kigi-windows/pull/3) empilée sur PR#2 ; branches
0.6.0/0.6.1 et main conservées. Chrome/Teams réels, choix ambigu/annulation du
sélecteur, déplacements manuels et changements d’écran restent à essayer sur le
Mac pilote ; politique de non-écrasement manuel couverte par les tests purs existants.

## 2026-11-13 — Studio0.6.3, phase4 réglages système

Suite demandée après0.6.2, nouvelle branche ui/studio-system. Pages Raccourcis,
Trackpad, Souris, Autorisations et Diagnostics reliées aux services natifs.
CRUD des raccourcis + revision/octets exacts, rollback Carbon/writer, global/reload/
reregister/reset, choix d’app et test explicite. Recorder natif modal avec suspension,
timeout15s et annulation ; aucun captureur JS. Données réelles, Chrome mesuré au dernier
appel, bilans et copie/export backend. UI des essais bornée13s, pas de polling au repos.
Brouillons partagés avec la garde de fermeture ; moteur/gestes10 % inchangés.
26 tests hôte, typage ARM et syntaxeJS passent ; Clippy : les5 avertissements préexistants
du filtre souris seulement. Cas natif Carbon/capture/pages/relaunch en attente du run.
