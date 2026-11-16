'use strict';
// Configuration views only. All registration, permissions, reports and input stay native.
window.KigiSystem = (() => {
  let ctx, draft = null, original = null, selected = 0, revision = null, filter = '', scope = 'all', timer, until = 0, generation = 0, captureRequired = false;
  let meetingDraft = null, meetingOriginal = null, meetingRevision = null, meetingTimer;
  let runtimeDraft = null, runtimeOriginal = null, runtimeRevision = null;
  const runtimeDirty = () => runtimeDraft != null && JSON.stringify(runtimeDraft) !== JSON.stringify(runtimeOriginal);
  const meetingDirty = () => meetingDraft != null && JSON.stringify(meetingDraft) !== JSON.stringify(meetingOriginal);
  const copy = value => value == null ? null : JSON.parse(JSON.stringify(value));
  const isDirty = () => runtimeDirty() || meetingDirty() || (draft != null && (original == null || JSON.stringify(draft) !== JSON.stringify(original)));
  const settings = () => ctx.data().shortcuts;
  const combo = binding => [[4096, '⌃'], [2048, '⌥'], [512, '⇧'], [256, '⌘']].filter(([flag]) => binding.modifiers & flag).map(([, glyph]) => glyph).join('') + binding.key_label;
  const actionName = command => ({TeamsSplit: 'Réunion Teams / chat', RightHalf: 'Fenêtre active à droite 50 %', CycleWindow: 'Fenêtre suivante de cette application', CycleAllWindows: 'Fenêtre suivante, toutes applications', NextTab: 'Onglet suivant (Ctrl+Tab)', PreviousTab: 'Onglet précédent (Ctrl+⇧Tab)', Application: command.name || 'Application', ChatGpt: 'ChatGPT dans Chrome', Maximize: 'Maximiser', PairLeft: 'Gauche puis droite', PairRight: 'Droite puis gauche', Test: 'Test / restauration de fenêtre', Work: 'Travail — Chrome 100 %', Meeting: 'Réunion — Teams / Chrome', Restore: 'Restaurer la disposition', ApplyProfile: ctx?.data().spaces.profiles.find(p => p.id === command.profile_id)?.name ? `Espace : ${ctx.data().spaces.profiles.find(p => p.id === command.profile_id).name}` : `Espace supprimé #${command.profile_id}`})[command.kind];
  const rowState = binding => !binding.enabled ? 'Désactivé' : !settings().globalEnabled ? 'Globaux désactivés' : settings().capturing ? 'Capture en cours' : settings().registrationFailed || !settings().registered ? 'À vérifier' : 'Enregistré';
  const dirtyChanged = () => { ctx.syncDirty(); const runtimeSave = document.getElementById('save-runtime'); if (runtimeSave) runtimeSave.disabled = !runtimeDirty() || ctx.busy(); const meetingSave = document.getElementById('save-meetings'); if (meetingSave) meetingSave.disabled = !meetingDirty() || ctx.busy(); const meetingMark = document.getElementById('meeting-dirty'); if (meetingMark) meetingMark.hidden = !meetingDirty(); const save = document.getElementById('save-shortcut'); if (save) save.disabled = captureRequired || !isDirty() || !settings().canWrite || ctx.busy(); const mark = document.getElementById('shortcut-dirty'); if (mark) mark.hidden = !isDirty(); const test = document.getElementById('test-shortcut'); if (test) test.disabled = isDirty() || selected == null || ctx.busy(); };
  function reset() { runtimeDraft = null; runtimeOriginal = null; runtimeRevision = null; meetingDraft = null; meetingOriginal = null; meetingRevision = null; captureRequired = false; draft = null; original = null; revision = null; ctx?.syncDirty(); }
  function stopPolling() { clearTimeout(meetingTimer); meetingTimer = null; clearTimeout(timer); until = 0; generation++; }
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
    if (runtimeDirty()) return saveRuntime();
    if (meetingDirty()) return saveMeetings();
    if (!draft || ctx.busy()) return false;
    if (captureRequired) { ctx.fail(new Error('Choisissez une combinaison avant d’enregistrer.')); return false; }
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
    const choice = await ctx.choose(runtimeDirty() ? 'Réglages non enregistrés' : meetingDirty() ? 'Réglages Teams non enregistrés' : 'Raccourci non enregistré', 'Enregistrer les modifications avant de continuer ?', [['save', 'Enregistrer', 'primary'], ['discard', 'Ignorer'], ['cancel', 'Annuler']]);
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
    if (result && !result.canceled) { captureRequired = false; draft.key = result.key; draft.modifiers = result.modifiers; draft.key_label = result.keyLabel; dirtyChanged(); ctx.render(); ctx.status(`Combinaison ${combo(draft)} capturée. Cliquez sur Enregistrer pour l’appliquer.`); }
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
    const record = ctx.button(captureRequired ? 'Choisir une combinaison…' : combo(draft), capture, 'app-picker'); record.id = 'capture-shortcut'; record.disabled = !data.canWrite; field(inspector, 'Capturer la combinaison', record);
    const action = ctx.el('select'); action.id = 'shortcut-command';
    for (const [value, label] of [['Application', 'Ouvrir / activer une application'], ['ChatGpt', 'Retrouver ChatGPT'], ['TeamsSplit', 'Réunion Teams / chat'], ['RightHalf', 'Fenêtre active à droite 50 %'], ['CycleWindow', 'Fenêtre suivante de cette application'], ['CycleAllWindows', 'Fenêtre suivante, toutes applications'], ['NextTab', 'Onglet suivant (Ctrl+Tab)'], ['PreviousTab', 'Onglet précédent (Ctrl+⇧Tab)'], ['Maximize', 'Maximiser'], ['PairLeft', 'Partager : gauche puis droite'], ['PairRight', 'Partager : droite puis gauche'], ['Test', 'Test / restauration de fenêtre'], ['Work', 'Travail — Chrome 100 %'], ['Meeting', 'Réunion — Teams / Chrome'], ['Restore', 'Restaurer la disposition'], ['ApplyProfile', 'Appliquer un espace enregistré']]) { const option = ctx.el('option', '', label); option.value = value; action.append(option); }
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
      inspector.append(ctx.el('p', '', 'Applique la version enregistrée de cet espace. Le lancement des applications absentes se règle dans Général.'));
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
  function runtimeSettings() {
    if (!runtimeDraft) { runtimeDraft = copy(ctx.data().runtime.settings); runtimeOriginal = copy(runtimeDraft); runtimeRevision = ctx.data().runtime.revision; }
    return runtimeDraft;
  }
  async function saveRuntime() {
    if (!runtimeDraft || ctx.busy()) return false;
    ctx.setBusy(true);
    try {
      await Kigi.request('runtime.save', {expectedRevision:runtimeRevision,settings:runtimeDraft});
      runtimeDraft = null; runtimeOriginal = null; runtimeRevision = null;
      await ctx.syncDirty(); await refreshData(); ctx.clearError(); ctx.status('Réglages enregistrés ; état du moteur actualisé'); return true;
    } catch (error) {ctx.fail(error);return false;}
    finally {ctx.setBusy(false);ctx.render();}
  }
  function runtimeSaveButton(parent) { const save = ctx.button('Enregistrer et appliquer', saveRuntime, 'primary'); save.id = 'save-runtime'; parent.append(save); dirtyChanged(); }
  function targetSelect(parent, key, label) {
    const draft = runtimeSettings(), input = ctx.el('select'); input.id = `gesture-${key}`;
    const disabled = ctx.el('option', '', 'Aucune action'); disabled.value = 'disabled'; input.append(disabled);
    ctx.data().shortcuts.settings.entries.forEach((binding,index) => {const option = ctx.el('option','',`${combo(binding)} · ${actionName(binding.command)}${binding.enabled?'':' (désactivé)'}`); option.value=`shortcut:${index}`; input.append(option);});
    ctx.data().spaces.profiles.forEach(profile => {const option=ctx.el('option','',`Espace : ${profile.name}`);option.value=`profile:${profile.id}`;input.append(option);});
    const target=draft[key];
    if(target.kind==='Shortcut') {const index=ctx.data().shortcuts.settings.entries.findIndex(b=>b.key===target.key&&b.modifiers===target.modifiers);if(index>=0) input.value=`shortcut:${index}`;else{const missing=ctx.el('option','','Raccourci absent · choisir sa nouvelle combinaison');missing.value='missing';input.append(missing);input.value='missing';}}
    else input.value=target.kind==='Profile'?`profile:${target.profile_id}`:'disabled';
    input.addEventListener('change',()=>{const [kind,id]=input.value.split(':');if(kind==='shortcut'){const binding=ctx.data().shortcuts.settings.entries[Number(id)];draft[key]={kind:'Shortcut',key:binding.key,modifiers:binding.modifiers};}else draft[key]=kind==='profile'?{kind:'Profile',profile_id:Number(id)}:{kind:'Disabled'};dirtyChanged();});field(parent,label,input);
  }
  function general(content, inspector) {
    const draft=runtimeSettings();ctx.heading(content,'Général','Retrouvez votre workflow au démarrage et adaptez la navigation.');
    check(content,'Lancer Kigi en arrière-plan à l’ouverture de ma session',ctx.data().runtime.loginEnabled,async enabled=>{const result=await operation('runtime.setLoginEnabled',{enabled},enabled?'Lancement à la prochaine connexion activé':'Lancement à la connexion désactivé');},'login-enabled');
    content.append(ctx.el('p','muted','Installez Kigi dans Applications avant d’activer le lancement à la connexion. L’icône de menu et le Dock permettent ensuite d’ouvrir Studio.'));
    check(content,'Au lancement : organiser la réunion Teams présente, sinon ouvrir Chrome à 100 %',draft.startupWork,value=>{draft.startupWork=value;dirtyChanged();},'startup-work');
    check(content,'Ouvrir les applications absentes lorsque j’applique un espace',draft.launchMissingApps,value=>{draft.launchMissingApps=value;dirtyChanged();},'launch-missing');
    check(content,'⌘Tab parcourt les fenêtres, toutes applications confondues',draft.windowSwitcher,value=>{draft.windowSwitcher=value;dirtyChanged();},'window-switcher');
    content.append(ctx.el('p','','⌘⇧Tab parcourt dans l’autre sens. La bascule conserve une liste pendant l’enchaînement et regroupe les applications récemment activées. Désactiver cette option rend immédiatement ⌘Tab à macOS.'));
    content.append(ctx.el('p','','Pour les onglets, les actions disponibles transmettent Ctrl+Tab ou Ctrl+⇧Tab à l’application active. Chrome les prend en charge ; les autres applications peuvent leur donner un autre comportement. Choisissez les combinaisons dans Raccourcis.'));
    runtimeSaveButton(content);
    inspector.replaceChildren(ctx.el('h2','','NAVIGATION'));ctx.field(inspector,'Remplacement ⌘Tab',ctx.data().runtime.switcherActive?'Actif':ctx.data().runtime.settings.windowSwitcher?'Demandé · vérifier Accessibilité':'Désactivé');
  }
  function gestures(content, inspector) {
    const data=ctx.data().gestures,draft=runtimeSettings();ctx.heading(content,'Gestes du trackpad','Un doigt depuis un bord vers l’intérieur, sur toute la hauteur.');
    check(content,'Activer et reprendre les gestes après relance / veille',draft.gesturesEnabled,value=>{draft.gesturesEnabled=value;dirtyChanged();},'gestures-enabled');
    targetSelect(content,'left','Geste depuis le bord gauche');targetSelect(content,'right','Geste depuis le bord droit');
    for(const [key,label,min,max] of [['edgePercent','Largeur de chaque bande de départ (%)',3,25],['travelPercent','Déplacement intérieur minimal (%)',5,40],['durationMs','Durée maximale du geste (ms)',500,3000]]) {
      const input=ctx.el('input');input.id=`gesture-${key}`;input.type='number';input.min=min;input.max=max;input.step=key==='durationMs'?100:1;input.value=draft[key];input.addEventListener('input',()=>{draft[key]=Number(input.value);dirtyChanged();});field(content,label,input);
    }
    content.append(ctx.el('p','','Une action liée à un raccourci suit sa commande actuelle : changer l’application de ⌥T change aussi le geste associé. Si vous changez la combinaison ou supprimez l’entrée, sélectionnez à nouveau sa cible ici. Sans lever le doigt, revenez à un bord puis glissez vers l’intérieur ; un retour au même bord demande un recul de6 %.'));
    runtimeSaveButton(content);
    tools(content,[['Test local',async()=>{if(await ctx.guard())return operation('gestures.startLocalProbe',{},'Fenêtre du test local ouverte');}],['Diagnostic global',async()=>{if(!(await ctx.guard()))return;const result=await operation('gestures.startGlobalProbe',{},'Diagnostic passif démarré');if(result)poll();}],['Arrêter les gestes',async()=>{if(!(await ctx.guard()))return;runtimeDraft=null;runtimeOriginal=null;runtimeRevision=null;stopPolling();await operation('gestures.stop',{},'Gestes désactivés et préférence mémorisée');}]]);
    content.append(ctx.el('pre','report',data.summary));inspector.replaceChildren(ctx.el('h2','','ÉCOUTE RÉELLE'));ctx.field(inspector,'Écoute',data.listening?'Active':'Arrêtée');
    for(const [key,label] of [['events','Événements'],['contacts','Avec contacts'],['recognized','Gestes reconnus']])ctx.field(inspector,label,data.counts?.[key]==null?'Non mesuré':String(data.counts[key]));
    inspector.append(ctx.el('p','','La reprise automatique utilise au maximum trois tentatives sans demander d’autorisation en arrière-plan. La réception globale reste dépendante du trackpad et de macOS.'));
  }
  function mouse(content, inspector) {
    const data = ctx.data().mouse; ctx.heading(content, 'Souris', 'Contrôlez le défilement horizontal des souris compatibles.');
    check(content, 'Bloquer le défilement horizontal', data.requested, enabled => operation('mouse.setHorizontalBlocked', {enabled}, enabled ? 'Filtre demandé ; état réel actualisé' : 'Filtre désactivé'), 'mouse-blocked');
    content.append(ctx.el('p', '', 'Le défilement vertical est conservé. Le trackpad et les séquences non identifiées passent. L’API ne permet pas d’affirmer la marque de la souris.'));
    if (data.requested && !data.active) tools(content, [['Réactiver le filtre', () => operation('mouse.setHorizontalBlocked', {enabled: true})]]);
    content.append(ctx.el('pre', 'report', data.report || 'Aucune mesure de filtre dans cette session.'));
    inspector.replaceChildren(ctx.el('h2', '', 'ÉTAT DU FILTRE')); ctx.field(inspector, 'Préférence enregistrée', data.requested ? 'Activée' : 'Désactivée'); ctx.field(inspector, 'Filtre actif', data.active ? 'Oui' : 'Non'); ctx.field(inspector, 'Accessibilité', data.accessibility ? 'Accordée' : 'Refusée / indisponible');
  }
  async function saveMeetings() {
    if (!meetingDraft || ctx.busy()) return false;
    ctx.setBusy(true);
    try {
      await Kigi.request('meetings.save', {expectedRevision: meetingRevision, settings: meetingDraft});
      meetingDraft = null; meetingOriginal = null; meetingRevision = null;
      await ctx.syncDirty(); await refreshData(); ctx.clearError();
      ctx.status('Réglages Teams enregistrés. L’état indique si cette réunion peut être automatisée.'); return true;
    } catch (error) { ctx.fail(error); return false; }
    finally { ctx.setBusy(false); ctx.render(); }
  }
  function pollMeetings() {
    if (meetingTimer || ctx.route() !== 'meetings') return;
    const token = generation;
    meetingTimer = setTimeout(async () => {
      meetingTimer = null;
      if (token !== generation || ctx.route() !== 'meetings') return;
      if (!ctx.busy()) {
        try {
          const data = await Kigi.request('meetings.get');
          if (token !== generation || ctx.route() !== 'meetings') return;
          ctx.data().meetings = data;
          const status = document.getElementById('meeting-live-status');
          if (status) status.textContent = data.status || 'Configurez et activez le profil pour commencer.';
        } catch (_) { /* The explicit refresh action reports persistent bridge errors. */ }
      }
      pollMeetings();
    }, 2000);
  }
  function meetings(content, inspector) {
    const data = ctx.data().meetings;
    if (!meetingDraft) { meetingDraft = copy(data.settings); meetingOriginal = copy(data.settings); meetingRevision = data.revision; }
    ctx.heading(content, 'Réunions Teams', 'Le chat et la réunion doivent exister dans deux fenêtres Teams séparées.');
    content.append(ctx.el('p', '', 'À la première configuration, ouvrez une réunion puis identifiez les fenêtres. Les repères de titre restent locaux. Gardez une partie stable du titre, commune à vos réunions, en retirant les noms de personnes ou de réunions. Aucun titre n’est ajouté aux journaux.'));
    tools(content, [['Identifier les fenêtres…', async () => {
      const result = await operation('meetings.identify');
      if (result && !result.canceled) { meetingDraft.chatTitle = result.chatTitle; meetingDraft.meetingTitle = result.meetingTitle;
        const chatSuffix=result.chatTitle.includes('|')?result.chatTitle.slice(result.chatTitle.indexOf('|')+1).trim():'';
        const meetingSuffix=result.meetingTitle.includes('|')?result.meetingTitle.slice(result.meetingTitle.indexOf('|')+1).trim():'';
        if(chatSuffix && chatSuffix===meetingSuffix){meetingDraft.meetingTitle=meetingSuffix;meetingDraft.distinguishChat=true;}
        ctx.render(); dirtyChanged(); ctx.status('Fenêtres identifiées. Vérifiez les parties stables des titres puis testez la détection.'); }
    }]]);
    for (const [key, label] of [['chatTitle', 'Le titre du chat contient'], ['meetingTitle', 'Le titre de la réunion contient']]) {
      const input = ctx.el('input'); input.id = `meeting-${key}`; input.type = 'text'; input.maxLength = 256; input.autocomplete = 'off'; input.spellcheck = false; input.value = meetingDraft[key]; input.addEventListener('input', () => { meetingDraft[key] = input.value; dirtyChanged(); }); field(content, label, input);
    }
    check(content,'Exclure le chat et les pages Teams ordinaires du repère réunion',meetingDraft.distinguishChat,value=>{meetingDraft.distinguishChat=value;dirtyChanged();},'meeting-distinguish');
    check(content,'Détecter aussi la fin si le titre ne correspond plus au profil pendant2 secondes',meetingDraft.endOnTitleChange,value=>{meetingDraft.endOnTitleChange=value;dirtyChanged();},'meeting-title-end');
    const side = ctx.el('select'); side.id = 'meeting-side';
    for (const [value, label] of [['right', 'Réunion à droite, chat à gauche'], ['left', 'Réunion à gauche, chat à droite']]) { const option = ctx.el('option', '', label); option.value = value; side.append(option); }
    side.value = meetingDraft.meetingRight ? 'right' : 'left'; side.addEventListener('change', () => { meetingDraft.meetingRight = side.value === 'right'; dirtyChanged(); }); field(content, 'Disposition pendant la réunion', side);
    const percent = ctx.el('input'); percent.id = 'meeting-chat-percent'; percent.type = 'number'; percent.min = 20; percent.max = 80; percent.step = 1; percent.value = meetingDraft.chatPercent;
    percent.addEventListener('input', () => { meetingDraft.chatPercent = Number(percent.value); dirtyChanged(); }); field(content, 'Largeur du chat (%) · la réunion prend le reste', percent);
    const end = ctx.el('select'); end.id = 'meeting-end';
    for (const [value, label] of [['maximize', 'Remettre le chat à 100 %'], ['restore', 'Rétablir la position initiale du chat']]) { const option = ctx.el('option', '', label); option.value = value; end.append(option); }
    end.value = meetingDraft.maximizeAfter ? 'maximize' : 'restore'; end.addEventListener('change', () => { meetingDraft.maximizeAfter = end.value === 'maximize'; dirtyChanged(); }); field(content, 'À la fermeture de la réunion', end);
    check(content, 'Activer la disposition automatique', meetingDraft.enabled, enabled => { meetingDraft.enabled = enabled; dirtyChanged(); }, 'meetings-enabled');
    content.append(ctx.el('p', 'muted', 'L’activation vérifie les repères sur les deux fenêtres actuellement ouvertes. Après un déplacement manuel ou une autre disposition appliquée par Kigi, cette réunion reste sous votre contrôle. Une fenêtre minimisée ou en plein écran suspend l’automatisme.'));
    const actions = ctx.el('div', 'system-actions'); const save = ctx.button('Enregistrer', saveMeetings, 'primary'); save.id = 'save-meetings';
    const mark = ctx.el('span', 'tag', 'Modifié'); mark.id = 'meeting-dirty';
    actions.append(save, ctx.button('Tester la détection', async () => { const result = await operation('meetings.probe', {settings: meetingDraft}); if (result) ctx.status(result.message); }, 'subtle'), mark); content.append(actions);
    inspector.replaceChildren(ctx.el('h2', '', 'AUTOMATISME TEAMS'));
    ctx.field(inspector, 'Préférence enregistrée', data.settings.enabled ? 'Activée' : 'Désactivée');
    ctx.field(inspector, 'Version Teams détectée', data.teamsVersion || 'Teams non lancé / version indisponible');
    const liveStatus = ctx.el('p', 'report', data.status || 'Configurez et activez le profil pour commencer.'); liveStatus.id = 'meeting-live-status'; inspector.append(liveStatus);
    tools(inspector, [['Appliquer maintenant', async () => { if(!(await ctx.guard()))return; const result=await operation('meetings.apply'); if(result)ctx.status(result.message); }], ['Actualiser l’état', () => operation('meetings.get')], ['Suspendre cette réunion', () => operation('meetings.pause', {}, 'Automatisme suspendu jusqu’à la fermeture de la réunion')]]);
    inspector.append(ctx.el('p', 'muted', 'Une seconde de stabilité avant le placement. Deux secondes après la disparition confirmée de la fenêtre réunion. Aucun déplacement si la détection est ambiguë. À valider sur votre version de Teams.'));
    dirtyChanged(); pollMeetings();
  }
  function permissions(content, inspector) {
    const data = ctx.data().permissions; ctx.heading(content, 'Autorisations', 'L’affichage ne déclenche aucune demande. Tant que l’accès manque, chaque clic sur Demander l’accès rouvre Réglages Système.');
    for (const [key, label, section, method, detail] of [['accessibility', 'Accessibilité', 'Accessibility', 'permissions.requestAccessibility', 'Déplacement et activation des fenêtres.'], ['inputMonitoring', 'Surveillance de l’entrée', 'InputMonitoring', 'permissions.requestInputMonitoring', 'Réception des événements du trackpad, sans garantie de contacts.'], ['chromeAutomation', 'Automatisation Chrome', 'Automation', null, 'Dernier appel ChatGPT de cette session ; une révocation ultérieure n’est connue qu’au prochain essai.']]) {
      const row = ctx.el('div', 'permission'), head = ctx.el('div', 'card-head'); head.append(ctx.el('h2', '', label), ctx.el('span', `state ${data[key]}`, ({granted: 'Accordée', denied: 'Refusée / indisponible', unknown: 'Non vérifiée'})[data[key]])); row.append(head, ctx.el('p', '', detail));
      const actions = []; if (method && data[key] !== 'granted') actions.push(['Demander l’accès', async () => { const result = await operation(method); if (result) ctx.status(result.message); }]); actions.push(['Ouvrir Réglages Système', async () => { const result = await operation('permissions.openSettings', {section}); if (result) ctx.status(result.message); }]); tools(row, actions); content.append(row);
    }
    inspector.replaceChildren(ctx.el('h2', '', 'COPIE ACTUELLE')); ctx.field(inspector, 'Application', `Kigi ${ctx.data().version}`); ctx.field(inspector, 'Bundle', 'io.kigi.windows'); inspector.append(ctx.el('p', '', 'Après remplacement d’une version ad hoc, macOS peut demander de réautoriser la copie actuelle. Une permission accordée ne confirme pas une action réussie.'));
  }
  function diagnostics(content, inspector) {
    const data = ctx.data().diagnostics; ctx.heading(content, 'Diagnostics', 'État et mesures de la session. Actualisez après un essai.');
    tools(content, [['Copier le rapport', () => operation('diagnostics.copy', {}, 'Rapport backend copié')], ['Exporter .txt', async () => { const result = await operation('diagnostics.export'); if (result) ctx.status(result.saved ? 'Rapport backend exporté' : 'Export annulé'); }], ['Réenregistrer les raccourcis', () => operation('shortcuts.reregister', {}, 'État de l’enregistrement actualisé')]]);
    content.append(ctx.el('pre', 'report', data.performance), ctx.el('h2', '', 'Dernier résultat'), ctx.el('p', '', data.lastResult || 'Aucune action mesurée.'));
    inspector.replaceChildren(ctx.el('h2', '', 'MESURES')); ctx.field(inspector, 'RSS du processus Rust', data.rustRssBytes == null ? 'Non mesuré' : `${(data.rustRssBytes / 1048576).toFixed(1)} Mio`); ctx.field(inspector, 'Mémoire auxiliaire WebKit', 'Non mesurée'); ctx.field(inspector, 'Raccourcis reçus', String(data.shortcutsReceived)); ctx.field(inspector, 'Retour de restauration', data.restoreAvailable ? 'Disponible' : 'Absent');
  }
  function openForProfile(id) {
    reset(); filter = ''; scope = 'all'; selected = settings().settings.entries.findIndex(b => b.command.kind === 'ApplyProfile' && b.command.profile_id === id);
    if (selected < 0) {
      if (!settings().canWrite || settings().settings.entries.length >= 32) { selected = 0; ctx.fail(new Error('Impossible d’ajouter un raccourci. Vérifiez le fichier et la limite de 32 entrées.')); ctx.render(); return; }
      selected = null; revision = settings().revision; captureRequired = true;
      draft = {enabled: false, key:45, modifiers:2048, key_label:'N', command:{kind:'ApplyProfile', profile_id:id}};
    }
    ctx.render(); dirtyChanged();
  }
  return {openForProfile, combination: combo, setup(context) { ctx = context; }, dirty: isDirty, guard, reset, stopPolling, render(route, content, inspector) { ({shortcuts, gestures, general, mouse, meetings, permissions, diagnostics})[route](content, inspector); }};
})();
