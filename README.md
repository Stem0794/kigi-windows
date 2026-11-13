# Kigi Windows — Studio 0.6.4 (aperçu)

Application macOS native en Rust, un seul crate et un seul processus. Binaires
Intel + Apple Silicon ; cible macOS 11 non encore validée sur matériel ancien.

## Studio : éditeur d’espaces

Depuis le menu Kigi, choisir **Ouvrir Studio…**. Interface redimensionnable :
Accueil, Espaces, Raccourcis, Trackpad, Souris, Autorisations et Diagnostics avec
données réelles. Dans Espaces :
créer ou sélectionner un espace, choisir ses apps, glisser le séparateur ou saisir
les proportions1–99 %, puis **Enregistrer**. Presets50/50,35/65,70/30,30/70 ;
flèches pour ajuster au clavier. L’aperçu et la sauvegarde ne déplacent pas les fenêtres.
Les dispositions libres restent intactes avant conversion explicite en deux colonnes.
Duplication/suppression et dialogues de brouillon sont pris en charge. Un fichier
modifié ailleurs bloque l’écriture : conserver le brouillon ou Actualiser.
**Appliquer** place les deux fenêtres ouvertes, avec sélection native en cas d’ambiguïté.
Un brouillon modifié doit être enregistré explicitement avant placement. Les largeurs
réellement constatées s’affichent ; si une app impose une taille, le placement reste
partiel et la proportion demandée est conservée. **Restaurer** revient aux positions
précédentes, en préservant les fenêtres déplacées manuellement ; un retour incertain
reste disponible. Un seul retour est conservé, partagé avec les dispositions classiques.
Ouvrez les deux fenêtres avant d’appliquer ; aucune fenêtre de remplacement n’est créée.
Studio propose maintenant Raccourcis, Trackpad, Souris, Autorisations et Diagnostics.
Capture des combinaisons dans une fenêtre native, édition d’app/action, activation,
conflits et relecture ; les fichiers invalides ne sont pas écrasés. Une capture
réussie ferme sa fenêtre et affiche la combinaison choisie, à enregistrer explicitement.
Actions ajoutées : Travail, Réunion, Restaurer et Appliquer un espace enregistré.
Un raccourci d’espace suit son identifiant stable après renommage ; supprimer
l’espace laisse une référence signalée manquante, sans appliquer un autre espace. Les essais du
trackpad affichent les vrais compteurs, sans prétendre valider le global depuis un
test local. Les options souris distinguent préférence et filtre actif. Copie/export
utilisent le rapport du moteur. Les réglages classiques restent le secours et le défaut
de lancement pendant cette validation ; `--studio-ui` permet de choisir Studio.

Options de lancement : `--studio-ui` ouvre Studio, `--native-ui` force l’ancienne
fenêtre. Fermer Studio conserve les fonctions de fond ; Dock/menu le réouvre.
Les ressources sont locales et hors ligne. Mesures mémoire : processus Rust,
auxiliaires WebKit non mesurés. Aperçu livré sur branche UI, avant fusion.

## Installer et utiliser

Quitter l'ancienne copie, remplacer `Kigi Windows.app` au même emplacement,
puis relancer. La fenêtre Kigi s’ouvre au lancement et reste accessible dans le Dock.
Cinq onglets regroupent Espaces, Raccourcis, Gestes et souris, Autorisations et
Diagnostics. Fermer la fenêtre conserve les fonctions de fond ; ⌘Q quitte Kigi.
Le menu Kigi conserve seulement l’ouverture, les espaces et les dispositions rapides.
Cette version utilise une
signature ad hoc, sans Developer ID ni notarisation ; les autorisations macOS
peuvent devoir être réaccordées après remplacement.

| Raccourci | Action |
| --- | --- |
| ⌥T | Activer Teams ou lancer son bundle principal installé |
| ⌥B | Activer/lancer Chrome et maximiser sa fenêtre principale |
| ⌥C | Retrouver un onglet ChatGPT existant dans Chrome, sinon en ouvrir un |
| ⌘⇧↑ | Maximiser la fenêtre active dans la zone visible de son écran |
| ⌘⇧← | Choisir gauche, puis droite : deux fenêtres à 50 % |
| ⌘⇧→ | Même partage en choisissant d'abord la fenêtre droite |
| ⌃⌥T | Test conservé : moitié droite puis restauration de la même fenêtre |

