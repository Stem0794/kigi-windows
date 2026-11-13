// Fixed isolated native recovery flow; no user configuration is touched.
(async () => {
 const q=s=>document.querySelector(s);
 const wait=async predicate=>{for(let i=0;i<100;i++){if(predicate())return;await new Promise(r=>setTimeout(r,30));}throw new Error('Recovery UI timed out');};
 const assert=(v,m)=>{if(!v)throw new Error(m);};
 try {
  await wait(()=>!q('#recover-backup').hidden);
  const before=await Kigi.request('spaces.list');assert(before.usingBackup&&!before.canWrite,'Backup not read-only');
  let refused=false;try{await Kigi.request('spaces.recoverBackup',{expectedRevision:before.revision,confirmed:false});}catch(e){refused=e.code==='CONFIRMATION_REQUIRED';}assert(refused,'Recovery lacked confirmation');
  q('#recover-backup').click();await wait(()=>!q('#modal').hidden);[...q('#modal-actions').querySelectorAll('button')].find(b=>b.textContent==='Annuler').click();
  assert((await Kigi.request('spaces.list')).usingBackup,'Cancel changed primary');
  q('#recover-backup').click();await wait(()=>!q('#modal').hidden);[...q('#modal-actions').querySelectorAll('button')].find(b=>b.textContent==='Récupérer').click();
  await wait(()=>q('#recover-backup').hidden&&!q('#refresh').disabled);
  const after=await Kigi.request('spaces.list');assert(after.canWrite&&!after.usingBackup&&JSON.stringify(after.profiles)===JSON.stringify(before.profiles),'Recovery changed spaces');
  q('#navigation [data-route="spaces"]').click();await wait(()=>!!q('#capture-space'));assert(!q('#capture-space').disabled,'Capture unavailable');
  window.__kigiEditorCheck={success:true,recovery:true};
 }catch(e){window.__kigiEditorCheck={success:false,message:e.message};}
})();
