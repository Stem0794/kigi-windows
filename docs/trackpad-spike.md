# Spike trackpad — P0

2026-11-12. **Verdict : PARTIAL avec validation utilisateur positive sur son Mac.**
Le développement a commencé sous Linux x86_64 ; les binaires sont maintenant
construits sur un runner macOS. Le premier retour utilisateur confirme un test
local réussi et des contacts globaux reçus sur son Mac (détails ci-dessous).
L’utilisateur a confirmé le diagnostic après réduction de zone et à nouveau en 0.0.5/0.0.6. Le comportement après veille reste à valider.

## Ce que le prototype permet de mesurer

Le diagnostic local affiche x/y normalisés, phase et nombre de candidats. Le
diagnostic global affiche les nombres d'événements Gesture, d'événements avec
contacts, d'échantillons et de candidats, ainsi que l'interruption éventuelle du
tap. Tout reste en mémoire, sans titres, frappes, logs de mouvements ni disque.
Aucun candidat ne déclenche de déplacement ou de scène.

Départ dans le coin x∈[0 ; 0,10], y∈[0,80 ; 1] ; descente `y_initial - y ≥ 0,22`,
dérive horizontale ≤0,12, durée ≤1 500 ms, un seul doigt, début requis, une seule
détection par début. Deuxième doigt, annulation, identité/device différent,
dérive ou délai excessif annulent le candidat. Les seuils sont expérimentaux.

| Essai | État actuel |
|---|---|
| Vérification des signatures publiques objc2/AppKit | Faite sur les sources des crates résolues |
| Typage cible Intel et Apple Silicon, déploiement 11.0 | Vérifié, sans édition de liens Apple |
| Contacts dans une NSView locale sur Mac | Réussite rapportée par l'utilisateur avec 0.0.1 |
| Contacts globaux sur le Mac utilisateur | 1 793 événements avec contacts et 3 candidats rapportés avec 0.0.1 ; app au premier plan non relevée |
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

## Retour utilisateur du 12 novembre — première validation terrain partielle

L'utilisateur rapporte un test local réussi. Sa capture du diagnostic global
affiche 2 069 événements, 1 793 avec contacts, 1 558 échantillons et 3 candidats ;
tap interrompu : false. Cela confirme la réception de contacts et de candidats
sur son Mac avec le premier prototype. Modèle, macOS, permissions exactes et
nombre de gestes tentés non relevés : pas de conclusion sur le taux de réussite,
les faux positifs, la veille ou la compatibilité Intel/macOS 11. Le verdict reste
PARTIAL, désormais avec une observation terrain positive.

En 0.0.2, la zone de départ passe de 20 % à 10 % de la largeur, toujours dans
les 20 % supérieurs. Le déplacement peut sortir de cette bande après le départ
dans la limite de dérive horizontale existante (0,12). Elle s'applique au
diagnostic local et global ; les confirmations suivantes figurent ci-dessous.

## Confirmation utilisateur avant P1

Après les packages 0.0.2 puis 0.0.5/0.0.6, l'utilisateur confirme « trackpad ok »
et « le scroll haut gauche fonctionne toujours ». La réduction à 10 % n'est
donc plus seulement une hypothèse de code. Aucun relevé de faux positifs,
veille/verrouillage, type exact de trackpad ou couverture d'autres OS n'est
fourni ; PARTIAL reste approprié avant activation d'actions dans P2. Le code
trackpad reste inchangé dans 0.1.0.

## P2 pilote 0.2.0 : action après activation explicite

Le diagnostic existant a reçu la confirmation utilisateur sur le Mac pilote ;
le profil actif de bord peut maintenant être essayé volontairement depuis le
menu. Il reprend toute hauteur gauche10 % / droite20 % et déclenche Teams ou
Chrome après sortie du callback. Le diagnostic local/global de coin est conservé.
L'écoute s'arrête après veille/session inactive/interruption et reste désactivée
à la reprise jusqu'à un clic utilisateur. Aucun verrouillage sans veille ou
branchement Magic Trackpad n'a encore été testé. Les tests purs de cooldown,
contact unique et annulation ne démontrent pas l'absence de faux positifs réels.
Verdict terrain reste PARTIAL ; ne pas présenter 0.2.0 comme support universel.
