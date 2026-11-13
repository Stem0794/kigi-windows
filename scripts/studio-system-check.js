// Fixed native CI flow. No system keystroke injection; capture is canceled through its native path.
(async () => {
  const q = selector => document.querySelector(selector);
  const assert = (value, label) => { if (!value) throw new Error(label); };
  const wait = async predicate => { await new Promise(resolve => setTimeout(resolve, 0)); for (let i = 0; i < 100; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 30)); } throw new Error(`UI wait timed out: ${predicate}; error=${q('#error')?.textContent}`); };
  try {
    q('#navigation [data-route="shortcuts"]').click(); await wait(() => !!q('#shortcut-command'));
    const command = q('#shortcut-command'); command.value = 'Maximize'; command.dispatchEvent(new Event('change', {bubbles: true}));
    q('#save-shortcut').click(); await wait(() => !q('#refresh').disabled && q('#save-shortcut').disabled);
    let listing = await Kigi.request('shortcuts.list'); assert(listing.settings.entries[0].command.kind === 'Maximize', 'UI command not saved');
    const original = JSON.stringify(listing.settings);
    const duplicate = JSON.parse(JSON.stringify(listing.settings.entries[1]));
    let refused = false;
    try { await Kigi.request('shortcuts.upsert', {expectedRevision: listing.revision, index: 0, binding: duplicate}); } catch (_) { refused = true; }
    assert(refused && JSON.stringify((await Kigi.request('shortcuts.list')).settings) === original, 'Duplicate mapping replaced old settings');
    const blocked = {enabled: true, key: 18, modifiers: 6400, key_label: '1', command: {kind: 'Maximize'}};
    refused = false;
    try { await Kigi.request('shortcuts.upsert', {expectedRevision: listing.revision, index: 0, binding: blocked}); } catch (error) { refused = error.code === 'SHORTCUT_CONFLICT'; }
    listing = await Kigi.request('shortcuts.list');
    assert(refused && JSON.stringify(listing.settings) === original && listing.registered && !listing.registrationFailed, `Carbon conflict rollback failed: ${JSON.stringify({refused, unchanged: JSON.stringify(listing.settings) === original, registered: listing.registered, failed: listing.registrationFailed, report: listing.registrationReport})}`);
    // Native Recorder modal is opened, then its own cancel routine restores Carbon bindings.
    const pending = Kigi.request('shortcuts.startCapture');
    await new Promise(resolve => setTimeout(resolve, 250));
    const recording = await Kigi.request('shortcuts.list'); assert(recording.capturing && !recording.registered, 'Native recorder was not focused or globals were not suspended');
    await Kigi.request('shortcuts.cancelCapture');
    const captured = await pending; assert(captured.canceled && captured.registrationRestored, 'Capture cancellation did not restore bindings');
    listing = await Kigi.request('shortcuts.list'); assert(!listing.capturing && listing.registered, 'Capture still active');
    // This second capture receives a fixed NSEvent directly in our Recorder on the native side.
    const captureStarted = performance.now();
    q('#capture-shortcut').click();
    await wait(() => q('#status').textContent.includes('capturée. Cliquez sur Enregistrer'));
    assert(performance.now() - captureStarted < 2500 && q('#capture-shortcut').textContent === '⌥N' && !q('#save-shortcut').disabled, 'Successful capture did not close its modal or update the draft promptly');
    listing = await Kigi.request('shortcuts.list'); assert(!listing.capturing && listing.registered && JSON.stringify(listing.settings) === original, 'Capture persisted without Save or left globals disabled');
    q('#refresh').click(); await wait(() => !q('#modal').hidden);
    [...q('#modal-actions').querySelectorAll('button')].find(b => b.textContent === 'Ignorer').click();
    await wait(() => !q('#refresh').disabled && q('#save-shortcut').disabled);
    for (const kind of ['Work', 'Meeting', 'Restore', 'ApplyProfile']) assert([...q('#shortcut-command').options].some(o => o.value === kind), `Missing action ${kind}`);
    const disabled = await Kigi.request('shortcuts.setGlobalEnabled', {enabled: false}); assert(!disabled.globalEnabled, 'Global disable failed');
    const enabled = await Kigi.request('shortcuts.setGlobalEnabled', {enabled: true}); assert(enabled.globalEnabled && enabled.registered, 'Global reenable failed');
    const count = enabled.settings.entries.length;
    const added = await Kigi.request('shortcuts.upsert', {expectedRevision: enabled.revision, index: null, binding: {enabled: false, key: 45, modifiers: 2048, key_label: 'N', command: {kind: 'Maximize'}}});
    assert(added.settings.entries.length === count + 1, 'Add failed');
    refused = false; try { await Kigi.request('shortcuts.delete', {expectedRevision: added.revision, index: count, confirmed: false}); } catch (error) { refused = error.code === 'CONFIRMATION_REQUIRED'; }
    assert(refused, 'Unconfirmed delete accepted');
    const removed = await Kigi.request('shortcuts.delete', {expectedRevision: added.revision, index: count, confirmed: true}); assert(removed.settings.entries.length === count, 'Delete failed');
    q('#refresh').click(); await wait(() => !q('#refresh').disabled);
    for (const route of ['gestures', 'mouse', 'permissions', 'diagnostics']) { q(`#navigation [data-route="${route}"]`).click(); await wait(() => q('#crumb').textContent === ({gestures: 'Trackpad', mouse: 'Souris', permissions: 'Autorisations', diagnostics: 'Diagnostics'})[route]); }
    const gestures = await Kigi.request('gestures.get'); assert(!gestures.active && gestures.counts == null, 'Fake gesture receipt at rest');
    const stopped = await Kigi.request('gestures.setEnabled', {enabled: false}); assert(!stopped.active, 'Gesture stop failed');
    const mouse = await Kigi.request('mouse.setHorizontalBlocked', {enabled: false}); assert(!mouse.active && !mouse.requested, 'Mouse disable failed');
    const permissions = await Kigi.request('permissions.get'); assert(permissions.chromeAutomation === 'unknown', 'Chrome permission invented');
    const copy = await Kigi.request('diagnostics.copy'); assert(copy.copied, 'Backend report not copied');
    let invalid = false; try { await Kigi.request('permissions.openSettings', {section: 'https://example.org'}); } catch (_) { invalid = true; } assert(invalid, 'Arbitrary settings URL accepted');
    window.__kigiEditorCheck = {success: true, captureCanceled: true, captureCompleted: true, carbonRollback: true, systemPages: true};
  } catch (error) { window.__kigiEditorCheck = {success: false, message: String(error.message)}; }
})();
