'use strict';
(() => {
  const $ = id => document.getElementById(id);
  const state = {data: null, route: 'home', selected: null, busy: false};
  const routes = [['home', 'Accueil', '⌂'], ['spaces', 'Espaces', '▤'], ['permissions', 'Autorisations', '◇'], ['diagnostics', 'Diagnostics', '≋']];
  const el = (tag, className, text) => { const node = document.createElement(tag); if (className) node.className = className; if (text !== undefined) node.textContent = text; return node; };
  const button = (label, action, className = '') => { const node = el('button', className, label); node.type = 'button'; node.addEventListener('click', action); return node; };
  const names = bundle => ({'com.google.Chrome': 'Google Chrome', 'com.microsoft.teams2': 'Microsoft Teams', 'com.microsoft.teams': 'Microsoft Teams', 'com.apple.Safari': 'Safari'})[bundle] || bundle;
  const permissionName = value => ({granted: 'Accordée', denied: 'Refusée / indisponible', unknown: 'Non vérifiée'})[value] || 'Non vérifiée';
  const profiles = () => state.data?.spaces.profiles || [];
  const current = () => profiles().find(p => p.id === state.selected) || null;
  function failure(error) { $('error').textContent = error.message; $('error').hidden = false; $('status').textContent = 'Actualisation nécessaire'; }
  async function native() { try { await Kigi.request('app.openNativeFallback'); } catch (error) { failure(error); } }
  async function refresh() {
    if (state.busy) return;
    state.busy = true; $('refresh').disabled = true; $('content').setAttribute('aria-busy', 'true');
    try {
      const data = await Kigi.request('app.bootstrap');
      state.data = data;
      if (!profiles().some(p => p.id === state.selected)) state.selected = profiles()[0]?.id ?? null;
      $('error').hidden = true;
      if (data.spaces.usingBackup) failure(new Error('Le catalogue principal est illisible. Les espaces de sauvegarde sont affichés ; ouvrez les réglages classiques pour les vérifier.'));
      render();
      $('status').textContent = data.spaces.usingBackup ? 'Catalogue de sauvegarde · lecture seule' : 'Lecture du moteur terminée';
      $('version').textContent = `Kigi ${data.version} · Studio`;
    } catch (error) { failure(error); if (!state.data) $('content').replaceChildren(el('p', '', 'Données indisponibles. Actualisez ou ouvrez les réglages classiques.')); }
    finally { state.busy = false; $('refresh').disabled = false; $('content').setAttribute('aria-busy', 'false'); }
  }
  function navigate(route, id = state.selected) { state.route = route; state.selected = id; render(); $('page-title')?.focus(); }
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
  function render() {
    if (!state.data) return;
    const data = state.data, content = $('content'), inspector = $('inspector');
    content.replaceChildren(); inspector.replaceChildren(); $('navigation').replaceChildren(); $('spaces-list').replaceChildren();
    for (const [route, label, icon] of routes) {
      const item = button('', () => navigate(route), state.route === route ? 'active' : ''); item.append(el('span', 'nav-icon', icon), el('span', '', label)); item.setAttribute('aria-current', state.route === route ? 'page' : 'false'); $('navigation').append(item);
    }
    for (const profile of profiles()) { const item = button('', () => navigate('spaces', profile.id), state.route === 'spaces' && state.selected === profile.id ? 'active' : ''); item.append(el('span', 'space-marker', '▣'), el('span', '', profile.name)); $('spaces-list').append(item); }
    if (!profiles().length) $('spaces-list').append(el('span', 'muted', 'Aucun espace enregistré'));
    $('crumb').textContent = routes.find(r => r[0] === state.route)[1];
    inspector.append(el('h2', '', state.route === 'spaces' && current() ? 'ESPACE SÉLECTIONNÉ' : 'ÉTAT DU MOTEUR'));
    if (state.route === 'home') {
      const hero = el('div', 'hero'); hero.append(el('div', 'eyebrow', 'VOTRE BUREAU, À VOTRE FAÇON')); heading(hero, 'Un espace pour chaque moment.', 'Retrouvez vos dispositions et gardez un œil sur les fonctions de Kigi.'); hero.append(button('Parcourir les espaces', () => navigate('spaces'), 'primary')); content.append(hero);
      const section = el('div', 'section-heading'); section.append(el('h2', '', 'Vos espaces'), el('span', 'tag', `${profiles().length} enregistrés`)); content.append(section);
      if (!profiles().length) { const empty = el('div', 'empty'); empty.append(el('p', '', 'Votre premier espace commence dans les réglages classiques.'), button('Ouvrir les réglages', native)); content.append(empty); }
      else profiles().forEach(p => content.append(card(p)));
    } else if (state.route === 'spaces') {
      const profile = current(); heading(content, profile?.name || 'Vos espaces', 'Aperçu de la disposition enregistrée. Les fenêtres restent à leur place.');
      if (profile) { content.append(preview(profile)); const caption = el('div', 'preview-caption'); caption.append(el('span', '', 'Zone visible de l’écran'), el('span', '', '2 fenêtres')); content.append(caption); }
      else content.append(el('div', 'empty', 'Aucun espace enregistré.'));
      const note = el('div', 'card'); note.style.marginTop = '28px'; note.append(el('h2', '', 'Gérer cet espace'), el('p', '', 'La modification, l’application et la restauration restent disponibles dans les réglages classiques.'), button('Ouvrir les réglages', native)); content.append(note);
      if (profile) { field(inspector, 'Nom', profile.name); profile.windows.forEach((w, i) => { field(inspector, `Application ${i + 1}`, names(w.bundle_id)); const g = w.geometry; field(inspector, 'Position / taille enregistrées', `X ${+(g.x * 100).toFixed(2)} % · Y ${+(g.y * 100).toFixed(2)} % · Largeur ${+(g.width * 100).toFixed(2)} % · Hauteur ${+(g.height * 100).toFixed(2)} %`); }); inspector.append(el('div', 'rule'), el('p', '', 'Lecture seule. Les proportions et positions enregistrées sont conservées.')); }
    } else if (state.route === 'permissions') {
      heading(content, 'Autorisations', 'Les droits sont lus depuis macOS, sans déclencher de demande.');
      for (const [key, label, description] of [['accessibility', 'Accessibilité', 'Déplacement des fenêtres et passage au premier plan.'], ['inputMonitoring', 'Surveillance de l’entrée', 'Réception des événements du trackpad.'], ['chromeAutomation', 'Automatisation Chrome', 'Réutilisation des onglets ChatGPT. Cet accès reste non vérifié dans Studio.']]) {
        const row = el('div', 'permission'), head = el('div', 'card-head'); head.append(el('h2', '', label), el('span', `state ${data.permissions[key]}`, permissionName(data.permissions[key]))); row.append(head, el('p', '', description)); content.append(row);
      }
      const card = el('div', 'card'); card.style.marginTop = '24px'; card.append(el('p', '', 'Demandez un accès ou ouvrez Réglages Système depuis les réglages classiques.'), button('Gérer les autorisations', native)); content.append(card);
    } else {
      heading(content, 'Diagnostics', 'Mesures de cette session. Utilisez Actualiser après un essai.');
      const stats = el('div', 'stats'); for (const [label, bytes] of [['Mémoire Rust actuelle', data.diagnostics.rustRssBytes], ['Pic mémoire Rust', data.diagnostics.rustPeakRssBytes]]) { const stat = el('div', 'stat'); stat.append(el('small', '', label), el('strong', '', bytes == null ? 'Non mesuré' : `${(bytes / 1048576).toFixed(1)} Mio`)); stats.append(stat); } content.append(stats);
      const report = el('pre', 'report', data.diagnostics.performance); content.append(report, el('p', '', 'La mémoire des processus auxiliaires WebKit n’est pas mesurée. Les valeurs ci-dessus concernent le processus Rust.'), button('Ouvrir les réglages classiques', native));
    }
    if (state.route !== 'spaces' || !current()) {
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
  $('refresh').addEventListener('click', refresh); $('native').addEventListener('click', native);
  refresh();
})();
