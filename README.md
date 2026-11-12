# Kigi Windows — prototype P0

Application macOS native en Rust, un seul processus et un seul crate. Cible
macOS 11, Intel et Apple Silicon. Le code natif est vérifiable depuis Linux,
mais le fonctionnement sur Mac et la compatibilité OS restent à valider.

## Lancer sur un Mac

Installer Rust stable et les outils de ligne de commande Xcode, puis :

```sh
git clone https://github.com/Stem0794/kigi-windows.git
cd kigi-windows
bash scripts/bundle.sh
open "dist/Kigi Windows.app"
```

Le menu `Kigi` apparaît dans la barre de menus. Conserver ce bundle à ce chemin
pendant les essais de permissions. La signature ad hoc convient au prototype
local ; une recompilation peut nécessiter une nouvelle autorisation TCC.
Aucun package commercial signé/notarisé n'est fourni à ce stade.

1. Cliquer « Autoriser / vérifier Accessibilité », autoriser Kigi dans les
   réglages macOS, puis recliquer pour vérifier. Rien n'est demandé au lancement.
2. Ouvrir Chrome ou Teams, puis « Détecter les fenêtres ». Seuls les bundles,
   nombres de fenêtres et géométrie sont affichés, sans lire les titres/contenus.
   L'inventaire est ponctuel et peut être partiel pour les apps AX inaccessibles.
3. Activer une fenêtre normale, puis appuyer sur **⌃⌥T** : elle passe à droite dans
   la zone visible de son écran. Appuyer sur **⌃⌥T** de nouveau restaure **cette même
   fenêtre**, même si le focus a changé. Le menu offre la même action ; son
   premier élément affiche le dernier résultat ou l'erreur.
4. Tester le diagnostic local du trackpad. Le pointeur doit rester dans la
   fenêtre de diagnostic. Un départ x≤0,10 / y≥0,80, puis une descente ≥0,22
   en ≤1,5 s, avec un seul doigt et dérive horizontale ≤0,12, incrémente un compteur.
5. Démarrer volontairement le diagnostic global passif, autoriser séparément
   Surveillance de l'entrée si nécessaire, passer dans Chrome/Teams et essayer
   le geste. Recliquer sur le même menu arrête l'écoute et affiche les compteurs.
   Une absence d'événements ne constitue pas une validation du geste.

**⌃⌥T est un raccourci de test P0**, pas encore l'action Teams de P1. Le code
Carbon utilise la position physique de la touche T. Cette combinaison évite le ⌥T Teams de Hammerspoon. En cas de conflit, l'erreur
reste visible et le menu fonctionne. Quitter temporairement Hammerspoon et relancer Kigi pour
l'essai si nécessaire ; son fichier `init.lua` n'est jamais modifié.

Le snapshot de test reste en mémoire seulement : restaurer avant de quitter.
Si la restauration échoue ou reste partielle, le snapshot est conservé pour
réessayer. Les fenêtres plein écran/minimisées ne sont pas sorties de cet état.
Certaines apps imposent des tailles minimales ou répondent tardivement à AX.
Le contrôle relit immédiatement la géométrie, sans polling ni retries infinis.

