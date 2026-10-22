window.addEventListener('message', e => {
  if (e.source !== window.parent || e.data?.type !== 'homeboard-code') return;
  document.head.querySelectorAll('style').forEach(n => n.remove());
  const style = document.createElement('style'); style.textContent = 'body{margin:12px;color:#e8edf4;font:16px system-ui;background:transparent}' + e.data.css; document.head.append(style);
  document.body.innerHTML = e.data.html;
  // Script execution is deliberately confined to an opaque-origin, sandboxed document.
  const script = document.createElement('script'); script.textContent = e.data.js; document.body.append(script);
});
