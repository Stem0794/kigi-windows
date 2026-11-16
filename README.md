# Kigi Windows / Kigi Studio

Application macOS autonome : moteur Rust natif, interface Studio locale, sans
Hammerspoon, compte ni service réseau. Fermer Studio conserve les fonctions de
fond ; ⌘Q quitte. L’icône de menu et le Dock rouvrent les réglages.

## Workflow personnel

- ⌥T : ouvrir/retrouver Teams ; ⌥B : Chrome100 % ; ⌥C : retrouver ChatGPT.
- ⌘⇧↑ : maximiser ; ⌘⇧←/→ : sélectionner deux fenêtres à partager.
- ⌃⌘⇧T : réappliquer chat/réunion Teams ; ⌘⇧↓ : moitié droite directe.
- ⌥Tab / ⌥⇧Tab : commande d’onglet suivante/précédente (Ctrl+Tab dans l’app).
- ⌥` : fenêtre suivante de la même app ; remplacement ⌘Tab volontaire dans Général.
- Gestes latéraux un doigt : cibles et seuils configurables, réarmement sans lever,
  préférence conservée et reprise bornée après veille/relance.
- Espaces nommés de deux fenêtres : capture, édition, proportions, essai,
  raccourcis, applications absentes ouvertes avant placement et restauration.
- Profil Teams opt-in : chat gauche/réunion droite, ratio modifiable, retour à100 %
  ou position initiale ; repères à vérifier sur la version de Teams utilisée.
- Filtre de défilement horizontal souris, permissions et diagnostics.
- Général : scénario de démarrage et lancement à la connexion.

Les nouvelles combinaisons sont ajoutées une seule fois si libres, sans remplacer
les choix existants. Toutes les affectations se modifient dans Raccourcis. Une
combinaison système refusée par macOS est signalée ; aucune activation fictive.

## Installer et utiliser

Décompresser le paquet universel, déplacer `Kigi Windows.app` dans Applications,
quitter l’ancienne copie et ouvrir celle installée. Studio s’affiche au lancement
manuel. Accorder Accessibilité pour les fenêtres, Surveillance de l’entrée pour
le trackpad si nécessaire, et Automatisation Chrome uniquement pour ChatGPT.
Les boutons demandent la rubrique correspondante dans Réglages Système.

Activer les gestes et leurs cibles dans Trackpad. Pour Teams, ouvrir ses deux
fenêtres, les identifier dans Réunions Teams, tester les repères puis enregistrer.
La taille minimale imposée par Teams peut empêcher50/50 ; les tailles réelles et
l’éventuel chevauchement sont affichés. Aucun pop-out n’est créé automatiquement.

Les builds actuels sont signés ad hoc, sans Developer ID ni notarisation. Une
mise à jour peut demander de réautoriser la nouvelle copie. Ne pas désinstaller
Hammerspoon ni supprimer sa configuration tant que la parité personnelle n’est
pas confirmée sur le Mac de test.

## Développement et validation

Un crate, dépendances réduites, APIs publiques AppKit/Accessibility/NSWorkspace,
WKWebView locale pour Studio, aucun serveur HTTP. Sur Mac :

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
bash scripts/bundle.sh universal
```

Intel+ARM construits ; contrôles natifs sur macOS15.7.9 Intel. macOS11 est une
cible de compilation, pas une compatibilité physique certifiée. Le transport
trackpad, Teams et les onglets réels restent à valider sur le Mac pilote. Les
mesures du moteur Rust ne représentent pas toute la mémoire WebKit.

Contrat de parité et limites : [docs/personal-parity.md](docs/personal-parity.md).
Historique : [docs/progress.md](docs/progress.md).
