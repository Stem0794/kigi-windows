# Profil de réunion Teams — pilote0.9.0

Le profil est désactivé par défaut. Dans **Réunions Teams**, ouvrir une réunion et
son chat dans deux fenêtres standard séparées, puis **Identifier les fenêtres**.
Choisir le chat, puis la réunion. Les titres sont affichés uniquement dans ce
parcours local ; aucun contenu de conversation n’est lu. Modifier les deux repères
pour ne garder que leur partie stable, sans nom de réunion ou de personne.
**Tester la détection** ne déplace rien. Activer puis enregistrer exige exactement
une fenêtre correspondant à chaque repère, toutes deux distinctes et normales.

Réglages : réunion à droite/chat à gauche par défaut, chat50 % (20–80 %), retour du
chat à100 % ou à sa position initiale. Ces préférences sont persistées localement
avec NSUserDefaults. Le bouton d’enregistrement conserve le brouillon en cas de
refus, et les révisions empêchent un formulaire périmé d’écraser les réglages.

La signature est fournie/vérifiée par l’utilisateur pour sa version et sa langue
de Teams. **Aucune version réelle de Teams n’est encore certifiée par ce pilote.**
Un titre dynamique sans fragment stable ne peut pas être classé automatiquement
avec cette version. Une fenêtre de pré-connexion portant le même titre qu’une
réunion sera reconnue : le déclencheur est la présence de la fenêtre identifiée,
pas la connexion audio. Refaire l’identification si Teams change ses titres.
Aucun pop-out n’est créé par Kigi.

## Exécution

Une interrogation par seconde, uniquement si le profil est activé et la session
non suspendue. Elle se limite aux fenêtres AX du processus Teams (maximum16,
budget d’inventaire750ms, délai AX250ms par appel). Deux fenêtres stables pendant
au moins1s déclenchent un seul placement avec lecture de retour. Elles doivent
être sur le même écran. Aucun changement de Space ou de plein écran ; les deux
fenêtres sont relevées uniquement si Teams est déjà au premier plan.

Le moteur garde les objets AX exacts, les positions initiales et appliquées et
l’écran. Un changement de titre de la réunion active ne suffit jamais à annoncer
la fin. Il faut une absence confirmée dans un inventaire complet de fenêtres pendant2s.
Une fenêtre encore minimisée ou en plein écran ne compte pas comme fermée.
L’invalidation AX est acceptée mais non exigée : Teams peut garder un objet
fenêtre vivant après sa fermeture. Un timeout/permission absente est « inconnu » : aucun
retour. La position du chat et l’écran sont revérifiés avant le retour.

Une disposition manuelle Kigi (maximisation, partage, scène, restauration), un déplacement
mesuré, une minimisation/plein écran, la veille, une fenêtre de réunion de
remplacement ou le bouton **Suspendre cette réunion** cède le contrôle à
l’utilisateur jusqu’à la fermeture de la fenêtre suivie. Une simple activation
d’application, notamment ⌥T sans maximisation, conserve le retour automatique.
Les dialogues et activations en cours différèrent la détection sans déplacer de
fenêtre. Désactiver ne restaure
rien. Modifier le profil pendant une réunion active ne la réapplique pas. Les
scènes manuelles et leur retour existant restent prioritaires. Aucun recentrage
ou nouvel essai répété après échec. Un placement partiel tente de rétablir les
positions encore reconnues, puis suspend le profil ; aucun succès fictif.

## Vérification

Tests purs : temporisations, interruption inconnue, réapparition, priorité manuelle,
profils invalides, plusieurs réunions/chats et fenêtre correspondant aux deux.
Vérification native dédiée : deux fenêtres AppKit contrôlées, placement réel50/50,
fermeture AX publique, retour à100 % après le délai logique ; deuxième passage
avec déplacement manuel et absence de retour forcé. Les temps logiques sont
avancés explicitement dans cette fixture ; la minuterie réelle reste à vérifier
sur le Mac pilote. Studio vérifie réglages, sauvegarde, révision périmée et refus
d’activation sans profil. Ces contrôles ne prouvent pas la signature Teams, les
contraintes de taille de Teams ni le comportement physique du Mac M5.
