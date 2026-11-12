# Spike trackpad — P0

2026-11-12. **Verdict : PARTIAL documentaire et prototype ; aucun GO matériel.**
L'environnement disponible est Linux x86_64. Aucun événement réel du trackpad,
aucune permission TCC et aucune fonction native n'ont été testés sur Mac.
La compilation de types macOS ne prouve pas leur fonctionnement.

## Ce que le prototype permet de mesurer

Le diagnostic local affiche x/y normalisés, phase et nombre de candidats. Le
diagnostic global affiche les nombres d'événements Gesture, d'événements avec
contacts, d'échantillons et de candidats, ainsi que l'interruption éventuelle du
tap. Tout reste en mémoire, sans titres, frappes, logs de mouvements ni disque.
Aucun candidat ne déclenche de déplacement ou de scène.

Départ dans le coin x∈[0 ; 0,20], y∈[0,80 ; 1] ; descente `y_initial - y ≥ 0,22`,
dérive horizontale ≤0,12, durée ≤1 500 ms, un seul doigt, début requis, une seule
détection par début. Deuxième doigt, annulation, identité/device différent,
dérive ou délai excessif annulent le candidat. Les seuils sont expérimentaux.

| Essai | État actuel |
|---|---|
| Vérification des signatures publiques objc2/AppKit | Faite sur les sources des crates résolues |
| Typage cible Intel et Apple Silicon, déploiement 11.0 | Vérifié, sans édition de liens Apple |
| Contacts dans une NSView locale sur Mac | Non exécuté |
| Contacts globaux pendant Chrome/Teams au premier plan | Non exécuté ; pas de garantie API |
| Avec/sans AX et Surveillance de l'entrée | Non exécuté |
| Trackpad intégré / Magic Trackpad, Intel / ARM | Non exécuté |
| Verrouillage, veille, reconnexion et coût CPU | Non exécuté |

## Un essai représentatif à faire sur Mac

1. Construire/lancer le bundle selon README. Noter modèle, architecture, macOS,
   type de trackpad et permissions réellement accordées.
2. Ouvrir le diagnostic local. Avec le pointeur dedans, lever les doigts puis
   essayer le geste. Vérifier que y diminue vers le bas. Essayer un départ au
   centre et deux doigts : le compteur ne doit pas augmenter.
3. Démarrer le diagnostic global via le menu. Tester sans accès, puis après
   autorisation séparée de Surveillance de l'entrée. Relever aussi l'état AX.
   Relancer l'app si macOS l'exige, puis recommencer l'écoute volontairement.
4. Passer dans Chrome/Teams. Essayer un glissement à un doigt, du scroll à deux
   doigts et les gestes système habituels. Arrêter l'écoute par le même menu
   pour relever les compteurs. Si aucun contact n'arrive : ne pas activer de
   geste global, garder le raccourci. Un compteur non nul seul ne suffit pas :
   vérifier détection voulue et absence de déclenchement sur gestes ordinaires.
5. Refaire un geste après verrouillage/déverrouillage et veille. Si le tap est
   interrompu, le rapport le signale ; arrêter/redémarrer volontairement.

Ajouter les observations ici avant de choisir GO / PARTIAL / NO-GO matériel.
Ne pas transformer l'absence de Mac de test en verdict d'impossibilité.
Voir [ADR 001](adr-001-trackpad.md) pour la limite de l'API et le repli clavier.
