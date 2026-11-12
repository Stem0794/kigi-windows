# Kigi Windows — pilote gestes 0.2.0

Application macOS native en Rust, un seul crate et un seul processus. Binaires
Intel + Apple Silicon ; cible macOS 11 non encore validée sur matériel ancien.

## Installer et utiliser

Quitter l'ancienne copie, remplacer `Kigi Windows.app` au même emplacement,
puis relancer. Kigi apparaît dans la barre de menus. Cette version utilise une
signature ad hoc, sans Developer ID ni notarisation ; les autorisations macOS
peuvent devoir être réaccordées après remplacement.

| Raccourci | Action |
| --- | --- |
| ⌥T | Activer Teams ou lancer son bundle principal installé |
| ⌥B | Activer/lancer Chrome et maximiser sa fenêtre principale |
| ⌥C | Retrouver un onglet ChatGPT existant dans Chrome, sinon en ouvrir un |
| ⌘⇧↑ | Maximiser la fenêtre active dans la zone visible de son écran |
| ⌘⇧← | Choisir gauche, puis droite : deux fenêtres à 50 % |
| ⌘⇧→ | Même partage en choisissant d'abord la fenêtre droite |
| ⌃⌥T | Test conservé : moitié droite puis restauration de la même fenêtre |

Les touches désignent les positions physiques macOS. L'enregistrement est
indépendant ; un conflit ne bloque pas les autres touches. Si nécessaire,
quitter temporairement Hammerspoon puis **Réenregistrer les raccourcis** depuis
Kigi. Son fichier n'est jamais modifié. Le menu propose aussi toutes les actions
et la suspension des raccourcis personnels, pour la session.

## ChatGPT : onglet existant

Au premier ⌥C, accepter **Automatisation → Google Chrome** si macOS le demande.
C'est distinct de l'Accessibilité. Un script fixe NSAppleScript s'exécute dans
Kigi, sans processus osascript : il compare uniquement les URL des onglets aux
hôtes exacts `chatgpt.com` et `chat.openai.com`, active le premier trouvé dans
les fenêtres Chrome, ou crée un onglet après une recherche sans correspondance.
Les onglets existants ne sont ni rechargés ni remplacés.

Si la recherche échoue, l'app affiche l'erreur et n'ouvre aucun onglet de secours.
Une permission refusée se réactive dans Réglages Système → Confidentialité et
sécurité → Automatisation. Aucun accès au contenu des pages, historique ou
stockage d'URL. Les logs indiquent seulement `reused`, `created` ou code d'erreur.
La demande de consentement peut attendre l'utilisateur ; le timeout AppleScript
3 s s'applique aux requêtes individuelles, pas à une durée totale de dialogue.

## Gestes de bord (P2 pilote)

Choisir **Kigi → Activer les gestes de bord Teams / Chrome**. Désactivés au
lancement et après redémarrage, ils nécessitent Accessibilité et, si macOS la
demande, Surveillance de l'entrée.

| Départ et mouvement, un seul doigt | Action |
| --- | --- |
| 10 % les plus à gauche, glissement vers le bas | Teams au premier plan |
| 20 % les plus à droite, glissement vers le bas | Chrome au premier plan à 100 % |

Le départ peut se faire sur **toute la hauteur**, conformément à la référence
Hammerspoon, puis descente ≥0,22, dérive horizontale ≤0,12 et durée ≤1,5 s.
Une action par contact, cooldown global 1 s. Deux doigts, clic/drag observé,
identité/device différent, mauvaise direction, délai ou début manquant annulent.
Une séquence annulée doit se terminer par retrait de tous les doigts avant de
réarmer. Une commande manuelle annule le geste encore en attente. Aucune action
pendant les sélecteurs, l'intégration Chrome ou le menu Kigi ouvert.

L'écoute est passive : pas de déplacement du pointeur ni de consommation des
gestes macOS. L'action est différée après le callback du tap ; aucun AX ou Apple
Events dans ce callback. Un seul tap existe, soit en diagnostic, soit en actions.
Le menu permet de désactiver ; ouvrir le diagnostic global pendant l'écoute
active l'arrête et affiche son bilan. Recliquer démarre le diagnostic sans action.

