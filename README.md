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
3. Activer une fenêtre normale, puis relâcher **⌃⌥T** : elle passe à droite dans
   la zone visible de son écran. Relâcher **⌃⌥T** de nouveau restaure **cette même
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

Les trois tests portent sur la conversion d'écrans et les cas critiques du
classifieur diagnostic. `cargo check` ne valide ni l'édition de liens Apple,
ni TCC, ni les appels exécutés, ni la détection du trackpad.

Voir [le rapport trackpad](docs/trackpad-spike.md), la
[décision API](docs/adr-001-trackpad.md) et le [suivi P0](docs/progress.md).
Le JSON du handover reste indicatif et n'est pas chargé par P0. Aucun éditeur,
workspace, automatisme Teams, lancement de navigateur ou système commercial.
