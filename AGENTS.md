# Consignes Codex — `kigi-windows`

**Dépôt unique :** https://github.com/Stem0794/kigi-windows
**Cahier des charges :** lire `HANDOVER_CODEX.md`. Le JSON dans `examples/default-config.json` est indicatif, pas une fonctionnalité déjà implémentée.

## Règles non négociables

- **Travaille dans ce dépôt existant uniquement.** Inspecte ses fichiers et son historique avant de modifier. Ne crée pas de dépôt concurrent et n'initialise pas de code hors du répertoire `kigi-windows`.
- **KISS / pas d'overengineering.** Un seul crate Rust pour commencer ; quelques modules, du code direct, le minimum de dépendances. N'introduis aucun framework interne, abstractions génériques, microservices, daemon, base de données ou système de plugins sans besoin réel démontré.
- **Pas de 10 000 smoke tests.** Ne construis pas de suite E2E ou CI lourde. Lance `cargo fmt --check`, `cargo check` et les tests utiles après changement significatif ; ajoute quelques tests unitaires seulement pour les logiques présentant un vrai risque. Contrôle manuellement sur Mac les fonctions Accessibility, raccourcis et trackpad. Pas de campagnes répétitives sans bug à investiguer.
- **Une phase à la fois.** N'implémente que la phase demandée, avec un commit cohérent et quelques lignes de suivi dans `docs/progress.md`. Ne code aucune fonction future « au cas où ».
- **Rust-first, API natives macOS.** AppKit `objc2`, pas Electron, WebView, Hammerspoon embarqué, Lua ni service cloud. Un seul processus natif.
- **Ne pas inventer les résultats.** Geste trackpad et ancien macOS : GO/PARTIAL/NO-GO basé sur essais réels. Cible macOS 11, Intel + ARM à valider.
- **Respect des autorisations.** API publiques Apple sauf accord explicite ; aucun contournement TCC/MDM, pas de root.
- **Migration en sécurité.** Ne modifie ni ne supprime `~/.hammerspoon/init.lua` sans consentement explicite ; audit en lecture seule.
- **Vie privée.** Ne capture ni frappes, ni contenu des applications, ni screenshots ; pas de titres sensibles dans les logs par défaut.

## Première mission

**P0 uniquement :** application barre de menus minimale, un raccourci, test Accessibility, spike du trackpad. Rapport bref `docs/trackpad-spike.md`, puis s'arrêter. Pas d'éditeur de scènes, de réunion Teams automatique, de licensing ou de CI avancée pendant P0.

## Phase actuelle — 12 novembre 2026

L'utilisateur a validé déplacement/restauration 0.0.6 et le diagnostic trackpad
sur son Mac, puis demandé de continuer. La première mission P0 ci-dessus est
historique : poursuivre **P1, raccourcis et actions manuelles**, en utilisant le
relevé `docs/migration-audit.md` comme référence. P2 gestes actifs et P5 réunion
automatique restent distincts. Ne pas annoncer les performances ou les nouveaux
comportements validés sur matériel avant retour utilisateur.

## Étape suivante autorisée — P2 pilote

L'utilisateur confirme les raccourcis manuels, demande la correction de ⌥C
(réutiliser l'onglet existant) et explicitement « continue la prochaine étape ».
P2 peut maintenant relier les gestes aux actions Teams/Chrome, sur activation
volontaire, avec les bords de la référence (gauche 10 %, droite 20 %). Distinguer
les tests purs et la compilation des essais terrain encore requis. Les scènes
P3 et les réunions automatiques P5 restent hors de cette étape.

## État GitHub repris — P3 0.3.1

La branche main contient désormais P2 horizontal (5 % à chaque extrême vers le
centre) et P3 manuel Travail/Réunion/restauration en 0.3.0. L'utilisateur demande
de poursuivre depuis GitHub. Conserver ces gestes et avancer dans P3 : capture
persistante d'une disposition personnelle de deux fenêtres sur un même écran,
JSON versionné/atomique, restauration fiable. Ne pas activer de détection Teams
ni développer l'éditeur complet P4 dans cette sous-étape.

## Option souris demandée — 0.3.2

L'utilisateur demande explicitement une option de désactivation du scroll
horizontal des souris Apple. Filtre natif ScrollWheel activable/mémorisable,
distinction conservatrice par phases publiques et passage des inconnus. Ne pas
prétendre identifier la marque avec CGEvent, ne pas utiliser d'API privée, ne
pas affecter le tap passif des gestes. Valider Magic Mouse/trackpad sur matériel
avant toute garantie ; pas de fonctionnalités de remappage supplémentaires.

## P4 pilote autorisé — 0.4.0

Après 0.3.2, l'utilisateur confirme « ça fonctionne bien » et demande la
prochaine étape. Développer une première fenêtre AppKit pour plusieurs espaces
nommés : capture, recapture, application, renommage, duplication, suppression
et positions simples, sans édition JSON. Migrer la disposition schema1 sans
la perdre ; conserver restauration, gestes5 % et filtre souris. Ne pas ajouter
P5/réunions automatiques ni un éditeur graphique complexe dans cette tranche.

## Interface principale demandée — 0.4.1

Ouvrir les réglages au lancement avec présence Dock, cinq onglets natifs et
menu de statut compact. Fermer conserve les fonctions de fond ; ⌘Q quitte.
Conserver les options réelles existantes, sans P5 ni capture de raccourcis.
