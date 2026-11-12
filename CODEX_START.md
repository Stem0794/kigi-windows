# Démarrer avec Codex dans le dépôt officiel

Dépôt de travail : **https://github.com/Stem0794/kigi-windows**

1. Dans Codex, sélectionne ou clone **ce dépôt existant**. Ne crée pas de nouveau dépôt ni de répertoire de projet parallèle.
2. Place à sa racine `HANDOVER_CODEX.md`, `AGENTS.md`, `CODEX_START.md`, et `examples/default-config.json` dans `examples/` si ces fichiers ne sont pas encore présents.
3. Commence par **P0 uniquement**. L'app devra être testée sur un vrai Mac avant de valider les permissions et le trackpad.

## Prompt à envoyer à Codex

> Travaille uniquement dans https://github.com/Stem0794/kigi-windows. Inspecte l'état du dépôt puis lis intégralement `AGENTS.md`, `HANDOVER_CODEX.md` et `examples/default-config.json`. **Garde le projet le plus simple possible : un seul crate Rust, peu de fichiers et de dépendances, aucune sur-ingénierie. Ne fais pas 10 000 smoke tests, pas de framework E2E, pas de fausse batterie de tests automatisés.** Ajoute 1 à 3 tests unitaires pertinents lorsqu'une logique pure le justifie, et effectue des vérifications manuelles ciblées pour les API macOS. Exécute uniquement P0 : petite app barre de menus native, raccourci global, test AX de déplacement/restauration de fenêtre et prototype de détection passive d'un geste trackpad, en documentant ce qui fonctionne réellement. Ne modifie pas `~/.hammerspoon/init.lua`. Cible macOS 11 / Intel + Apple Silicon sans prétendre avoir validé les systèmes indisponibles. À la fin, fournis le commit, commandes et tests réellement exécutés, limites et GO/PARTIAL/NO-GO du trackpad. N'entame pas P1.

## Suite

Suivre P1 → P7 seulement après validation explicite de chaque phase, sans ajouter d'abstractions ou de tests massifs avant le besoin. En cas de blocage des gestes, conserver les raccourcis et fonctions fenêtres utilisables.
