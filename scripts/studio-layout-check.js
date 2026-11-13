// Native-CI-only real AX flow against two dedicated fixture apps; no system key injection.
(async () => {
  const q = selector => document.querySelector(selector);
  const assert = (condition, label) => { if (!condition) throw new Error(label); };
  const wait = async predicate => { await new Promise(resolve => setTimeout(resolve, 0)); for (let i = 0; i < 100; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 30)); } throw new Error(`UI wait timed out: ${predicate}; error=${q('#error')?.textContent}`); };
  const clickModal = text => { const button = [...document.querySelectorAll('#modal-actions button')].find(b => b.textContent === text); assert(button, `Missing modal ${text}`); button.click(); };
  try {
    const initial = await Kigi.request('spaces.list');
    const minimum = initial.profiles[0].name === 'Taille minimale';
    document.querySelectorAll('#navigation button')[1].click();
    await wait(() => !!q('#apply-space'));
    if (!minimum) {
      const input = q('#left-percent'); input.value = '35'; input.dispatchEvent(new Event('input', {bubbles: true}));
      q('#apply-space').click(); await wait(() => !q('#modal').hidden); clickModal('Annuler');
      assert(!(await Kigi.request('diagnostics.summary')).restoreAvailable, 'Canceled apply changed windows');
      q('#apply-space').click(); await wait(() => !q('#modal').hidden); clickModal('Enregistrer et appliquer');
    } else { q('#apply-space').click(); }
    await wait(() => q('#layout-result')?.dataset.operation === 'apply' && !q('#refresh').disabled);
    const snapshot = await Kigi.request('diagnostics.summary'); assert(snapshot.restoreAvailable, 'Missing return point');
    assert(q('#apply-space').disabled && !q('#restore-layout').hidden && !q('#restore-layout').disabled, 'Snapshot controls incoherent');
    assert(q('#layout-result').dataset.status === (minimum ? 'partial' : 'applied'), `Wrong apply status; ${q('#layout-result')?.textContent}`);
    const text = q('#layout-result').textContent; assert(text.includes('35 %'), 'Requested split not reported');
    const reportedWidths = q('#layout-result').querySelectorAll('dd')[1].textContent.split(' / ').map(value => parseFloat(value));
    assert(reportedWidths.every(Number.isFinite), 'Fixture geometry was not measured');
    let refused = false;
    const saved = await Kigi.request('spaces.list');
    try { await Kigi.request('spaces.apply', {profileId: saved.profiles[0].id, expectedRevision: saved.revision}); } catch (error) { refused = error.code === 'SNAPSHOT_ACTIVE'; }
    assert(refused, 'Existing return point overwritten');
    // Keep the controlled placement visible long enough for independent CG bounds sampling.
    await new Promise(resolve => setTimeout(resolve, 750));
    q('#restore-layout').click();
    await wait(() => q('#layout-result')?.dataset.operation === 'restore' && !q('#refresh').disabled);
    assert(q('#layout-result').dataset.status === 'restored', `Restore failed: ${q('#layout-result').textContent}`);
    assert(!(await Kigi.request('diagnostics.summary')).restoreAvailable && q('#restore-layout').hidden, 'Return point not cleared');
    assert(!q('#apply-space').disabled, 'Apply not reenabled after restore');
    const final = await Kigi.request('spaces.list'); assert(final.profiles[0].windows[0].geometry.width === .35, 'Apply/restore changed requested ratio');
    window.__kigiEditorCheck = {success: true, minimum, ratio: .35, restored: true, reportedWidths};
  } catch (error) { window.__kigiEditorCheck = {success: false, message: String(error.message)}; }
})();
