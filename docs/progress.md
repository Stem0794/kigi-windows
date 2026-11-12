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
