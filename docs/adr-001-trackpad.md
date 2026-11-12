# ADR 001 — Écoute publique et diagnostic du trackpad

2026-11-12 — P0 uniquement.

Le handover propose `CGEventTap → NSEvent → allTouches`. Les bindings vérifiés
exposent effectivement `NSEvent::eventWithCGEvent`, `allTouches`,
`NSTouch::normalizedPosition`, `identity`, `device` et `phase`. Le code
Hammerspoon utilise cette conversion pour `NSEventTypeGesture`.

La documentation Apple décrit toutefois les contacts comme attachés à une vue
sous le pointeur. Les moniteurs globaux AppKit ne garantissent pas les contacts.
`NSEventTypeGesture` vaut 29 dans AppKit ; ce n'est pas un type documenté dans
`CGEventType`. Le fait de passer son masque à un tap CG public reste une
hypothèse de livraison entre couches, pas un contrat de support macOS.

Décision :

- Un diagnostic global **opt-in** utilise seulement un tap session **listen-only**
  avec ce masque AppKit. Il convertit uniquement les événements Gesture et
  compte contacts/candidats. Aucun événement clavier, aucun appel AX dans le
  callback, aucune action de fenêtre. Un tap interrompu nécessite un redémarrage
  explicite de l'essai. Pas de réactivation automatique.
- Un diagnostic local `NSView` reçoit les contacts indirects par les méthodes
  publiques `touchesBegan/Moved/Ended/CancelledWithEvent:`. Il permet de vérifier
  la convention des coordonnées et le geste, sans prétendre à un geste global.
- Aucun framework privé, sélecteur privé, extraction IOHID ou événement numérique
  privé n'est employé. Le masque inter-couches ci-dessus reste expérimental.
- Si le tap ne reçoit pas de contacts complets, conserver le raccourci comme
  déclencheur utilisable. Un coin d'écran fondé sur le curseur serait une autre
  fonction, pas l'équivalent d'un coin physique du trackpad ; non implémenté P0.

Le diagnostic n'est pas un moteur P2 : pas d'assignation de scènes, calibration
persistante, gestion complète du cooldown/devices ou garantie après veille.

Sources consultées :

- [Apple : Handling Trackpad Events](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/HandlingTouchEvents/HandlingTouchEvents.html)
- [Apple : NSTouch](https://developer.apple.com/documentation/appkit/nstouch)
- [Apple : global event monitor](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents%28matching%3Ahandler%3A%29)
- [Bindings NSEvent 0.3.2](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSEvent.html)
- [Code Hammerspoon inspecté, non copié](https://github.com/Hammerspoon/hammerspoon/blob/master/extensions/eventtap/libeventtap_event.m)