## Vérifications

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
# Vérification de types macOS, possible depuis Linux sans linker/SDK :
rustup target add x86_64-apple-darwin aarch64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target aarch64-apple-darwin
# Sur Mac, bundle universal facultatif après installation des deux cibles :
bash scripts/bundle.sh universal
```

Les six tests sur Mac portent sur la conversion d'écrans et les cas critiques du
classifieur diagnostic et le déclenchement unique du raccourci, ainsi que la borne du journal de diagnostic et la demande unique d’Accessibilité. `cargo check` ne valide ni l'édition de liens Apple,
ni TCC, ni les appels exécutés, ni la détection du trackpad.

Voir [le rapport trackpad](docs/trackpad-spike.md), la
[décision API](docs/adr-001-trackpad.md) et le [suivi P0](docs/progress.md).
Le JSON du handover reste indicatif et n'est pas chargé par P0. Aucun éditeur,
workspace, automatisme Teams, lancement de navigateur ou système commercial.

## Si le raccourci semble sans effet (0.0.3)

Le menu contient une ligne permanente « Raccourci ⌃⌥T : enregistré — reçu N fois ».
Cliquer dessus ouvre un diagnostic avec la version, l'état Accessibilité et le
dernier résultat de l'action. Le compteur augmente à l'appui, une seule fois
même si la touche reste enfoncée. Une erreur d'enregistrement reste visible.

Compteur à zéro après un essai : le raccourci n'a pas été reçu ; essayer sans
autre gestionnaire et relancer Kigi si nécessaire. Compteur augmenté sans
déplacement : consulter l'erreur AX, vérifier l'accès Accessibilité pour le
bundle remplacé et fermer le diagnostic local avant de réactiver Chrome.
Le menu « Tester / restaurer » exécute la même action indépendamment du clavier.

## Logs transmissibles (0.0.4)

Le menu propose **Copier les logs de diagnostic** pour les coller dans un
message, et **Exporter les logs…** pour choisir un fichier `.txt` à transmettre.
Reproduire le problème puis copier/exporter avant de quitter Kigi : le journal
est limité aux 200 derniers événements de la session et n'est pas conservé au
redémarrage. Aucun fichier automatique, envoi réseau ou dépendance de logging.

Le rapport contient version, architecture, macOS, chemin de l'exécutable
(répertoire personnel abrégé en `~`), état AX, enregistrement/réception du
raccourci, actions, erreurs AX, géométrie demandée/obtenue et bilan du diagnostic
trackpad s'il est actif. Il exclut frappes, titres, contenu des applications,
URLs d'onglets, identifiants de contacts et coordonnées de doigts.

Si le raccourci est reçu mais AX refusée, une explication apparaît une seule
fois par session. Autoriser l'application actuelle dans Accessibilité, puis
réessayer ; le raccourci et le diagnostic trackpad ne suffisent pas à prouver
que cette permission est accordée.

La configuration Hammerspoon fournie est conservée dans
[examples/hammerspoon-reference.lua](examples/hammerspoon-reference.lua).
Le [relevé de migration](docs/migration-audit.md) distingue ses comportements
exacts de ceux du prototype P0 : elle n'est pas exécutée ni importée comme code.

## Accessibilité cochée mais non reconnue (0.0.5)

Le premier clic sur la ligne Accessibilité peut demander l'autorisation. Les
clics suivants **vérifient sans relancer la demande**. Après le premier clic,
Kigi vérifie toutes les deux secondes pendant environ 30 secondes (15 contrôles au maximum), puis arrête
ce suivi ; ouvrir le menu ou tester une fenêtre effectue aussi une vérification
à jour. Il n'y a pas de scrutation permanente au repos.

- « Afficher la copie active dans Finder » sélectionne l'application qui tourne
  réellement : c'est cette copie qu'il faut ajouter dans les réglages macOS.
- « Diagnostic Accessibilité » affiche version, PID, chemin et résultat brut
  des deux API de confiance, sans demander à nouveau l'autorisation. Un essai
  AX en lecture seule indique le code renvoyé par macOS pour `AXWindows` d'une
  application tierce ; aucun titre ou contenu n'est lu, aucune fenêtre déplacée.
- Les logs incluent aussi le nombre d'autres processus Kigi et la présence
  d'un chemin temporaire `AppTranslocation`. Une nouvelle copie refuse de
  démarrer si une autre instance du même bundle est encore active.

Pour un essai propre : quitter Kigi, conserver une seule copie dans Applications,
la lancer, la sélectionner avec le menu Finder, puis autoriser cette application
actuelle dans Accessibilité. Si macOS exige une relance, quitter puis rouvrir
cette même copie. Si le refus persiste, copier les logs avant toute nouvelle
mise à jour ; ils permettent de distinguer le précontrôle du refus AX réel.

La signature des packages reste **ad hoc**. Elle ne garantit pas la conservation
des autorisations entre deux builds. Une identité de signature Apple stable
sera nécessaire pour fiabiliser les mises à jour ; ce correctif ne remplace
pas cette identité et ne modifie ni la signature de confiance ni la base TCC.

Sources : [retour asynchrone AXIsProcessTrustedWithOptions](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions),
[explication Apple des changements d'identité ad hoc](https://developer.apple.com/forums/thread/819406)
(dans un cas ScreenCapture ; piste d'identité à vérifier ici, sans cause AX
confirmée sur le Mac utilisateur).

## Restauration du test fenêtre (0.0.6)

Le test doit alterner entre la moitié droite et la géométrie initiale. Le
passage durable par une moitié gauche au deuxième appui est un résultat partiel,
pas un troisième état voulu. La version 0.0.6 positionne d'abord la fenêtre
avant de l'agrandir, puis réapplique sa position, pour éviter qu'une expansion
depuis le bord droit soit limitée à la largeur restante de l'écran. Vérifier
sur le Mac le parcours 100 % / moitié droite / 100 % avec deux appuis. Les logs
contiennent l'ordre des écritures AX, la cible et la géométrie obtenue.
