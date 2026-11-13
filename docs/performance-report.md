# Mesures P0/P1

Version 0.5.2. Les mesures portent sur le processus Kigi de la session courante.

- RSS actuelle : libproc `proc_pidinfo(PROC_PIDTASKINFO)`, octets résidents du
  propre PID ; pic RSS : `getrusage(RUSAGE_SELF).ru_maxrss`, octets sur Darwin.
- Géométrie : permission, préflight, position/taille/position et relecture AX.
- Maximisation : sélection/préparation, transition éventuelle et placement.
- Activation : lancement/focus et placement si demandé, jusqu’au résultat.
- p95 par rang supérieur, sur les 64 derniers succès de chaque catégorie ;
  erreurs à part. Les annulations par nouvelle commande ne sont pas des succès.
- Les dialogues de choix 50/50 ne sont pas inclus dans la mesure géométrie.

Aucune action correspondante réussie : **non mesuré**, pas zéro milliseconde.
Le p95 sur quelques actions n’est pas une garantie de performance. Le CPU au
repos et la latence physique du geste ne sont pas mesurés par ce rapport.

Le workflow relève deux démarrages du même binaire Release sur macOS Intel et
leurs RSS. Il n’effectue aucune action Accessibility sur une application tierce.
Les chiffres du runner ne sont pas ceux du Mac utilisateur ni d’un ancien Mac.

Sur le Mac pilote : effectuer quelques activations/maximisations ordinaires,
une sortie de plein écran, puis copier le rapport dans Diagnostics. Noter le
modèle, macOS, succès/échecs et permissions. Pas de campagne répétitive requise.

Sources des unités et APIs :
https://github.com/apple/darwin-xnu/blob/main/bsd/man/man2/getrusage.2
https://developer.apple.com/documentation/appkit/nsapplication/currentsystempresentationoptions
https://developer.apple.com/documentation/applicationservices/kaxfullscreenbuttonattribute

## Relevé du runner 0.5.2

[Run 43327603804](https://github.com/Stem0794/kigi-windows/actions/runs/43327603804),
source ddde02d4fcbc81b4c77295b2cf12e91764cf538f, Release x86_64 sous macOS
15.7.9. Deux relevés environ deux secondes après lancement :

| Relevé | RSS actuelle | Pic RSS | Fenêtre visible | AX / surveillance |
| --- | ---: | ---: | --- | --- |
| Premier lancement | 38 178 816 octets | 38 178 816 octets | Oui | true / true |
| Deuxième lancement | 38 084 608 octets | 38 084 608 octets | Oui | true / true |

La deuxième session reconnaît la précédente. Aucune collision de raccourcis
signalée. Les drapeaux de précontrôle restent autorisés, sans nouvelle demande
de permission. Cela ne remplace pas un essai de contrôle AX d’app tierce.
Aucune activation/maximisation de tiers n’ayant été demandée, leurs p95 restent
**non mesurés**. Pas de conclusion de performance ARM ni de CPU au repos.
La comparaison de permissions conserve le relevé au démarrage ; le champ AX
général du rapport diagnostique indique séparément l’état courant.
