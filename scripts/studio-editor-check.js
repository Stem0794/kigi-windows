// Fixed, native-CI-only DOM flow. No synthetic system input or third-party windows.
(async () => {
  const q = selector => document.querySelector(selector);
  const assert = (value, label) => { if (!value) throw new Error(label); };
  const wait = async predicate => { await new Promise(resolve => setTimeout(resolve, 0)); for (let i = 0; i < 100; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 30)); } throw new Error(`UI wait timed out: ${String(predicate).slice(0,100)}; status=${q('#status')?.textContent}; error=${q('#error')?.textContent}; left=${q('#left-percent')?.value}; cancelDisabled=${[...document.querySelectorAll('button')].find(b=>b.textContent==='Annuler les modifications')?.disabled}`); };
  const clickText = text => { const node = [...document.querySelectorAll('button')].find(n => n.textContent === text); assert(node, `Missing ${text}`); node.click(); };
  const input = (id, value) => { const node = q(id); assert(node, `Missing ${id}`); node.value = value; node.dispatchEvent(new Event('input', {bubbles: true})); };
  try {
    document.querySelectorAll('#navigation button')[1].click();
    await wait(() => !!q('#space-name'));
    assert(q('#page-title').textContent === 'Disposition personnelle', 'Schema1 profile missing');
    assert(!q('[role="separator"]'), 'Free layout silently converted');
    clickText('Passer en deux colonnes');
    await wait(() => !!q('[role="separator"]'));
    input('#space-name', 'Ratio35');
    clickText('35 / 65');
    const before = await Kigi.request('spaces.list');
    assert(before.profiles[0].windows[0].geometry.x === .1, 'Preview wrote config');
    q('#save-space').click();
    await wait(() => q('#status').textContent.startsWith('Espace enregistré'));
    let listing = await Kigi.request('spaces.list');
    assert(listing.profiles[0].windows[0].geometry.width === .35, '35/65 not saved');
    input('#left-percent', '49');
    assert(q('#right-percent').value === '51', 'Numeric fields not linked');
    clickText('Annuler les modifications');
    await wait(() => q('#left-percent')?.value === '35');
    assert((await Kigi.request('spaces.list')).profiles[0].windows[0].geometry.width === .35, 'Cancel wrote config');
    q('[role="separator"]').dispatchEvent(new KeyboardEvent('keydown', {key: 'ArrowRight', bubbles: true}));
    assert(q('#left-percent').value === '36', 'Keyboard separator not working');
    document.querySelectorAll('#navigation button')[0].click();
    await wait(() => !q('#modal').hidden); clickText('Annuler');
    assert(q('#left-percent').value === '36', 'Canceled navigation lost draft');
    document.querySelectorAll('#navigation button')[0].click();
    await wait(() => !q('#modal').hidden); clickText('Ignorer');
    await wait(() => q('#page-title').textContent === 'Un espace pour chaque moment.');
    document.querySelectorAll('#navigation button')[1].click();
    await wait(() => q('#left-percent')?.value === '35');
    clickText('Dupliquer'); await wait(() => document.querySelectorAll('#spaces-list button').length === 2 && q('#status').textContent === 'Copie enregistrée');
    clickText('Supprimer'); await wait(() => !q('#modal').hidden); clickText('Annuler');
    assert(document.querySelectorAll('#spaces-list button').length === 2, 'Canceled delete applied');
    clickText('Supprimer'); await wait(() => !q('#modal').hidden); q('#modal-actions .danger').click();
    await wait(() => document.querySelectorAll('#spaces-list button').length === 1);
    listing = await Kigi.request('spaces.list');
    const profile = listing.profiles[0];
    // Backend mutation represents another settings view; the UI keeps its old revision.
    await Kigi.request('spaces.update', {profileId: profile.id, expectedRevision: listing.revision, name: 'Ratio35', windows: profile.windows, leftPercent: null});
    // A no-op write has identical bytes, so force a real catalog change before conflict.
    listing = await Kigi.request('spaces.list');
    await Kigi.request('spaces.update', {profileId: profile.id, expectedRevision: listing.revision, name: 'Ratio35 externe', windows: profile.windows, leftPercent: null});
    input('#space-name', 'Brouillon conservé'); q('#save-space').click();
    await wait(() => !q('#error').hidden && !q('#refresh').disabled);
    assert(q('#space-name').value === 'Brouillon conservé', 'Conflict discarded draft');
    assert((await Kigi.request('spaces.list')).profiles[0].name === 'Ratio35 externe', 'Conflict overwrote backend');
    q('#refresh').click(); await wait(() => !q('#modal').hidden); clickText('Ignorer');
    await wait(() => q('#space-name')?.value === 'Ratio35 externe');
    input('#space-name', 'Ratio35'); q('#save-space').click(); await wait(() => q('#status').textContent.startsWith('Espace enregistré'));
    listing = await Kigi.request('spaces.list');
    const created = await Kigi.request('spaces.create', {profileId: null, expectedRevision: listing.revision, name: 'Temporaire', windows: listing.profiles[0].windows, leftPercent: 35});
    let rejected = false;
    try { await Kigi.request('spaces.delete', {profileId: created.profileId, expectedRevision: created.spaces.revision, confirmed: false}); } catch (error) { rejected = error.code === 'CONFIRMATION_REQUIRED'; }
    assert(rejected, 'Backend accepted unconfirmed deletion');
    await Kigi.request('spaces.delete', {profileId: created.profileId, expectedRevision: created.spaces.revision, confirmed: true});
    q('#refresh').click(); await wait(() => !q('#refresh').disabled);
    assert(![...document.querySelectorAll('button')].find(b => b.textContent === 'Annuler les modifications').disabled, 'Refresh left editor controls disabled');
    input('#left-percent', '49');
    window.KigiEditor.requestClose(false);
    await wait(() => !q('#modal').hidden); clickText('Annuler');
    assert(q('#left-percent').value === '49', 'Cancel close lost draft');
    clickText('Annuler les modifications'); await wait(() => q('#left-percent')?.value === '35');
    listing = await Kigi.request('spaces.list');
    assert(listing.profiles.length === 1 && listing.profiles[0].name === 'Ratio35', 'Unexpected final catalog');
    window.__kigiEditorCheck = {success: true, width: listing.profiles[0].windows[0].geometry.width, schema: 2};
  } catch (error) { window.__kigiEditorCheck = {success: false, message: String(error.message)}; }
})();
