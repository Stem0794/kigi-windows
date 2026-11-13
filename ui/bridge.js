'use strict';
window.Kigi = {
  sequence: 0,
  async request(method, params = {}) {
    if (!window.webkit?.messageHandlers?.kigi) throw new Error('Le moteur local est indisponible. Ouvrez Studio depuis l’application Kigi.');
    const id = `req-${++this.sequence}`;
    let timer;
    try {
      const response = await Promise.race([
        window.webkit.messageHandlers.kigi.postMessage(JSON.stringify({protocolVersion: 1, id, method, params})),
        new Promise((_, reject) => { if (!['spaces.selectApp', 'spaces.apply', 'spaces.restore'].includes(method)) timer = setTimeout(() => reject(new Error('Le moteur ne répond pas. Vérifiez les données avec Actualiser avant de réessayer.')), 8000); })
      ]);
      const envelope = typeof response === 'string' ? JSON.parse(response) : response;
      if (envelope.protocolVersion !== 1 || envelope.id !== id) throw new Error('Réponse incompatible du moteur local.');
      if (!envelope.ok) throw Object.assign(new Error(envelope.error?.message || 'Action indisponible.'), {code: envelope.error?.code});
      return envelope.data;
    } finally { clearTimeout(timer); }
  }
};
