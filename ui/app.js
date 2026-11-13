'use strict';
(() => {
  const $ = id => document.getElementById(id);
  const state = {data: null, route: 'home', selected: null, busy: false, draft: null, original: null, draftRevision: null, dirty: false, splitEdited: false, invalid: new Set(), dirtySync: Promise.resolve(), rawRatio: {}, layoutResult: null, nativeDirty: false};
  const routes = [['home', 'Accueil', '⌂'], ['spaces', 'Espaces', '▤'], ['shortcuts', 'Raccourcis', '⌘'], ['gestures', 'Trackpad', '◫'], ['mouse', 'Souris', '◉'], ['permissions', 'Autorisations', '◇'], ['diagnostics', 'Diagnostics', '≋']];
  const el = (tag, className, text) => { const node = document.createElement(tag); if (className) node.className = className; if (text !== undefined) node.textContent = text; return node; };
  const button = (label, action, className = '') => { const node = el('button', className, label); node.type = 'button'; node.addEventListener('click', action); return node; };
  const names = bundle => ({'com.google.Chrome': 'Google Chrome', 'com.microsoft.teams2': 'Microsoft Teams', 'com.microsoft.teams': 'Microsoft Teams', 'com.apple.Safari': 'Safari'})[bundle] || bundle;
  const permissionName = value => ({granted: 'Accordée', denied: 'Refusée / indisponible', unknown: 'Non vérifiée'})[value] || 'Non vérifiée';
  const profiles = () => state.data?.spaces.profiles || [];
  const current = () => profiles().find(p => p.id === state.selected) || null;
  function failure(error) { $('error').textContent = error.message; $('error').hidden = false; $('status').textContent = 'Actualisation nécessaire'; }
  async function native() { if (!(await guardDraft())) return; try { await Kigi.request('app.openNativeFallback'); } catch (error) { failure(error); } }
  async function refresh() {
    if (state.busy || !(await guardDraft())) return;
    state.busy = true; $('refresh').disabled = true; $('content').setAttribute('aria-busy', 'true');
    try {
      const data = await Kigi.request('app.bootstrap');
      state.data = data; KigiSystem.reset(); state.draft = null; state.original = null; state.invalid.clear();
      if (!profiles().some(p => p.id === state.selected)) state.selected = profiles()[0]?.id ?? null;
      $('error').hidden = true;
      if (data.spaces.usingBackup) failure(new Error('Le catalogue principal est absent ou illisible. La sauvegarde est affichée en lecture seule. Vous pouvez la récupérer après confirmation.'));
      render();
      $('status').textContent = data.spaces.usingBackup ? 'Catalogue de sauvegarde · lecture seule' : 'Lecture du moteur terminée';
      $('version').textContent = `Kigi ${data.version} · Studio`;
    } catch (error) { failure(error); if (!state.data) $('content').replaceChildren(el('p', '', 'Données indisponibles. Actualisez ou ouvrez l’interface de secours.'), button('Interface de secours', native)); }
    finally { state.busy = false; $('refresh').disabled = false; $('content').setAttribute('aria-busy', 'false'); if (state.data) render(); $('recover-backup').hidden = !state.data?.spaces.usingBackup; }
  }
  async function navigate(route, id = state.selected) { if (!(await guardDraft())) return; KigiSystem.stopPolling(); KigiSystem.reset(); state.route = route; state.selected = id; state.draft = null; state.original = null; state.invalid.clear(); render(); $('page-title')?.focus(); }
  function field(parent, label, value, className = '') { const dl = el('dl', 'field'); dl.append(el('dt', '', label), el('dd', className, value)); parent.append(dl); }
  function heading(parent, title, subtitle) { const h = el('h1', '', title); h.id = 'page-title'; h.tabIndex = -1; parent.append(h); if (subtitle) parent.append(el('p', '', subtitle)); }
  function preview(profile, small = false) {
    const node = el('div', small ? 'mini-preview' : 'workspace-preview');
    node.setAttribute('aria-label', `Disposition enregistrée de ${profile.name}`);
    profile.windows.forEach((window, i) => {
      const w = el('div', `${small ? 'mini-window' : 'preview-window'}${i ? ' second' : ''}`);
      const g = window.geometry;
      for (const [key, value] of Object.entries({left: g.x, top: g.y, width: g.width, height: g.height})) w.style[key] = `${value * 100}%`;
      if (!small) {
        w.append(el('div', 'window-bar', names(window.bundle_id)));
        const body = el('div', 'window-body'); body.append(el('strong', '', names(window.bundle_id)), el('small', '', `${Math.round(g.width * 100)} % de largeur`)); w.append(body);
      }
      node.append(w);
    });
    return node;
  }
  function card(profile) { const node = button('', () => navigate('spaces', profile.id), 'space-card'); const head = el('div', 'card-head'); head.append(el('span', 'card-title', profile.name), el('span', 'tag', '2 fenêtres')); node.append(head, preview(profile, true), el('div', 'app-labels', profile.windows.map(w => names(w.bundle_id)).join(' · '))); return node; }
  const clone = value => value == null ? null : JSON.parse(JSON.stringify(value));
  const appNames = new Map();
  let modalResolve = null;
  function choose(title, message, choices) {
    if (modalResolve) return Promise.resolve('cancel');
    const previous = document.activeElement;
    $('modal-title').textContent = title; $('modal-message').textContent = message; $('modal-actions').replaceChildren();
    $('modal').hidden = false;
    return new Promise(resolve => {
      modalResolve = value => { $('modal').hidden = true; modalResolve = null; previous?.focus(); resolve(value); };
      for (const [value, label, type] of choices) $('modal-actions').append(button(label, () => modalResolve?.(value), type || ''));
      $('modal-actions').querySelector('button')?.focus();
    });
  }
  document.addEventListener('keydown', event => {
    if (!modalResolve) return;
    if (event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); modalResolve('cancel'); }
    else if (event.key === 'Tab') {
      const buttons = [...$('modal-actions').querySelectorAll('button')];
      let index = buttons.indexOf(document.activeElement);
      if (index < 0 || (!event.shiftKey && index === buttons.length - 1) || (event.shiftKey && index === 0)) { event.preventDefault(); buttons[event.shiftKey ? buttons.length - 1 : 0]?.focus(); }
    }
  });
  function splitWindows(percent, windows) {
    const left = percent / 100;
    return windows.map((w, i) => ({...w, geometry: {x: i ? left : 0, y: 0, width: i ? 1 - left : left, height: 1}}));
  }
  function splitPercent(profile) {
    const [a, b] = profile.windows.map(w => w.geometry), percent = Math.round(a.width * 100);
    const near = (a, b) => Math.abs(a - b) < 1e-9;
    return percent >= 1 && percent <= 99 && near(a.width * 100, percent) && near(a.x, 0) && near(a.y, 0) && near(b.y, 0) && near(a.height, 1) && near(b.height, 1) && near(b.x, a.width) && near(b.x + b.width, 1) ? percent : null;
  }
  function syncNativeDirty() {
    const dirty = state.dirty || KigiSystem.dirty();
    if (dirty !== state.nativeDirty) { state.nativeDirty = dirty; state.dirtySync = state.dirtySync.catch(() => {}).then(() => Kigi.request('app.setDraftDirty', {dirty})); state.dirtySync.catch(failure); }
    return state.dirtySync;
  }
  function dirtyChanged() {
    const dirty = !!state.draft && (!state.original || JSON.stringify(state.draft) !== JSON.stringify(state.original) || state.invalid.size > 0);
    state.dirty = dirty; syncNativeDirty();
    const indicator = $('draft-indicator'); if (indicator) indicator.hidden = !dirty;
    const save = $('save-space'); if (save) save.disabled = state.busy || !dirty || !state.data?.spaces.canWrite || state.invalid.size > 0;
    const apply = $('apply-space'); if (apply) apply.disabled = state.busy || !state.data?.spaces.canWrite || !!state.data?.diagnostics.restoreAvailable || state.invalid.size > 0 || !state.draft?.name.trim() || state.draft?.windows.some(w => !w.bundle_id);
    if (dirty) $('status').textContent = 'Modifications non enregistrées';
  }
  function resetDraft() {
    state.draft = null; state.original = null; state.invalid.clear(); state.rawRatio = {}; state.splitEdited = false; dirtyChanged();
  }
  async function discardDraft() { resetDraft(); await state.dirtySync; render(); $('status').textContent = 'Modifications ignorées'; }
  async function guardDraft() {
    if (state.busy || modalResolve) return false;
    if (KigiSystem.dirty() && !(await KigiSystem.guard())) return false;
    if (!state.dirty) return true;
    const choice = await choose('Modifications non enregistrées', 'Enregistrer cet espace avant de continuer ?', [['save', 'Enregistrer', 'primary'], ['discard', 'Ignorer'], ['cancel', 'Annuler']]);
    if (choice === 'save') return saveDraft();
    if (choice === 'discard') { await discardDraft(); return true; }
    return false;
  }
  function setEditorBusy(busy) {
    state.busy = busy;
    $('refresh').disabled = busy; $('recover-backup').disabled = busy; $('restore-layout').disabled = busy || !state.data?.diagnostics.restoreAvailable;
    $('content').setAttribute('aria-busy', String(busy));
    if (busy) document.querySelectorAll('#content button, #content input, #content select, #inspector button, #inspector input, #inspector select, #navigation button, #spaces-list button').forEach(n => { n.disabled = true; });
  }
  async function newSpace() {
    if (!(await guardDraft())) return;
    if (!state.data?.spaces.canWrite) { failure(new Error('Réparez ou rechargez le catalogue avant de créer un espace.')); return; }
    const existing = new Set(profiles().map(p => p.name.toLowerCase()));
    let name = 'Nouvel espace'; for (let n = 2; existing.has(name.toLowerCase()); n++) name = `Nouvel espace ${n}`;
    state.route = 'spaces'; state.selected = null; state.original = null; state.draftRevision = state.data.spaces.revision; state.splitEdited = true; state.invalid.clear();
    state.draft = {id: null, name, windows: splitWindows(50, [{bundle_id: ''}, {bundle_id: ''}])};
    render(); dirtyChanged(); $('space-name')?.focus();
  }
  async function recoverBackup() {
    if (!(await guardDraft())) return;
    if (await choose('Récupérer les espaces sauvegardés ?', 'La sauvegarde affichée deviendra le catalogue principal. Un éventuel fichier invalide sera conservé à côté et la sauvegarde restera intacte.', [['recover', 'Récupérer', 'primary'], ['cancel', 'Annuler']]) !== 'recover') return;
    setEditorBusy(true);
    try { await Kigi.request('spaces.recoverBackup', {expectedRevision: state.data.spaces.revision, confirmed: true}); state.data = await Kigi.request('app.bootstrap'); resetDraft(); $('error').hidden = true; $('recover-backup').hidden = true; $('status').textContent = 'Espaces récupérés · sauvegarde conservée'; }
    catch (error) { failure(error); }
    finally { setEditorBusy(false); render(); }
  }
  async function captureSpace() {
    if (state.busy || !state.data?.spaces.canWrite) return;
    if (state.dirty && await choose('Remplacer les positions du brouillon ?', 'La capture remplacera les deux fenêtres et leurs positions. Le nom et le brouillon restent non enregistrés.', [['capture', 'Capturer'], ['cancel', 'Annuler']]) !== 'capture') return;
    const previous = clone(state.draft), original = clone(state.original);
    const revision = previous ? state.draftRevision : state.data.spaces.revision;
    setEditorBusy(true);
    try {
      const result = await Kigi.request('spaces.capture', {expectedRevision: revision});
      if (result.canceled) { $('status').textContent = 'Capture annulée · aucune modification'; return; }
      const profile = previous || current();
      let name = 'Disposition capturée'; const existing = new Set(profiles().map(p => p.name.toLowerCase()));
      for (let n = 2; existing.has(name.toLowerCase()); n++) name = `Disposition capturée ${n}`;
      state.route = 'spaces'; state.original = previous ? original : clone(profile); state.selected = profile?.id ?? null;
      state.draft = {id: profile?.id ?? null, name: profile?.name || name, windows: result.windows};
      state.draftRevision = revision; state.splitEdited = false; state.invalid.clear();
      dirtyChanged(); $('error').hidden = true; $('status').textContent = 'Disposition capturée · cliquez sur Enregistrer';
    } catch (error) { failure(error); }
    finally { setEditorBusy(false); render(); }
  }
  async function saveDraft() {
    if (state.busy || !state.draft) return false;
    const draft = state.draft;
    if (state.invalid.size || !draft.name.trim() || [...draft.name.trim()].length > 60 || draft.windows.some(w => !w.bundle_id)) { failure(new Error('Choisissez un nom de 1 à 60 caractères, deux applications et une proportion de 1 à 99 %.')); return false; }
    setEditorBusy(true);
    try {
      await state.dirtySync;
      const result = await Kigi.request(draft.id == null ? 'spaces.create' : 'spaces.update', {profileId: draft.id, expectedRevision: state.draftRevision, name: draft.name.trim(), windows: draft.windows, leftPercent: state.splitEdited ? splitPercent(draft) : null});
      state.data.spaces = result.spaces; state.selected = result.profileId; resetDraft(); await state.dirtySync;
      $('error').hidden = true; $('status').textContent = 'Espace enregistré · aucune fenêtre déplacée';
      return true;
    } catch (error) { failure(error); return false; }
    finally { setEditorBusy(false); render(); }
  }
  async function mutateSpace(method) {
    if (!(await guardDraft())) return;
    const profile = current(); if (!profile) return;
    if (method === 'spaces.delete' && await choose('Supprimer cet espace ?', `« ${profile.name} » sera supprimé. Les fenêtres ouvertes et leur restauration sont conservées.`, [['delete', 'Supprimer', 'danger'], ['cancel', 'Annuler']]) !== 'delete') return;
    setEditorBusy(true);
    try {
      const params = {profileId: profile.id, expectedRevision: state.data.spaces.revision};
      if (method === 'spaces.delete') params.confirmed = true;
      const result = await Kigi.request(method, params);
      state.data.spaces = result.spaces; state.selected = result.profileId ?? profiles()[0]?.id ?? null; resetDraft(); await state.dirtySync;
      $('error').hidden = true; $('status').textContent = method === 'spaces.delete' ? 'Espace supprimé · fenêtres conservées' : 'Copie enregistrée';
    } catch (error) { failure(error); }
    finally { setEditorBusy(false); render(); }
  }
  async function chooseApp(index) {
    if (state.busy || !state.data.spaces.canWrite) return;
    setEditorBusy(true);
    try { const result = await Kigi.request('spaces.selectApp'); if (!result.canceled) { state.draft.windows[index].bundle_id = result.bundleId; appNames.set(result.bundleId, result.name); dirtyChanged(); } }
    catch (error) { failure(error); }
    finally { setEditorBusy(false); render(); }
  }
  function setRatio(percent) {
    if (!Number.isInteger(percent) || percent < 1 || percent > 99 || state.busy) return;
    state.invalid.delete('left'); state.invalid.delete('right'); state.rawRatio = {}; state.splitEdited = true;
    state.draft.windows = splitWindows(percent, state.draft.windows); dirtyChanged();
    const canvas = $('editor-preview');
    if (canvas) {
      canvas.querySelectorAll('.preview-window').forEach((w, i) => { const g = state.draft.windows[i].geometry; w.style.left = `${g.x * 100}%`; w.style.width = `${g.width * 100}%`; w.querySelector('.window-body small').textContent = `${i ? 100 - percent : percent} % de largeur`; });
      const handle = canvas.querySelector('.split-handle'); if (handle) { handle.style.left = `${percent}%`; handle.setAttribute('aria-valuenow', percent); handle.setAttribute('aria-valuetext', `${percent} pour cent à gauche, ${100 - percent} pour cent à droite`); }
    }
    if ($('left-percent')) { $('left-percent').value = percent; $('left-percent').setAttribute('aria-invalid', 'false'); }
    if ($('right-percent')) { $('right-percent').value = 100 - percent; $('right-percent').setAttribute('aria-invalid', 'false'); }
  }
  async function layoutAction(restore = false) {
    if (state.busy || modalResolve) return;
    if (!restore && state.dirty) {
      const choice = await choose('Enregistrer et appliquer ?', 'Le brouillon sera enregistré, puis ses fenêtres seront placées. La sauvegarde et le placement restent deux opérations distinctes.', [['save', 'Enregistrer et appliquer', 'primary'], ['cancel', 'Annuler']]);
      if (choice !== 'save' || !(await saveDraft())) return;
    }
    const profile = current(); if (!restore && !profile) return;
    setEditorBusy(true); state.layoutResult = null; $('layout-result')?.remove();
    $('status').textContent = restore ? 'Restauration en cours…' : 'Placement en cours…';
    try {
      const result = await Kigi.request(restore ? 'spaces.restore' : 'spaces.apply', restore ? {} : {profileId: profile.id, expectedRevision: state.data.spaces.revision});
      state.layoutResult = result;
      state.data.diagnostics.restoreAvailable = result.result.restoreAvailable;
      $('error').hidden = true; $('status').textContent = result.result.message;
    } catch (error) {
      failure(error);
      try { state.data.diagnostics = await Kigi.request('diagnostics.summary'); } catch (_) { /* Keep the last measured state; backend guards remain authoritative. */ }
    } finally { setEditorBusy(false); render(); }
  }
  function renderLayoutResult(content) {
    const last = state.layoutResult; if (!last) return;
    const result = last.result, card = el('div', `card layout-result ${result.status}`); card.id = 'layout-result'; card.dataset.status = result.status; card.dataset.operation = last.operation; card.setAttribute('role', 'status');
    card.append(el('h2', '', last.operation === 'restore' ? 'Dernière restauration' : `Dernier placement · ${last.profileName}`), el('p', '', result.message));
    if (last.operation === 'apply' && result.status !== 'canceled') {
      const widths = values => values.map(r => r == null ? 'Non mesuré' : `${+(r.width * 100).toFixed(1)} %`).join(' / ');
      field(card, 'Largeurs demandées · fenêtres 1 / 2', widths(result.requested));
      field(card, 'Largeurs constatées · fenêtres 1 / 2', widths(result.actual));
    }
    content.append(card);
  }
  function renderEditor(content, inspector) {
    if (!state.draft && current()) { state.draft = clone(current()); state.original = clone(current()); state.draftRevision = state.data.spaces.revision; state.splitEdited = false; state.invalid.clear(); }
    const draft = state.draft, writable = state.data.spaces.canWrite;
    const toolbar = el('div', 'editor-toolbar'); toolbar.append(button('Nouvel espace', newSpace, 'subtle')); const capture = button('Capturer deux fenêtres…', captureSpace, 'subtle'); capture.id = 'capture-space'; toolbar.append(capture);
    if (draft) {
      const save = button('Enregistrer', saveDraft, 'primary'); save.id = 'save-space'; toolbar.append(save); const apply = button('Appliquer', () => layoutAction(false)); apply.id = 'apply-space'; toolbar.append(apply);
      toolbar.append(button('Annuler les modifications', async () => { await discardDraft(); if (!current()) state.selected = profiles()[0]?.id ?? null; render(); $('status').textContent = 'Modifications ignorées'; }, 'subtle'));
    }
    toolbar.querySelectorAll('button').forEach(b => { b.disabled = !writable || state.busy; }); content.append(toolbar);
    heading(content, draft?.name || 'Vos espaces', 'L’aperçu et la sauvegarde ne déplacent aucune fenêtre.');
    const indicator = el('span', 'tag', 'Modifié'); indicator.id = 'draft-indicator'; indicator.hidden = !state.dirty; content.append(indicator); renderLayoutResult(content);
    if (!draft) { content.append(el('div', 'empty', 'Créez votre premier espace pour choisir ses applications et ses proportions.')); inspector.append(el('p', '', 'Aucun espace sélectionné.')); return; }
    const percent = splitPercent(draft), canvas = preview(draft); canvas.id = 'editor-preview'; content.append(canvas);
    canvas.querySelectorAll('.preview-window').forEach((window, i) => { if (!draft.windows[i].bundle_id) { window.querySelector('.window-bar').textContent = `Application ${i + 1}`; window.querySelector('.window-body strong').textContent = 'Choisir une app'; } });
    if (percent != null && writable) {
      const handle = el('div', 'split-handle'); handle.tabIndex = 0; handle.setAttribute('role', 'separator'); handle.setAttribute('aria-label', 'Proportion des deux fenêtres'); handle.setAttribute('aria-orientation', 'vertical'); handle.setAttribute('aria-valuemin', 1); handle.setAttribute('aria-valuemax', 99); handle.setAttribute('aria-valuenow', percent); handle.setAttribute('aria-valuetext', `${percent} pour cent à gauche, ${100 - percent} pour cent à droite`); handle.style.left = `${percent}%`; handle.append(el('span', 'split-grip', ''));
      let drag = null;
      handle.addEventListener('pointerdown', event => { if (event.button !== 0 || state.busy) return; event.preventDefault(); drag = {id: event.pointerId, original: splitPercent(state.draft)}; handle.setPointerCapture(event.pointerId); handle.focus(); });
      handle.addEventListener('pointermove', event => { if (drag?.id !== event.pointerId) return; const rect = canvas.getBoundingClientRect(); setRatio(Math.max(1, Math.min(99, Math.round((event.clientX - rect.left) * 100 / rect.width)))); });
      const end = event => { if (drag?.id === event.pointerId) { drag = null; if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId); } };
      handle.addEventListener('pointerup', end); handle.addEventListener('pointercancel', end); handle.addEventListener('lostpointercapture', () => { drag = null; });
      handle.addEventListener('keydown', event => {
        let next = splitPercent(state.draft), step = event.shiftKey ? 5 : 1;
        if (event.key === 'ArrowLeft') next -= step; else if (event.key === 'ArrowRight') next += step; else if (event.key === 'Home') next = 1; else if (event.key === 'End') next = 99; else if (event.key === 'Escape' && drag) { next = drag.original; drag = null; } else return;
        event.preventDefault(); setRatio(Math.max(1, Math.min(99, next)));
      }); canvas.append(handle);
      const presets = el('div', 'presets'); for (const p of [50, 35, 70, 30]) { const preset = button(`${p} / ${100 - p}`, () => setRatio(p), 'subtle'); preset.dataset.percent = p; presets.append(preset); } content.append(presets);
    } else if (percent == null) {
      const free = el('div', 'card'); free.style.marginTop = '20px'; free.append(el('h2', '', 'Disposition enregistrée'), el('p', '', 'Les positions libres sont conservées. Passez explicitement en deux colonnes pour modifier le partage.'));
      const convert = button('Passer en deux colonnes', () => { state.draft.windows = splitWindows(50, state.draft.windows); state.splitEdited = true; dirtyChanged(); render(); }); convert.disabled = !writable; free.append(convert); content.append(free);
    }
    const tip = el('div', 'preview-caption'); tip.append(el('span', '', percent == null ? 'Positions d’origine conservées' : 'Glissez le séparateur ou utilisez les flèches du clavier'), el('span', '', '2 fenêtres')); content.append(tip);
    const nameField = el('div', 'field'), nameLabel = el('label', '', 'Nom de l’espace'), nameInput = el('input'); nameLabel.htmlFor = 'space-name'; nameInput.id = 'space-name'; nameInput.value = draft.name; nameInput.disabled = !writable;
    nameInput.addEventListener('input', () => { state.draft.name = nameInput.value; if (nameInput.value.trim() && [...nameInput.value.trim()].length <= 60 && !/[\u0000-\u001f\u007f]/.test(nameInput.value)) state.invalid.delete('name'); else state.invalid.add('name'); $('page-title').textContent = nameInput.value || 'Nouvel espace'; dirtyChanged(); }); nameField.append(nameLabel, nameInput); inspector.append(nameField);
    draft.windows.forEach((window, i) => { const group = el('div', 'field'); group.append(el('label', '', percent == null ? `Application ${i + 1}` : i ? 'Application à droite' : 'Application à gauche')); const choose = button(appNames.get(window.bundle_id) || names(window.bundle_id) || 'Choisir une application…', () => chooseApp(i), 'app-picker'); choose.dataset.appIndex = i; choose.disabled = !writable; group.append(choose); inspector.append(group); });
    if (percent != null) {
      const ratios = el('div', 'ratio-fields');
      for (const [side, label, value] of [['left', 'Gauche (%)', percent], ['right', 'Droite (%)', 100 - percent]]) {
        const group = el('div', 'field'), input = el('input'), text = el('label', '', label); input.id = `${side}-percent`; text.htmlFor = input.id; input.type = 'number'; input.min = '1'; input.max = '99'; input.step = '1'; input.value = state.invalid.has(side) ? (state.rawRatio[side] ?? '') : value; input.setAttribute('aria-invalid', String(state.invalid.has(side))); input.disabled = !writable;
        input.addEventListener('input', () => { const value = Number(input.value); if (input.value && input.validity.valid && Number.isInteger(value)) { state.invalid.delete(side); setRatio(side === 'left' ? value : 100 - value); } else { state.rawRatio[side] = input.value; state.invalid.add(side); input.setAttribute('aria-invalid', 'true'); dirtyChanged(); } }); group.append(text, input); ratios.append(group);
      } inspector.append(ratios);
    }
    if (draft.id != null) { const actions = el('div', 'inspector-actions'); const duplicate = button('Dupliquer', () => mutateSpace('spaces.duplicate')); const remove = button('Supprimer', () => mutateSpace('spaces.delete'), 'danger'); duplicate.disabled = remove.disabled = !writable; actions.append(duplicate, remove); inspector.append(actions); }
    inspector.append(el('div', 'rule'), el('p', '', state.data.diagnostics.restoreAvailable ? 'Une disposition temporaire est active. Restaurez-la avant un nouveau placement.' : 'Appliquer déplace les fenêtres ouvertes et crée un retour de restauration.'));
    dirtyChanged();
  }
  window.KigiEditor = {async requestClose(quit) { try { if (await guardDraft()) { await state.dirtySync; KigiSystem.stopPolling(); await Kigi.request(quit ? 'app.quit' : 'app.closeStudio'); } } catch (error) { failure(error); } }};
  function render() {
    if (!state.data) return;
    $('restore-layout').hidden = !state.data.diagnostics.restoreAvailable; $('restore-layout').disabled = state.busy || !state.data.diagnostics.restoreAvailable;
    const data = state.data, content = $('content'), inspector = $('inspector');
    content.replaceChildren(); inspector.replaceChildren(); $('navigation').replaceChildren(); $('spaces-list').replaceChildren();
    for (const [route, label, icon] of routes) {
      const item = button('', () => navigate(route), state.route === route ? 'active' : ''); item.append(el('span', 'nav-icon', icon), el('span', '', label)); item.dataset.route = route; item.setAttribute('aria-current', state.route === route ? 'page' : 'false'); $('navigation').append(item);
    }
    for (const profile of profiles()) { const item = button('', () => navigate('spaces', profile.id), state.route === 'spaces' && state.selected === profile.id ? 'active' : ''); item.append(el('span', 'space-marker', '▣'), el('span', '', profile.name)); $('spaces-list').append(item); }
    if (!profiles().length) $('spaces-list').append(el('span', 'muted', 'Aucun espace enregistré'));
    $('crumb').textContent = routes.find(r => r[0] === state.route)[1];
    inspector.append(el('h2', '', state.route === 'spaces' && current() ? 'ESPACE SÉLECTIONNÉ' : 'ÉTAT DU MOTEUR'));
    if (state.route === 'home') {
      const hero = el('div', 'hero'); hero.append(el('div', 'eyebrow', 'VOTRE BUREAU, À VOTRE FAÇON')); heading(hero, 'Un espace pour chaque moment.', 'Retrouvez vos dispositions et gardez un œil sur les fonctions de Kigi.'); hero.append(button('Parcourir les espaces', () => navigate('spaces'), 'primary')); content.append(hero);
      const section = el('div', 'section-heading'); section.append(el('h2', '', 'Vos espaces'), button('Nouvel espace', newSpace, 'subtle')); content.append(section);
      if (!profiles().length) { const empty = el('div', 'empty'); empty.append(el('p', '', 'Créez un espace et choisissez ses deux applications.'), button('Créer un espace', newSpace, 'primary')); content.append(empty); }
      else profiles().forEach(p => content.append(card(p)));
    } else if (state.route === 'spaces') {
      renderEditor(content, inspector);
    } else { KigiSystem.render(state.route, content, inspector); }
    if (state.route === 'home') {
      field(inspector, 'Accessibilité', permissionName(data.permissions.accessibility), `state ${data.permissions.accessibility}`);
      field(inspector, 'Raccourcis', data.diagnostics.shortcutConflict ? 'Conflit à vérifier' : data.diagnostics.shortcutsEnabled ? 'Activation demandée' : 'Désactivés');
      field(inspector, 'Gestes du trackpad', data.diagnostics.gesturesActive ? 'Pilote actif' : 'Pilote arrêté');
      field(inspector, 'Filtre horizontal souris', data.diagnostics.mouseFilterActive ? 'Actif' : data.diagnostics.mouseFilterRequested ? 'Demandé, actuellement inactif' : 'Désactivé');
      inspector.append(el('div', 'rule'), el('p', '', 'Fermer cette fenêtre conserve les fonctions en arrière-plan. ⌘Q quitte Kigi.'));
    }
  }
  function properties(open) { $('inspector').classList.toggle('open', open); $('properties').setAttribute('aria-expanded', String(open)); }
  $('properties').addEventListener('click', () => properties(!$('inspector').classList.contains('open')));
  document.addEventListener('keydown', event => { if (event.key === 'Escape' && $('inspector').classList.contains('open')) { properties(false); $('properties').focus(); } });
  KigiSystem.setup({el, button, field, heading, choose, fail: failure, guard: guardDraft, syncDirty: syncNativeDirty, busy: () => state.busy, setBusy: setEditorBusy, data: () => state.data, route: () => state.route, replace: data => { state.data = data; }, render, status: text => { $('status').textContent = text; }, clearError: () => { if (!state.data?.spaces.usingBackup) $('error').hidden = true; }});
  $('restore-layout').addEventListener('click', () => layoutAction(true));
  $('refresh').addEventListener('click', refresh); $('recover-backup').addEventListener('click', recoverBackup);
  refresh();
})();
