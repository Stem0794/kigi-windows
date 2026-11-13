'use strict';
// Configuration views only. All registration, permissions, reports and input stay native.
window.KigiSystem = (() => {
  let ctx, draft = null, original = null, selected = 0, revision = null, filter = '', scope = 'all', timer, until = 0, generation = 0;
  const copy = value => value == null ? null : JSON.parse(JSON.stringify(value));
  const isDirty = () => draft != null && (original == null || JSON.stringify(draft) !== JSON.stringify(original));
  const settings = () => ctx.data().shortcuts;
  const combo = binding => [[4096, '⌃'], [2048, '⌥'], [512, '⇧'], [256, '⌘']].filter(([flag]) => binding.modifiers & flag).map(([, glyph]) => glyph).join('') + binding.key_label;
  const actionName = command => ({Application: command.name || 'Application', ChatGpt: 'ChatGPT dans Chrome', Maximize: 'Maximiser', PairLeft: 'Gauche puis droite', PairRight: 'Droite puis gauche', Test: 'Test / restauration de fenêtre', Work: 'Travail — Chrome 100 %', Meeting: 'Réunion — Teams / Chrome', Restore: 'Restaurer la disposition', ApplyProfile: ctx?.data().spaces.profiles.find(p => p.id === command.profile_id)?.name ? `Espace : ${ctx.data().spaces.profiles.find(p => p.id === command.profile_id).name}` : `Espace supprimé #${command.profile_id}`})[command.kind];
  const rowState = binding => !binding.enabled ? 'Désactivé' : !settings().globalEnabled ? 'Globaux désactivés' : settings().capturing ? 'Capture en cours' : settings().registrationFailed || !settings().registered ? 'À vérifier' : 'Enregistré';
  const dirtyChanged = () => { ctx.syncDirty(); const save = document.getElementById('save-shortcut'); if (save) save.disabled = !isDirty() || !settings().canWrite || ctx.busy(); const mark = document.getElementById('shortcut-dirty'); if (mark) mark.hidden = !isDirty(); const test = document.getElementById('test-shortcut'); if (test) test.disabled = isDirty() || selected == null || ctx.busy(); };
  function reset() { draft = null; original = null; revision = null; ctx?.syncDirty(); }
  function stopPolling() { clearTimeout(timer); until = 0; generation++; }
  async function refreshData() { const data = await Kigi.request('app.bootstrap'); ctx.replace(data); }
  async function operation(method, params = {}, message = '') {
    if (ctx.busy()) return null;
    ctx.setBusy(true);
    try {
      const result = await Kigi.request(method, params);
      await refreshData();
      if (message) ctx.status(result.canceled ? 'Action annulée' : message);
      ctx.clearError();
      return result;
    } catch (error) { ctx.fail(error); try { await refreshData(); } catch (_) {} return null; }
    finally { ctx.setBusy(false); ctx.render(); }
  }
  async function save() {
    if (!draft || ctx.busy()) return false;
    if (draft.command.kind === 'ApplyProfile' && !draft.command.profile_id) { ctx.fail(new Error('Choisissez un espace enregistré.')); return false; }
    if (draft.command.kind === 'Application' && !draft.command.bundle) { ctx.fail(new Error('Choisissez une application.')); return false; }
    ctx.setBusy(true);
    try {
      const index = selected == null ? settings().settings.entries.length : selected;
      await Kigi.request('shortcuts.upsert', {expectedRevision: revision, index: selected, binding: draft});
      selected = index; reset(); await ctx.syncDirty(); await refreshData(); ctx.clearError(); ctx.status('Raccourci enregistré'); return true;
    } catch (error) { ctx.fail(error); try { await refreshData(); } catch (_) {} return false; }
    finally { ctx.setBusy(false); ctx.render(); }
  }
  async function guard() {
    if (!isDirty()) return true;
    const choice = await ctx.choose('Raccourci non enregistré', 'Enregistrer les modifications avant de continuer ?', [['save', 'Enregistrer', 'primary'], ['discard', 'Ignorer'], ['cancel', 'Annuler']]);
    if (choice === 'save') return save();
    if (choice === 'discard') { reset(); await ctx.syncDirty(); return true; }
    return false;
  }
  async function select(index) { if (ctx.busy() || !(await ctx.guard())) return; selected = index; reset(); ctx.render(); }
  async function add() {
    if (ctx.busy() || !(await ctx.guard())) return;
    if (!settings().canWrite || settings().settings.entries.length >= 32) return;
    selected = null; original = null; revision = settings().revision;
    draft = {enabled: false, key: 45, modifiers: 2048, key_label: 'N', command: {kind: 'Maximize'}};
    ctx.render(); dirtyChanged();
  }
  async function capture() {
    const result = await operation('shortcuts.startCapture', {}, 'Capture terminée');
    if (result && !result.canceled) { draft.key = result.key; draft.modifiers = result.modifiers; draft.key_label = result.keyLabel; dirtyChanged(); ctx.render(); ctx.status(`Combinaison ${combo(draft)} capturée. Cliquez sur Enregistrer pour l’appliquer.`); }
    if (result && !result.registrationRestored) ctx.fail(new Error('La capture est terminée, mais macOS refuse le réenregistrement. Consultez les diagnostics.'));
  }
  function field(parent, label, input) { const group = ctx.el('div', 'field'); const text = ctx.el('label', '', label); if (input.id) text.htmlFor = input.id; group.append(text, input); parent.append(group); }
  function check(parent, label, checked, action, id) { const line = ctx.el('label', 'switch-line'); const input = ctx.el('input'); input.type = 'checkbox'; input.checked = checked; if (id) input.id = id; input.addEventListener('change', () => action(input.checked)); line.append(input, ctx.el('span', '', label)); parent.append(line); return input; }
  function tools(parent, items) { const bar = ctx.el('div', 'system-actions'); for (const [label, action] of items) bar.append(ctx.button(label, action, 'subtle')); parent.append(bar); return bar; }
  function shortcuts(content, inspector) {
    const data = settings(), entries = data.settings.entries;
    if (!draft && selected !== null && entries.length) { selected = Math.min(selected, entries.length - 1); draft = copy(entries[selected]); original = copy(draft); revision = data.revision; }
    ctx.heading(content, 'Raccourcis', 'Les combinaisons sont capturées et enregistrées par macOS.');
    if (data.error) content.append(ctx.el('div', 'inline-warning', data.error));
    if (data.registrationFailed) content.append(ctx.el('div', 'inline-warning', 'macOS refuse au moins une combinaison. Les entrées actives sont à vérifier.'), ctx.el('pre', 'report', data.registrationReport));
    check(content, 'Activer les raccourcis globaux', data.globalEnabled, enabled => operation('shortcuts.setGlobalEnabled', {enabled}, enabled ? 'Activation demandée ; état de l’enregistrement actualisé' : 'Raccourcis globaux désactivés'), 'global-shortcuts');
    const controls = ctx.el('div', 'system-actions');
    const search = ctx.el('input'); search.type = 'search'; search.id = 'shortcut-search'; search.placeholder = 'Rechercher une combinaison ou une action'; search.setAttribute('aria-label', 'Rechercher les raccourcis'); search.value = filter;
    search.addEventListener('input', () => { const position = search.selectionStart; filter = search.value; ctx.render(); const next = document.getElementById('shortcut-search'); next.focus(); next.setSelectionRange(position, position); });
    const mode = ctx.el('select'); mode.setAttribute('aria-label', 'Filtrer les raccourcis'); for (const [value, label] of [['all', 'Tous'], ['enabled', 'Actifs'], ['disabled', 'Désactivés'], ['conflict', 'À vérifier']]) { const option = ctx.el('option', '', label); option.value = value; mode.append(option); } mode.value = scope; mode.addEventListener('change', () => { scope = mode.value; ctx.render(); });
    const create = ctx.button('Ajouter', add); create.id = 'add-shortcut'; create.disabled = !data.canWrite || entries.length >= 32; controls.append(search, mode, create); content.append(controls);
    const table = ctx.el('table', 'shortcut-table'), head = ctx.el('thead'), header = ctx.el('tr'); for (const label of ['Combinaison', 'Action', 'État']) header.append(ctx.el('th', '', label)); head.append(header); table.append(head); const body = ctx.el('tbody');
    entries.forEach((binding, index) => {
      const status = rowState(binding), haystack = `${combo(binding)} ${actionName(binding.command)}`.toLocaleLowerCase();
      if (filter && !haystack.includes(filter.toLocaleLowerCase()) || scope === 'enabled' && !binding.enabled || scope === 'disabled' && binding.enabled || scope === 'conflict' && status !== 'À vérifier') return;
      const row = ctx.el('tr', index === selected ? 'selected' : ''); const combination = ctx.el('td'); const choose = ctx.button(combo(binding), () => select(index), 'row-select'); choose.dataset.shortcutIndex = index; combination.append(choose); row.append(combination, ctx.el('td', '', actionName(binding.command)), ctx.el('td', 'muted', status)); body.append(row);
    }); table.append(body); content.append(table);
    const serviceTools = tools(content, [['Réenregistrer', () => operation('shortcuts.reregister', {}, 'État du réenregistrement actualisé')], ['Recharger le fichier', async () => { if (await ctx.guard()) { const result = await operation('shortcuts.reload', {}, 'Raccourcis rechargés'); if (result) { reset(); ctx.render(); } } }], ['Valeurs par défaut', async () => { if (!(await ctx.guard())) return; if (await ctx.choose('Rétablir les raccourcis ?', 'Les combinaisons et applications personnalisées seront remplacées. Les espaces sont conservés.', [['reset', 'Rétablir', 'danger'], ['cancel', 'Annuler']]) !== 'reset') return; const result = await operation('shortcuts.reset', {expectedRevision: settings().revision, confirmed: true}, 'Valeurs par défaut enregistrées'); if (result) { reset(); ctx.render(); } }]]);
    serviceTools.lastElementChild.disabled = !data.canWrite;
    inspector.replaceChildren(ctx.el('h2', '', 'RACCOURCI')); if (!draft) { inspector.append(ctx.el('p', '', 'Ajoutez ou sélectionnez un raccourci.')); return; }
    const mark = ctx.el('span', 'tag', 'Modifié'); mark.id = 'shortcut-dirty'; mark.hidden = !isDirty(); inspector.append(mark);
    check(inspector, 'Activer cette entrée', draft.enabled, enabled => { draft.enabled = enabled; dirtyChanged(); }, 'shortcut-enabled').disabled = !data.canWrite;
    const record = ctx.button(combo(draft), capture, 'app-picker'); record.id = 'capture-shortcut'; record.disabled = !data.canWrite; field(inspector, 'Capturer la combinaison', record);
    const action = ctx.el('select'); action.id = 'shortcut-command';
    for (const [value, label] of [['Application', 'Ouvrir / activer une application'], ['ChatGpt', 'Retrouver ChatGPT'], ['Maximize', 'Maximiser'], ['PairLeft', 'Partager : gauche puis droite'], ['PairRight', 'Partager : droite puis gauche'], ['Test', 'Test / restauration de fenêtre'], ['Work', 'Travail — Chrome 100 %'], ['Meeting', 'Réunion — Teams / Chrome'], ['Restore', 'Restaurer la disposition'], ['ApplyProfile', 'Appliquer un espace enregistré']]) { const option = ctx.el('option', '', label); option.value = value; action.append(option); }
    action.value = draft.command.kind; action.disabled = !data.canWrite; action.addEventListener('change', () => { draft.command = action.value === 'Application' ? {kind: 'Application', bundle: '', name: '', maximize: false} : action.value === 'ApplyProfile' ? {kind: 'ApplyProfile', profile_id: ctx.data().spaces.profiles[0]?.id || 0} : {kind: action.value}; dirtyChanged(); ctx.render(); }); field(inspector, 'Action', action);
    if (draft.command.kind === 'Application') {
      const app = ctx.button(draft.command.name || 'Choisir une application…', async () => { const result = await operation('spaces.selectApp'); if (result && !result.canceled) { draft.command.bundle = result.bundleId; draft.command.name = result.name; dirtyChanged(); ctx.render(); } }, 'app-picker'); app.disabled = !data.canWrite; field(inspector, 'Application', app);
      check(inspector, 'Maximiser après activation', draft.command.maximize, value => { draft.command.maximize = value; dirtyChanged(); }, 'shortcut-maximize').disabled = !data.canWrite;
    }
    if (draft.command.kind === 'ApplyProfile') {
      const profiles = ctx.el('select'); profiles.id = 'shortcut-profile';
      const placeholder = ctx.el('option', '', 'Choisir un espace…'); placeholder.value = '0'; profiles.append(placeholder);
      for (const profile of ctx.data().spaces.profiles) { const item = ctx.el('option', '', profile.name); item.value = String(profile.id); profiles.append(item); }
      if (draft.command.profile_id && !ctx.data().spaces.profiles.some(p => p.id === draft.command.profile_id)) { const missing = ctx.el('option', '', `Espace supprimé #${draft.command.profile_id}`); missing.value = String(draft.command.profile_id); profiles.append(missing); }
      profiles.value = String(draft.command.profile_id); profiles.disabled = !data.canWrite;
      profiles.addEventListener('change', () => { draft.command.profile_id = Number(profiles.value); dirtyChanged(); }); field(inspector, 'Espace enregistré', profiles);
      inspector.append(ctx.el('p', '', 'Applique la version enregistrée de cet espace. Ouvrez ses applications avant de déclencher le raccourci.'));
    }
    const actions = ctx.el('div', 'system-actions'); const persist = ctx.button('Enregistrer', save, 'primary'); persist.id = 'save-shortcut'; actions.append(persist);
    if (selected != null) { const remove = ctx.button('Supprimer', async () => { if (!(await ctx.guard())) return; if (await ctx.choose('Supprimer le raccourci ?', 'Cette combinaison ne déclenchera plus son action.', [['delete', 'Supprimer', 'danger'], ['cancel', 'Annuler']]) !== 'delete') return; const result = await operation('shortcuts.delete', {expectedRevision: settings().revision, index: selected, confirmed: true}, 'Raccourci supprimé'); if (result) { selected = Math.max(0, selected - 1); reset(); ctx.render(); } }, 'danger'); remove.disabled = !data.canWrite; actions.append(remove); }
    inspector.append(actions);
    const test = ctx.button('Tester la commande enregistrée', async () => { if (await ctx.choose('Exécuter cette commande ?', 'Elle peut ouvrir une application, un onglet ou déplacer des fenêtres.', [['test', 'Exécuter'], ['cancel', 'Annuler']]) !== 'test') return; const result = await operation('shortcuts.test', {index: selected, expectedRevision: settings().revision}); if (result) ctx.status(result.message); }, 'subtle'); test.id = 'test-shortcut'; inspector.append(test, ctx.el('p', '', 'Enregistrez avant de tester. Une commande transmise n’est pas une confirmation de réussite.'));
    dirtyChanged();
  }
  function poll() {
    stopPolling(); until = Date.now() + 13000; const token = generation;
    const tick = async () => {
      if (token !== generation || ctx.route() !== 'gestures' || Date.now() > until) return;
      try { const data = await Kigi.request('gestures.get'); if (token !== generation) return; ctx.data().gestures = data; ctx.render(); if (data.listening) timer = setTimeout(tick, 1000); } catch (error) { ctx.fail(error); }
    }; timer = setTimeout(tick, 500);
  }
  function gestures(content, inspector) {
    const data = ctx.data().gestures; ctx.heading(content, 'Gestes du trackpad', 'Pilote expérimental · un doigt, depuis les 10 % extérieurs vers l’intérieur.');
    const diagram = ctx.el('div', 'trackpad-diagram'); diagram.setAttribute('role', 'img'); diagram.setAttribute('aria-label', 'Deux bandes de départ de 10 pour cent, glissement horizontal vers l’intérieur'); diagram.append(ctx.el('span', 'edge', '10 %'), ctx.el('span', 'gesture-direction', '›  Un doigt vers l’intérieur  ‹'), ctx.el('span', 'edge', '10 %')); content.append(diagram);
    check(content, 'Activer les gestes globaux expérimentaux', data.active, async enabled => { const result = await operation('gestures.setEnabled', {enabled}, enabled ? 'Pilote démarré ; réception à vérifier' : 'Gestes arrêtés'); if (result && enabled) poll(); else stopPolling(); }, 'gestures-enabled');
    content.append(ctx.el('p', '', 'Gauche : Teams · droite : Chrome. Déplacement intérieur d’au moins 10 % de la largeur, durée maximale 1,5 s, dérive verticale limitée. Sans lever le doigt : revenez dans une bande de bord, puis glissez à nouveau vers l’intérieur. Un retour au même bord exige un recul d’au moins 6 % pour éviter les rebonds. Les cibles et la zone ne sont pas configurables dans ce pilote.'));
    tools(content, [['Test local', () => operation('gestures.startLocalProbe', {}, 'Fenêtre du test local ouverte')], ['Diagnostic global', async () => { const result = await operation('gestures.startGlobalProbe', {}, 'Diagnostic passif démarré ; aucune action d’app'); if (result) poll(); }], ['Arrêter', async () => { stopPolling(); await operation('gestures.stop', {}, 'Écoute globale arrêtée'); }]]);
    content.append(ctx.el('pre', 'report', data.summary)); if (data.interrupted) content.append(ctx.el('div', 'inline-warning', 'Écoute interrompue. Une réactivation manuelle est nécessaire.'));
    inspector.replaceChildren(ctx.el('h2', '', 'RÉCEPTION RÉELLE')); ctx.field(inspector, 'Écoute', data.listening ? 'Active' : 'Arrêtée');
    for (const [key, label] of [['events', 'Événements'], ['contacts', 'Avec contacts'], ['began', 'Débuts de contact'], ['edgeStarts', 'Départs sur un bord'], ['recognized', 'Gestes reconnus']]) ctx.field(inspector, label, data.counts?.[key] == null ? 'Non mesuré' : String(data.counts[key]));
    inspector.append(ctx.el('p', '', 'Le test local ne valide pas l’écoute globale. Compteurs actualisés pendant 13 secondes après un essai, puis à la demande.'));
  }
  function mouse(content, inspector) {
    const data = ctx.data().mouse; ctx.heading(content, 'Souris', 'Contrôlez le défilement horizontal des souris compatibles.');
    check(content, 'Bloquer le défilement horizontal', data.requested, enabled => operation('mouse.setHorizontalBlocked', {enabled}, enabled ? 'Filtre demandé ; état réel actualisé' : 'Filtre désactivé'), 'mouse-blocked');
    content.append(ctx.el('p', '', 'Le défilement vertical est conservé. Le trackpad et les séquences non identifiées passent. L’API ne permet pas d’affirmer la marque de la souris.'));
    if (data.requested && !data.active) tools(content, [['Réactiver le filtre', () => operation('mouse.setHorizontalBlocked', {enabled: true})]]);
    content.append(ctx.el('pre', 'report', data.report || 'Aucune mesure de filtre dans cette session.'));
    inspector.replaceChildren(ctx.el('h2', '', 'ÉTAT DU FILTRE')); ctx.field(inspector, 'Préférence enregistrée', data.requested ? 'Activée' : 'Désactivée'); ctx.field(inspector, 'Filtre actif', data.active ? 'Oui' : 'Non'); ctx.field(inspector, 'Accessibilité', data.accessibility ? 'Accordée' : 'Refusée / indisponible');
  }
  function permissions(content, inspector) {
    const data = ctx.data().permissions; ctx.heading(content, 'Autorisations', 'L’affichage ne déclenche aucune demande de permission.');
    for (const [key, label, section, method, detail] of [['accessibility', 'Accessibilité', 'Accessibility', 'permissions.requestAccessibility', 'Déplacement et activation des fenêtres.'], ['inputMonitoring', 'Surveillance de l’entrée', 'InputMonitoring', 'permissions.requestInputMonitoring', 'Réception des événements du trackpad, sans garantie de contacts.'], ['chromeAutomation', 'Automatisation Chrome', 'Automation', null, 'Dernier appel ChatGPT de cette session ; une révocation ultérieure n’est connue qu’au prochain essai.']]) {
      const row = ctx.el('div', 'permission'), head = ctx.el('div', 'card-head'); head.append(ctx.el('h2', '', label), ctx.el('span', `state ${data[key]}`, ({granted: 'Accordée', denied: 'Refusée / indisponible', unknown: 'Non vérifiée'})[data[key]])); row.append(head, ctx.el('p', '', detail));
      const actions = []; if (method && data[key] !== 'granted') actions.push(['Demander l’accès', () => operation(method, {}, 'Demande transmise ; actualisez après votre décision dans macOS')]); actions.push(['Ouvrir Réglages Système', () => operation('permissions.openSettings', {section}, 'Ouverture demandée ; choisissez la rubrique dans Confidentialité et sécurité')]); tools(row, actions); content.append(row);
    }
    inspector.replaceChildren(ctx.el('h2', '', 'COPIE ACTUELLE')); ctx.field(inspector, 'Application', `Kigi ${ctx.data().version}`); ctx.field(inspector, 'Bundle', 'io.kigi.windows'); inspector.append(ctx.el('p', '', 'Après remplacement d’une version ad hoc, macOS peut demander de réautoriser la copie actuelle. Une permission accordée ne confirme pas une action réussie.'));
  }
  function diagnostics(content, inspector) {
    const data = ctx.data().diagnostics; ctx.heading(content, 'Diagnostics', 'État et mesures de la session. Actualisez après un essai.');
    tools(content, [['Copier le rapport', () => operation('diagnostics.copy', {}, 'Rapport backend copié')], ['Exporter .txt', async () => { const result = await operation('diagnostics.export'); if (result) ctx.status(result.saved ? 'Rapport backend exporté' : 'Export annulé'); }], ['Réenregistrer les raccourcis', () => operation('shortcuts.reregister', {}, 'État de l’enregistrement actualisé')], ['Interface de secours', () => operation('app.openNativeFallback')]]);
    content.append(ctx.el('pre', 'report', data.performance), ctx.el('h2', '', 'Dernier résultat'), ctx.el('p', '', data.lastResult || 'Aucune action mesurée.'));
    inspector.replaceChildren(ctx.el('h2', '', 'MESURES')); ctx.field(inspector, 'RSS du processus Rust', data.rustRssBytes == null ? 'Non mesuré' : `${(data.rustRssBytes / 1048576).toFixed(1)} Mio`); ctx.field(inspector, 'Mémoire auxiliaire WebKit', 'Non mesurée'); ctx.field(inspector, 'Raccourcis reçus', String(data.shortcutsReceived)); ctx.field(inspector, 'Retour de restauration', data.restoreAvailable ? 'Disponible' : 'Absent');
  }
  return {setup(context) { ctx = context; }, dirty: isDirty, guard, reset, stopPolling, render(route, content, inspector) { ({shortcuts, gestures, mouse, permissions, diagnostics})[route](content, inspector); }};
})();