Ces combinaisons sont les valeurs initiales. Dans **Raccourcis** :

1. Sélectionner une entrée ou cliquer sur **Ajouter** (nouvelle entrée désactivée).
2. Cliquer sur **Modifier la combinaison…**, puis presser la combinaison voulue.
   Utiliser ⌘, ⌥ ou ⌃, avec ⇧ si souhaité ; Échap annule. La saisie se limite
   à ce champ, sans écoute clavier globale. Les raccourcis Kigi sont suspendus
   pendant cette saisie, puis rétablis ; délai d’annulation de 15 secondes.
3. Choisir l’action. **Ouvrir / activer une application** permet de choisir un
   `.app` installé et, en option, de maximiser après activation. ⌥T peut donc
   ouvrir Safari, par exemple, ou Teams peut recevoir une autre combinaison.
4. Cocher **Activer ce raccourci**, puis **Enregistrer**. Les doublons actifs
   sont refusés ; un conflit Carbon conserve les réglages précédents.

Jusqu’à 32 entrées, suppression, activation individuelle et rétablissement
confirmé des valeurs initiales. Toutes les entrées et leur activation globale
sont mémorisées. Le fichier `shortcuts.json` est séparé du catalogue d’espaces ;
une erreur ou une modification externe bloque son remplacement. Les touches
restent physiques après choix, avec un libellé issu du clavier au moment de la
saisie. Les touches média/Fn seules et les raccourcis sans modificateur ne sont
pas proposés. Certains raccourcis réservés par macOS peuvent être interceptés
avant Kigi : les essayer après enregistrement.

En cas de conflit, quitter temporairement Hammerspoon puis **Réenregistrer**.
Son fichier n’est jamais modifié. Les gestes restent liés à Teams/Chrome,
indépendamment des raccourcis personnalisés.

## Passage au premier plan — 0.5.4

Correction du refus AppKit depuis l’arrière-plan observé sur le Mac pilote :
un retour false laisse poursuivre l’activation Accessibility autorisée, avec
AXFrontmost si disponible puis AXMain/AXRaise et confirmation du premier plan.
Les logs 0.5.3 montrent que les gestes continuaient à être reçus et reconnus ;
les seuils et le réarmement du trackpad restent inchangés.

Les raccourcis d’apps et les gestes relèvent la fenêtre principale/focalisée
existante via AX, puis confirment le premier plan. Une demande d’activation
acceptée n’est plus annoncée comme un succès avant cette confirmation.
L’attente tolère une transition depuis une fenêtre maximisée ou un Space
plein écran, pendant 4 s maximum, et abandonne si une tierce app est choisie.
Aucune fenêtre de secours n’est créée. Sans Accessibilité, l’activation simple
reste possible, mais le relevé AX n’est pas déclaré vérifié.

Le geste garde sa bande de départ de 10 % et nécessite seulement un déplacement
vers l’intérieur d’au moins 10 % de la largeur depuis le point de départ, sans
arrivée au centre imposée. Le déplacement doit être principalement horizontal.
Les micro-mouvements et les déplacements surtout verticaux ne déclenchent pas
d’action. Une action par contact ; lever le doigt réarme le geste.

## Fiabilité P0/P1 — 0.5.2

La maximisation directe et celle demandée après activation d’une app peuvent
sortir du plein écran natif si l’application expose son bouton Accessibility.
Kigi presse ce bouton une seule fois, attend la sortie et un rectangle stable,
puis maximise géométriquement. Attente de transition bornée à 4 s, annulée si
vous changez d’app. Aucun raccourci clavier synthétique, nouvelle fenêtre ou
écriture de l’attribut AXFullScreen. En cas de refus du bouton, sortir du plein
écran manuellement. Les sélecteurs 50/50 continuent de proposer des fenêtres
ordinaires ; les scénarios automatiques ne sont pas ajoutés.

