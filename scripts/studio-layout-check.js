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
    let captureGeometry = null, captureBundles = null;
    if (!minimum) {
      q('#capture-space').click(); await wait(() => q('#status').textContent.startsWith('Capture annulée') && !q('#refresh').disabled);
      assert(JSON.stringify((await Kigi.request('spaces.list')).profiles) === JSON.stringify(initial.profiles), 'Canceled capture wrote catalog');
      q('#capture-space').click(); await wait(() => q('#status').textContent.startsWith('Disposition capturée') && !q('#refresh').disabled);
      assert(!q('#draft-indicator').hidden && !(await Kigi.request('diagnostics.summary')).restoreAvailable, 'Capture moved windows or failed to create draft');
      captureGeometry = [...document.querySelectorAll('#editor-preview .preview-window')].map(w => Object.fromEntries(['left','top','width','height'].map(key=>[key,parseFloat(w.style[key])/100])));
      captureBundles = [...document.querySelectorAll('#editor-preview .window-bar')].map(w => w.textContent);
      assert(JSON.stringify((await Kigi.request('spaces.list')).profiles) === JSON.stringify(initial.profiles), 'Successful capture saved implicitly');
      [...document.querySelectorAll('button')].find(b=>b.textContent==='Annuler les modifications').click();
      await wait(() => !!q('#left-percent'));
      const input = q('#left-percent'); input.value = '35'; input.dispatchEvent(new Event('input', {bubbles: true}));
      q('#preview-space').click();
      await wait(() => q('#layout-result')?.dataset.operation === 'preview' && !q('#refresh').disabled);
      assert(q('#layout-result').dataset.status === 'applied', 'Draft preview was not applied');
      assert(JSON.stringify((await Kigi.request('spaces.list')).profiles) === JSON.stringify(initial.profiles), 'Preview wrote the catalog');
      assert(q('#left-percent').value === '35' && !q('#draft-indicator').hidden && q('#preview-space').disabled, 'Preview lost draft or snapshot guard');
      let previewRefused=false;
      try { await Kigi.request('spaces.previewApply',{expectedRevision:initial.revision,profileId:initial.profiles[0].id,name:initial.profiles[0].name,windows:initial.profiles[0].windows,leftPercent:null}); }catch(e){previewRefused=e.code==='SNAPSHOT_ACTIVE';}
      assert(previewRefused,'Preview replaced the existing snapshot');
      await new Promise(resolve => setTimeout(resolve,750));
      q('#restore-layout').click();
      await wait(() => q('#layout-result')?.dataset.operation === 'restore' && !q('#refresh').disabled);
      assert(q('#layout-result').dataset.status === 'restored' && q('#left-percent').value === '35' && !q('#draft-indicator').hidden,'Preview restore lost draft');
      assert(JSON.stringify((await Kigi.request('spaces.list')).profiles) === JSON.stringify(initial.profiles),'Preview restore wrote catalog');
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
    window.__kigiEditorCheck = {success: true, captureGeometry, captureBundles, previewChecked: !minimum, minimum, ratio: .35, restored: true, reportedWidths};
  } catch (error) { window.__kigiEditorCheck = {success: false, message: String(error.message)}; }
})();
