# Référence de migration Hammerspoon

Source : fichier `Pasted text.txt` fourni par l'utilisateur le 12 novembre 2026,
conservé intégralement dans `examples/hammerspoon-reference.lua` (455 lignes).
Le fichier réellement chargé sur le Mac n'a pas été inspecté ; aucun accès ni
changement à `~/.hammerspoon/init.lua`. Ce relevé remplace les hypothèses du
handover pour la future reproduction, sans déclarer ces fonctions implémentées.

| Déclencheur | Comportement exact de la configuration fournie | État Kigi P0 0.0.4 |
|---|---|---|
| Option + T | Trouver Teams (avec repli par nom hors helpers), démasquer/activer ; lancer si absent | À migrer ; Control + Option + T reste le test AX |
| Option + B | Activer/lancer Chrome ; après 0,3 s, maximiser sa fenêtre principale ou première fenêtre standard | À migrer |
| Option + C | AppleScript Chrome : retrouver un onglet ChatGPT existant dans toutes les fenêtres ; sinon ouvrir un onglet/fenêtre | À migrer ; permission Automatisation Chrome à demander uniquement pour cette fonction |
| Command + Shift + Haut | Déminimisation si nécessaire, sortie de plein écran natif puis maximisation après 1 s ; relever/focaliser | À migrer ; le test P0 refuse encore les fenêtres minimisées/plein écran |
| Command + Shift + Gauche | Choisir deux fenêtres distinctes, d'abord celle de gauche, puis celle de droite ; écran déterminé avant les choix | À migrer ; ce n'est pas un simple déplacement de la fenêtre active |
| Command + Shift + Droite | Même sélection en commençant par la fenêtre droite | À migrer |
| Control + Command + Shift + T | Rechercher chat/réunion Teams et réappliquer le partage chat gauche / réunion droite | À migrer |
| Fenêtre Teams créée/détruite/titre modifié | Debounce 0,8 s, repli par contrôle toutes les 5 s ; appliquer une fois le partage à la détection de réunion | À migrer ; heuristique à confirmer sur la version Teams utilisée |
| Fin de réunion suivie ou titre devenu non réunion | Maximiser le chat Teams suivi, ou le premier chat disponible | À migrer ; ce n'est pas une restauration générale de snapshot ni un retour Chrome systématique |
| Chargement/rechargement, après 1,5 s | Synchroniser Teams ; en l'absence de réunion suivie, maximiser Chrome | À migrer |
| Glissement descendant bord gauche, un doigt | Activer Option + T via une frappe synthétique | Diagnostic seulement ; future action native Teams directe plutôt qu'une frappe synthétique |
| Glissement descendant bord droit, un doigt | Activer Option + B via une frappe synthétique | À migrer |

## Écarts importants à conserver pour la migration

- La configuration utilise **toute la hauteur des bords**, pas uniquement les
  coins supérieurs. Le diagnostic P0 actuel reste un coin haut-gauche. Ce
  changement de forme devra être annoncé lors de l'activation du profil réel.
- Le fichier fourni contient une largeur de bord de 20 %. L'utilisateur a
  demandé ensuite une zone gauche plus étroite : **10 % à gauche** est donc la
  préférence la plus récente. Le bord droit du fichier reste à 20 %.
- Seuils fournis : descente ≥0,22 ; dérive horizontale ≤0,12 ; durée ≤1,5 s ;
  cooldown global de 1 s ; une seule action par contact. Le Lua peut démarrer
  sur un mouvement sans phase initiale. Le prototype Rust exige un vrai début
  pour éviter de réarmer accidentellement après interruption/multitouch.
- Deux vraies fenêtres Teams sont nécessaires : chat reconnu par préfixe
  `Chat |`, réunion par préfixe avant `|`, exclusion de pages ordinaires et
  marqueur d'organisation présent dans la référence. C'est une heuristique,
  pas une preuve universelle de réunion ; les titres ne doivent pas entrer
  dans les logs de diagnostic.
- Un changement du titre de réunion peut être pris à tort pour une fin de
  réunion par cette règle. Pour la migration, garder cette limite visible et
  l'automatisme désactivable ; pas de promesse de détection fiable sans essai.
- Les deux sélecteurs ignorent Hammerspoon, fenêtres non standard, minimisées,
  plein écran et fenêtre choisie en premier. Les titres servent à choisir dans
  l'interface, jamais à alimenter les logs transmissibles.
- L'intégration AppleScript devra rester dans le processus macOS ; pas de
  processus `osascript`, d'analyse du contenu des pages ou d'historique Chrome.

## État P1 0.1.0

L'utilisateur confirme l'Accessibilité fonctionnelle et le retour direct
100 % / droite / 100 % en 0.0.6. Le diagnostic trackpad fonctionne sur son Mac.
P1 est autorisé par sa demande de poursuivre l'implémentation.

Implémenté : ⌥T, ⌥B, ⌥C (URL simple), ⌘⇧↑ et les deux sélections
⌘⇧←/→, avec second choix focalisé. Les sélecteurs sont des listes natives
simples, sans champ de recherche à ce stade. ⌃⌥T reste le test/restauration.
Les conflits sont individuels, affichés et réenregistrables depuis le menu ;
suspension des raccourcis personnels pour la session.

Écarts explicites : ⌥C peut ouvrir un nouvel onglet ; pas d'Apple Events ni de
réutilisation d'onglet. Maximiser refuse le plein écran natif au lieu de forcer
sa sortie ; Chrome minimisé est déminimisé. Aucun ⌘⇧↓ ajouté : absent de la
référence réelle. Les gestes actifs et l'automatisation Teams restent à faire.

Les titres sont limités au sélecteur en mémoire ; aucune trace dans les logs.
Un lancement/focus ne demande pas de nouvelle fenêtre. La fin du lancement et
l'apparition AX sont attendues au plus 3 s, uniquement pendant l'action.
Validation fonctionnelle des nouvelles commandes, p95 et RSS encore à relever
sur le Mac pilote. Les tableaux P0 précédents restent le relevé historique.

## P2 pilote 0.2.0

À la demande suivante, ⌥C est porté vers NSAppleScript in-process avec recherche
d'onglet existant (hôtes exacts, pas préfixe trompeur), activation et création
uniquement en l'absence de correspondance. Automatisation Chrome séparée et
explicite ; aucun repli créant un doublon après refus/erreur.

Gestes activables par menu : bande gauche10 % et droite20 %, toute hauteur,
une descente → Teams/Chrome100 %. Pas de frappe synthétique. Seuils de la
référence, cooldown1s, ajout du début strict et annulation jusqu'au retrait
complet. Pause après veille/session inactive/interruption, reprise manuelle.
Toujours aucune heuristique automatique Teams ni modification du Lua original.
Les fonctionnalités sont compilées ; elles attendent le retour terrain P2.