**Diagnostics → Mémoire, latences et relance…** affiche la RSS actuelle, son
maximum depuis le lancement, et le p95 des 64 derniers succès par catégorie.
Les erreurs sont comptées séparément ; zéro succès indique « non mesuré ».
La géométrie AX exclut dialogues/lancement ; l’activation et la maximisation
incluent l’attente correspondante. Le rapport copiable contient ces valeurs
et le dernier état des permissions de la session précédente. Il ne mesure
ni le CPU au repos, ni la latence physique du trackpad.

Pour vérifier les permissions : remplacer l’app au même emplacement, autoriser
cette copie, quitter puis relancer sans nouveau remplacement. Comparer le bilan
de relance. La signature ad hoc n’assure pas la stabilité TCC entre builds ;
aucune permission n’est accordée ou modifiée par Kigi.

## Désactiver le scroll horizontal de la souris (0.3.2)

Dans **Gestes et souris**, cocher **Bloquer le défilement horizontal des souris**.
Option désactivée par défaut, mémorisée via les préférences natives macOS.
Elle agit dans toutes les applications pendant que Kigi tourne. Décocher
l'option ou quitter Kigi rétablit immédiatement le défilement normal.

Le filtre annule les trois représentations publiques de l'axe horizontal des
événements reconnus comme souris ; il conserve les axes vertical/profondeur,
les modificateurs et les limites des séquences. Un événement exclusivement
horizontal sans rôle de début/fin est supprimé, pour éviter les callbacks
parasites dans les jeux. Il ne réémet aucun événement synthétique.

La distinction souris/trackpad utilise les phases publiques : MayBegin marque
la préparation d'un scroll de trackpad, contrairement à la Magic Mouse.
Les événements du trackpad et leur inertie sont laissés passer. Les séquences
incomplètes ou incertaines sont aussi laissées passer. Un doigt posé longtemps
sur le trackpad ne fait pas expirer son marqueur préalable. Aucun périphérique
n'est identifié par marque/numéro de série : d'autres souris compatibles sont
également concernées. Les outils qui réécrivent les phases ou l'utilisation
simultanée de plusieurs dispositifs peuvent rendre la classification ambiguë.
Cela reste une option pilote à vérifier avec la Magic Mouse et le trackpad réels.

Un tap modifiable, limité aux événements ScrollWheel, est créé uniquement quand
l'option est demandée. Accessibilité est nécessaire. Il est indépendant du tap
passif des gestes à 10 % ; aucun clavier/clic/mouvement de pointeur n'est surveillé.
En cas d'interruption par macOS, les événements passent et l’onglet Gestes et souris indique le
filtre inactif ; désactiver/réactiver l'option pour le relancer. Après veille/
retour de session, un seul essai de reprise est fait à chaque notification
pertinente si la préférence était active, sans nouvelle demande d'autorisation
ni polling au repos. Une permission absente laisse le filtre inactif.

Les logs exportables ajoutent uniquement des compteurs : souris, trackpad,
inconnus, horizontaux filtrés et événements supprimés. Aucune valeur de
mouvement, frappe, cible d'application ou identifiant de souris n'est enregistré.

Essai conseillé : activer, tester horizontal/vertical/diagonal dans Minecraft
et une page défilante, puis vérifier le scroll horizontal à deux doigts et son
inertie sur le trackpad. Désactiver pour confirmer le retour immédiat, puis
relancer Kigi pour vérifier la préférence. Le test natif de géométrie CG ne
remplace pas cet essai matériel.