Veille du Mac/écran, changement de session et interruption du tap arrêtent
l'écoute et les actions en attente. **Réactiver manuellement après reprise**,
sans boucle de relance automatique. Un retrait/rebranchement de périphérique
invalide le contact ; lever les doigts puis recommencer. Le verrouillage sans
veille et les faux positifs restent à vérifier sur le Mac pilote : aucune
promesse de fiabilité universelle. Pas d'API privée ou de lecture de TCC.

Le diagnostic local reste un coin haut-gauche x≤0,10 / y≥0,80, sans action.
Le diagnostic global conserve ce même coin ; ce n'est pas le profil de bord.

## Permissions et limites fenêtres

La ligne Accessibilité est actualisée à l'ouverture du menu. La première demande
est explicite et unique par session, suivie d'un contrôle temporaire borné.
Une case d'une ancienne version peut ne pas autoriser le processus actif :
**Afficher la copie active dans Finder** permet de choisir le bon bundle.
L'erreur AX est expliquée une fois par session pour toutes les actions de
placement. Ouvrir Teams ou Chrome ne valide pas l'autorisation de placement.

Les sélecteurs utilisent les titres en mémoire pour choisir, jamais dans les
logs. Deux fenêtres distinctes sont nécessaires, standard, non minimisées et
hors fullscreen ; Hammerspoon et Kigi sont exclus. Un inventaire borné peut
être partiel : c'est indiqué. Annuler ne déplace rien ; le second choix reçoit
le focus. L'écran est celui de la fenêtre active avant le sélecteur.

Chrome peut être déminimisé pour ⌥B. Le fullscreen natif est refusé explicitement.
Les apps peuvent imposer leurs dimensions : AX relit la géométrie et signale un
résultat partiel. Position/taille/position reprend le correctif 0.0.6 validé par
l'utilisateur. Le snapshot de ⌃⌥T reste en mémoire : restaurer avant de quitter.
Les lancements NSWorkspace sont asynchrones, avec contrôle temporaire ≤3 s ;
une nouvelle commande annule le traitement en attente, pas le lancement OS.

## Logs et essai court

Reproduire un problème, puis **Copier les logs de diagnostic** ou **Exporter les
logs…**, avant de quitter. Journal limité aux 200 derniers événements de session,
avec version/OS/architecture, permissions, conflits, actions et leurs résultats,
rectangles AX, compteurs et interruptions du tap. Pas de frappes, titres, contenu
d'apps, URL d'onglets, identité des contacts ou coordonnées de doigts. Aucun envoi
réseau, aucun journal persistant automatique.

Sur le Mac pilote : ⌥C deux fois avec ChatGPT déjà ouvert (nombre d'onglets
inchangé), avec ChatGPT absent (un seul nouvel onglet), puis un geste volontaire
de chaque bord. Essayer aussi un départ central, deux doigts et l'usage ordinaire
pour relever les faux positifs ; désactiver si nécessaire. Après veille,
confirmer l'arrêt puis réactiver volontairement. Les nouveaux comportements
restent à valider sur matériel ; p95/RSS n'ont pas été mesurés ici.

## Construire et vérifier

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target aarch64-apple-darwin
# Sur macOS avec Rust stable et outils Xcode :
bash scripts/bundle.sh universal
open "dist/Kigi Windows.app"
```

Le workflow macOS lie les deux architectures, signe ad hoc et crée le ZIP.
Il compile aussi le script fixe si Chrome est installé, sans l'exécuter ni
demander Automatisation. Les checks Linux/macOS ne remplacent pas les essais
Chrome, permissions et gestes sur un Mac. Voir [progress.md](docs/progress.md),
[trackpad-spike.md](docs/trackpad-spike.md) et [migration-audit.md](docs/migration-audit.md).
Pas d'éditeur, scènes persistantes, détection de réunion ou système commercial.
