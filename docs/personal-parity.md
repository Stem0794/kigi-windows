# Parité du workflow personnel — 0.10.0

Référence : `examples/hammerspoon-reference.lua`, `docs/migration-audit.md` et
les demandes ultérieures de raccourcis modifiables, gestes horizontaux à10 %,
réarmement sans lever et interface Studio.

| Usage | Implémentation0.10.0 |
|---|---|
| Teams, Chrome100 %, ChatGPT existant | Actions conservées, intégration Chrome conditionnelle à l’autorisation Apple Events |
| Maximiser, sélectionner deux fenêtres gauche/droite | Actions conservées, sortie du plein écran par bouton AX public |
| Fenêtre active droite50 % | Nouvelle action et combinaison ⌘⇧↓ ajoutée si libre |
| Réappliquer chat/réunion Teams | Nouvelle action TeamsSplit, bouton Studio/menu et ⌃⌘⇧T ajouté si libre ; préserve le retour initial si la même paire est suivie |
| Fin de réunion | Retour du chat à100 % ou position initiale ; option de fin sur titre devenu non conforme pendant2s |
| Repères Teams stables | Option d’exclusion du chat et des pages ordinaires ; capture propose le suffixe après `|` quand il est identique dans les deux titres |
| Scénario au lancement | Option Général : réunion présente, sinon ouverture de Chrome100 % ; aucune scène forcée par défaut |
| Gestes latéraux un doigt | Paramètres persistants : bord3–25 %, trajet5–40 %, durée500–3000ms ; mêmes seuils pour action et diagnostic |
| Cibles des gestes | Combinaison enregistrée suivie dynamiquement, espace ou aucune action ; aucun index de liste instable |
| Reprise gestes | Préférence mémorisée, reprise au lancement/veille/interruption,3 créations automatiques maximum par reprise, sans prompt en arrière-plan ; plus d’arrêt après12s sans geste |
| Navigation fenêtres | ⌥` ajouté si libre pour la même app ; action toutes apps et remplacement volontaire ⌘Tab/⌘⇧Tab dans Général |
| Navigation onglets | ⌥Tab / ⌥⇧Tab ajoutés si libres ; transmission Ctrl+Tab / Ctrl+⇧Tab au PID actif, sans lecture de pages |
| Appliquer un espace avec apps fermées | Ouverture NSWorkspace des apps absentes, attente de fenêtres bornée8s et annulable ; révision revérifiée avant déplacement |
| Lancement connexion | Option créant un seul fichier RunAtLoad utilisateur géré par Kigi ; même app, mode --login sans Studio initial ; aucun KeepAlive/helper |

## Sémantique et limites

- Les combinaisons occupées, y compris désactivées, ne sont jamais remplacées.
  Une migration unique sauvegarde le fichier antérieur puis ajoute les commandes manquantes ; les entrées restent
  modifiables/supprimables dans Studio. Les tests à configuration isolée ne
  modifient pas les raccourcis personnels du runner.
- Un geste suit une combinaison, pas une position dans la liste. Changer la
  commande de ⌥T est immédiatement suivi ; changer/supprimer la combinaison
  nécessite de sélectionner sa nouvelle cible. Un raccourci désactivé n’agit pas.
- Le remplacement ⌘Tab est désactivé par défaut et ne coexiste pas avec une
  affectation de ⌘Tab ou ⌘⇧Tab. Désactiver les raccourcis globaux libère aussi
  ce remplacement ; les réactiver le reprend si les autorisations le permettent.
  Il ne consomme que ces combinaisons et leur relâchement,
  sans enregistrer les autres touches. Le callback utilise une boucle d’événements
  dédiée pour que les lectures AX du moteur ne bloquent pas le clavier ; les
  commandes seules sont transmises au thread principal, regroupées dans une file
  bornée. La liste est figée pendant l’enchaînement,
  regroupe les applications récemment activées et ne fournit pas de vignettes
  d’écran. Les fenêtres inaccessibles restent hors liste. Ce n’est pas une
  promesse de reproduire l’interface graphique du sélecteur Windows.
- L’adapter onglets transmet la combinaison standard à l’app active. Il ne peut
  ni vérifier un onglet interne ni garantir le comportement d’une app qui utilise
  Ctrl+Tab pour autre chose. Aucun AppleScript libre ni moniteur de frappes.
- Teams reste limité à une paire native identifiée. Les titres changent selon
  la version/langue ; le suffixe commun proposé reste à vérifier. Une ambiguïté
  suspend l’automatisme. Le placement utilise l’écran de la réunion, comme le
  script initial, et peut y amener le chat d’un autre écran. Les contraintes de
  largeur de Teams restent affichées au lieu d’annoncer un50/50 fictif.
- Les espaces généraux restent des dispositions de deux fenêtres sur un écran.
  Le multiécran/multi-fenêtres arbitraire et les règles contextuelles générales
  du projet commercial ne faisaient pas partie du comportement Lua à reproduire.
  Une app déjà lancée sans fenêtre doit encore fournir elle-même une fenêtre ;
  Kigi ne crée pas de pop-out ou de document arbitraire.
- Au démarrage, un profil Teams non configuré/incertain empêche le scénario
  d’imposer Chrome devant une possible réunion. L’option agit après1,5s et est
  annulée par une commande personnelle explicite.
- Le lancement à la connexion suppose une app installée à un chemin stable ;
  App Translocation est refusé. Le fichier étranger ou un lien symbolique est
  conservé. Déplacer l’app ensuite nécessite de réactiver l’option.
- La reprise ne contourne aucune permission. Les contacts globaux restent soumis
  au transport public disponible sur le Mac pilote. Les mesures Rust n’incluent
  pas toute la mémoire WebKit : aucune comparaison de légèreté Hammerspoon n’est
  revendiquée sans mesure sur la même machine.

## Validation

Tests purs ciblés : suivi des raccourcis, migration additive, seuils de gestes,
filtre exclusivement ⌘Tab, profil Teams à suffixe partagé, fichier de connexion.
Fixtures natives : géométrie/restitution, réapplication Teams avec conservation
du retour, navigation même app/toutes apps et moitié droite, lancement de deux
apps de test absentes via NSWorkspace. Les tests de navigation n’injectent aucune
frappe globale. Studio vérifie les options, la révision et la relecture après relance.
La veille suspend le suivi Teams, puis les géométries sont revérifiées au réveil
avant de restaurer. Les gestes physiques, les onglets réels et une session utilisateur complète avec
veille/reconnexion restent à confirmer sur le Mac pilote.

Validation de livraison : 40 tests hôte, 43 tests natifs et tous les parcours
macOS/Studio réussis dans le [run 43923722491](https://github.com/Stem0794/kigi-windows/actions/runs/43923722491).
Source du binaire : eaf571047a66ec0cef1452cd6d7f4feb20ef4582 ; bundle 0.10.0/build 32.
Les architectures Intel et Apple Silicon sont liées ; l’exécution native contrôlée
porte sur macOS 15.7.9 Intel.