Sources API : [NSEvent.phase](https://developer.apple.com/documentation/appkit/nsevent/phase-swift.property),
[NSEvent.momentumPhase](https://developer.apple.com/documentation/appkit/nsevent/momentumphase),
[Apple Event Handling Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/HandlingTouchEvents/HandlingTouchEvents.html).

## ChatGPT : onglet existant

Au premier ⌥C, accepter **Automatisation → Google Chrome** si macOS le demande.
C'est distinct de l'Accessibilité. Un script fixe NSAppleScript s'exécute dans
Kigi, sans processus osascript : il compare uniquement les URL des onglets aux
hôtes exacts `chatgpt.com` et `chat.openai.com`, active le premier trouvé dans
les fenêtres Chrome, ou crée un onglet après une recherche sans correspondance.
Les onglets existants ne sont ni rechargés ni remplacés.

Si la recherche échoue, l'app affiche l'erreur et n'ouvre aucun onglet de secours.
Une permission refusée se réactive dans Réglages Système → Confidentialité et
sécurité → Automatisation. Aucun accès au contenu des pages, historique ou
stockage d'URL. Les logs indiquent seulement `reused`, `created` ou code d'erreur.
La demande de consentement peut attendre l'utilisateur ; le timeout AppleScript
3 s s'applique aux requêtes individuelles, pas à une durée totale de dialogue.

## Gestes de bord (P2 pilote)

Dans **Gestes et souris**, cocher **Tester les gestes globaux de bord**. Désactivés au
lancement et après redémarrage, ils nécessitent Accessibilité et, si macOS la
demande, Surveillance de l'entrée.

| Départ et mouvement, un seul doigt | Action |
| --- | --- |
| 10 % les plus à gauche, court glissement horizontal vers l’intérieur | Teams au premier plan |
| 10 % les plus à droite, court glissement horizontal vers l’intérieur | Chrome au premier plan à 100 % |

Le retour utilisateur 0.4.1 signale une absence de détection dans les logs.
La réception globale n’est donc pas considérée corrigée. Cette version affiche
les compteurs en direct, journalise la première réception et arrête le test
après 12 s sans geste reconnu avec `trackpad.trial.complete`. Le test local vérifie le
même geste horizontal à 10 %, sans lancer d’application. Les diagnostics locaux
et globaux n’utilisent plus l’ancien geste vertical de coin. Une création de
tap réussie n’est jamais présentée comme une preuve de réception de contacts.

Le départ peut se faire sur **toute la hauteur** : x≤0,10 à gauche ou x≥0,90
à droite. Le doigt doit avancer d’au moins 10 % de la largeur depuis son point de départ
vers l’intérieur en ≤1,5 s, sans devoir atteindre le milieu, avec
une dérive verticale ≤0,12 ; tolérance de recul vers l'extérieur de 0,03.
Une action par contact ; aucun délai imposé après avoir levé le doigt. Deux doigts, clic/drag observé,
identité/device différent, mauvaise direction, délai ou début manquant annulent.
Une séquence annulée doit se terminer par retrait de tous les doigts avant de
réarmer. Une commande manuelle annule le geste encore en attente. Aucune action
pendant les sélecteurs, l'intégration Chrome ou le menu Kigi ouvert.

L'écoute est passive : pas de déplacement du pointeur ni de consommation des
gestes macOS. L'action est différée après le callback du tap ; aucun AX ou Apple
Events dans ce callback. Un seul tap existe, soit en diagnostic, soit en actions.
L’onglet Gestes et souris permet de désactiver ; ouvrir le diagnostic global pendant l'écoute
active l'arrête et affiche son bilan. Recliquer démarre le diagnostic sans action.

Veille du Mac/écran, changement de session et interruption du tap arrêtent
l'écoute et les actions en attente. **Réactiver manuellement après reprise**,
sans boucle de relance automatique. Un retrait/rebranchement de périphérique
invalide le contact ; lever les doigts puis recommencer. Le verrouillage sans
veille et les faux positifs restent à vérifier sur le Mac pilote : aucune
promesse de fiabilité universelle. Pas d'API privée ou de lecture de TCC.

Le diagnostic local reste un coin haut-gauche x≤0,10 / y≥0,80, sans action.
Le diagnostic global conserve ce même coin ; ce n'est pas le profil de bord.

## Espaces de travail manuels (P3 pilote)

Trois commandes dans le menu Kigi :

- **Espace Travail** : active Chrome et le maximise à 100 % ; Teams reste ouvert derrière.
- **Espace Réunion** : place une fenêtre standard Teams déjà ouverte à gauche et
  une fenêtre Chrome déjà ouverte à droite, moitié/moitié, sur l'écran de Chrome.
- **Restaurer la disposition** : remet les deux fenêtres à leurs positions
  précédentes, tant qu'elles n'ont pas été déplacées manuellement depuis
  l'application de la scène.

Les deux applications et les fenêtres voulues doivent déjà être ouvertes.
Utilisez ⌥T et ⌥B au besoin. Une seule disposition de retour est gardée en
mémoire ; un deuxième clic sur Réunion ne remplace pas ce snapshot. Travail
annule la possibilité de restaurer la scène Réunion précédente une fois Chrome
maximisé. En cas de placement partiel, la commande Restaurer reste disponible.

Il n'y a **pas encore** de détection de réunion automatique, de sélection
fiable d'une fenêtre Teams « réunion » plutôt que « chat », de détection automatique fiable. Pas de déplacement de fenêtres entre Spaces
Mission Control. Les gestes P2 déclenchent toujours seulement Teams/Chrome.

## Gérer les espaces de travail (P4 pilote 0.4.0)

Dans la fenêtre principale, choisir **Espaces**. Cet onglet permet de
créer plusieurs espaces nommés sans éditer le JSON :

1. Choisir **Nouvel espace…** dans la liste, saisir un nom, puis cliquer sur
   **Enregistrer deux fenêtres…** et choisir les deux fenêtres déjà ouvertes.
2. Sélectionner ensuite un espace enregistré. **Recapturer les deux fenêtres…**
   remplace ses applications/positions par une nouvelle sélection. Annuler
   l'un des choix ne modifie ni la configuration ni les fenêtres.
3. **Renommer**, **Dupliquer** et **Supprimer** gèrent les espaces ; la suppression
   demande confirmation et ne ferme/déplace aucune application.
4. Les menus de position proposent **Position enregistrée**, **Moitié gauche**,
   **Moitié droite** et **100 % de l'écran** pour chaque fenêtre.
5. **Enregistrer les positions** conserve les modifications sans déplacer les
   fenêtres. **Appliquer** enregistre aussi les modifications du formulaire puis
   place les fenêtres. Le menu **Kigi → Espaces enregistrés** applique directement
   les versions enregistrées, sans dépendre d'un formulaire non sauvegardé.

Les espaces sont conservés après redémarrage, avec une limite de 32 espaces et
60 caractères par nom. Deux fenêtres sur un même écran par espace à cette étape.
Les applications/fenêtres doivent déjà être ouvertes. En cas d'ambiguïté, Kigi
redemande laquelle utiliser ; les titres ne sont utilisés que dans le sélecteur.
Depuis la fenêtre de réglages, l'écran et le focus de retour prennent pour
contexte l'application active avant son ouverture (écran principal en repli).

La disposition personnelle créée dans 0.3.1/0.3.2 est reprise sous le nom
**Disposition personnelle**. La lecture convertit le schéma 1 en mémoire ; la
première modification enregistre le schéma 2 avec sauvegarde du fichier initial.
Fermer/ouvrir les réglages ne réécrit pas le fichier. Réappliquer un formulaire
inchangé ne remplace pas sa sauvegarde par une copie identique.

`~/Library/Application Support/io.kigi.windows/config.json` contient les noms
choisis, les identifiants stables des espaces, les bundle IDs et rectangles
normalisés. Aucune persistance de titre de fenêtre, PID, pointeur AX ou URL.
Écritures atomiques, sauvegarde `config.json.bak`, validation stricte (64 Ko,
géométrie et IDs/noms uniques). Une sauvegarde valide peut être utilisée si le
fichier principal est endommagé ; celui-ci reste conservé et n'est pas écrasé.
Le menu Finder reste disponible pour les cas de récupération avancés.

Un seul retour temporaire est partagé entre Réunion et les espaces enregistrés.
Utiliser **Restaurer la disposition précédente** ou **Espace Travail** avant
une autre application. Le retour respecte les déplacements manuels, conserve
les erreurs partielles et reste en mémoire seulement. Modifier/renommer un
espace ne change pas le retour déjà capturé. Aucune fenêtre n'est recréée, aucun
onglet n'est ouvert par les espaces.

Les raccourcis personnels, gestes horizontaux à 10 % et filtre souris sont
conservés. Les gestes gardent leurs actions Teams/Chrome. Cette première UI P4
n’inclut pas encore le glisser-déposer des rectangles, ni l’affectation de
raccourcis ou de gestes aux espaces ; ces sous-étapes restent distinctes.
La nouvelle fenêtre native et le cycle création/édition/application restent
à essayer sur le Mac pilote.

## Permissions et limites fenêtres

L’onglet Autorisations est actualisé à l’activation de Kigi. La première demande
est explicite et unique par session, suivie d'un contrôle temporaire borné.
Une case d'une ancienne version peut ne pas autoriser le processus actif :
**Afficher la copie active dans Finder** permet de choisir le bon bundle.
L'erreur AX est expliquée une fois par session pour toutes les actions de
placement. Ouvrir Teams ou Chrome ne valide pas l'autorisation de placement.

Les sélecteurs utilisent les titres en mémoire pour choisir, jamais dans les
logs. Deux fenêtres distinctes sont nécessaires, standard, non minimisées et
hors fullscreen ; Hammerspoon et Kigi sont exclus. Un inventaire borné peut
être partiel : c'est indiqué. Annuler ne déplace rien ; le second choix reçoit
le focus. L'écran est celui de la fenêtre active avant le sélecteur.

Chrome peut être déminimisé pour ⌥B. La maximisation peut demander la sortie du
plein écran natif via son bouton AX, avec attente bornée ; les sélecteurs 50/50
continuent de proposer uniquement des fenêtres ordinaires.
Les apps peuvent imposer leurs dimensions : AX relit la géométrie et signale un
résultat partiel. Position/taille/position reprend le correctif 0.0.6 validé par
l'utilisateur. Le snapshot de ⌃⌥T reste en mémoire : restaurer avant de quitter.
Les lancements NSWorkspace sont asynchrones, avec contrôle temporaire ≤4 s ;
une nouvelle commande annule le traitement en attente, pas le lancement OS.

## Logs et essai court

Reproduire un problème, puis **Copier les logs de diagnostic** ou **Exporter les
logs…**, avant de quitter. Journal limité aux 200 derniers événements de session,
avec version/OS/architecture, permissions, conflits, actions et leurs résultats,
rectangles AX, compteurs et interruptions du tap. Pas de frappes, titres, contenu
d'apps, URL d'onglets, identité des contacts ou coordonnées de doigts. Aucun envoi
réseau, aucun journal persistant automatique.

Sur le Mac pilote : ⌥C deux fois avec ChatGPT déjà ouvert (nombre d'onglets
inchangé), avec ChatGPT absent (un seul nouvel onglet), puis un geste horizontal volontaire
de chaque bord vers l’intérieur. Essayer aussi un départ central, deux doigts, un
glissement vertical depuis un bord et l'usage ordinaire
pour relever les faux positifs ; désactiver si nécessaire. Après veille,
confirmer l'arrêt puis réactiver volontairement. Les nouveaux comportements
restent à valider sur matériel ; p95/RSS n'ont pas été mesurés ici.

## Construire et vérifier

```sh
cargo fmt --check
cargo check --locked
cargo test --locked
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=11.0 cargo check --locked --target aarch64-apple-darwin
# Sur macOS avec Rust stable et outils Xcode :
bash scripts/bundle.sh universal
open "dist/Kigi Windows.app"
```

Le workflow macOS lie les deux architectures, signe ad hoc et crée le ZIP.
Il compile aussi le script fixe si Chrome est installé, sans l'exécuter ni
demander Automatisation. Les checks Linux/macOS ne remplacent pas les essais
Chrome, permissions et gestes sur un Mac. Voir [progress.md](docs/progress.md),
[trackpad-spike.md](docs/trackpad-spike.md) et [migration-audit.md](docs/migration-audit.md).
Pas d'éditeur, scènes persistantes, détection de réunion ou système commercial.
