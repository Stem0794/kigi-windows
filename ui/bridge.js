'use strict';
window.Kigi = {
  sequence: 0,
  async request(method) {
    if (!window.webkit?.messageHandlers?.kigi) throw new Error('Le moteur local est indisponible. Ouvrez Studio depuis l’application Kigi.');
    const id = `req-${++this.sequence}`;
    let timer;
    try {
      const response = await Promise.race([
        window.webkit.messageHandlers.kigi.postMessage(JSON.stringify({protocolVersion: 1, id, method, params: {}})),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Le moteur ne répond pas. Réessayez ou ouvrez les réglages classiques.')), 8000); })
      ]);
      const envelope = typeof response === 'string' ? JSON.parse(response) : response;
      if (envelope.protocolVersion !== 1 || envelope.id !== id) throw new Error('Réponse incompatible du moteur local.');
      if (!envelope.ok) throw new Error(envelope.error?.message || 'Action indisponible.');
      return envelope.data;
    } finally { clearTimeout(timer); }
  }
};
