export const $ = (s, root = document) => root.querySelector(s);
export function el(tag, cls, text) { const n = document.createElement(tag); if (cls) n.className = cls; if (text !== undefined) n.textContent = text; return n; }
export function button(text, action, cls = 'button') { const b = el('button', cls, text); b.type = 'button'; b.addEventListener('click', action); return b; }
export async function api(path, options = {}) {
  const response = await fetch(path, { ...options, headers: { 'Content-Type': 'application/json', ...options.headers } });
  let value; try { value = await response.json(); } catch { throw new Error('Ungültige Serverantwort. Bitte erneut versuchen.'); }
  if (!response.ok) throw new Error(value.error || `Anfrage fehlgeschlagen (${response.status}).`);
  return value;
}
export const clone = v => JSON.parse(JSON.stringify(v));
export const kinds = { clock: 'Uhr & Datum', weather: 'Wetter', embed: 'Einbettung', text: 'Text', heading: 'Überschrift', link: 'Link-Button', image: 'Bild', divider: 'Trennlinie', spacer: 'Abstand', code: 'Eigener Code' };
export const icons = { clock: '◷', weather: '☀', embed: '▦', text: 'T', heading: 'H', link: '↗', image: '▧', divider: '—', spacer: '↕', code: '⌘' };
export function applyAppearance(a, custom = true) {
  document.documentElement.dataset.theme = a.theme;
  document.documentElement.style.setProperty('--accent', a.accent);
  document.documentElement.style.setProperty('--font-size', `${a.font_size}px`);
  document.body.style.background = '';
  if (a.background_type === 'image') document.body.style.background = `linear-gradient(#10151f99,#10151fcc), url(${JSON.stringify(a.background)}) center / cover fixed`;
  else document.body.style.background = a.background;
  let css = $('#custom-css'); if (!css) { css = el('style'); css.id = 'custom-css'; document.head.append(css); }
  css.textContent = custom ? a.custom_css : '';
}
export function safeUrl(value) { try { const u = new URL(value); return ['http:', 'https:'].includes(u.protocol) && !u.username && !u.password ? u.href : ''; } catch { return ''; } }
export function openLink(url, text = 'Öffnen') { const a = el('a', 'button open-button', text + ' ↗'); a.href = safeUrl(url) || '#'; a.target = '_blank'; a.rel = 'noopener noreferrer'; return a; }
export function newWidget(kind) {
  return { id: crypto.randomUUID ? crypto.randomUUID() : `w-${Date.now()}-${Math.random().toString(16).slice(2)}`, kind, title: kinds[kind], x: 0, y: 0, w: kind === 'weather' ? 6 : 4, h: kind === 'weather' ? 5 : ['divider','spacer'].includes(kind) ? 1 : 3,
    settings: { text: '', url: '', icon: icons[kind], timezone: 'Europe/Zurich', latitude: 47.3769, longitude: 8.5417, place: 'Zürich', html: '', css: '', js: '', embed_mode: 'auto' } };
}
export function settle(widgets, changed) {
  // Deterministic collision resolution; move the other cards down, preserving their size.
  const overlap = (a,b) => a.x < b.x+b.w && a.x+a.w > b.x && a.y < b.y+b.h && a.y+a.h > b.y;
  const ordered = [...widgets].filter(w => w !== changed).sort((a,b) => a.y-b.y || a.x-b.x);
  const placed = changed ? [changed] : [];
  for (const w of ordered) { let n=0; while (placed.some(p => overlap(w,p)) && n++<600) w.y++; placed.push(w); }
}
export function position(node, w) { node.style.gridColumn = `${w.x+1} / span ${w.w}`; node.style.gridRow = `${w.y+1} / span ${w.h}`; }
export function weatherInfo(code, day = true) {
  if (code === 0) return [day ? '☀' : '☾', 'Klar'];
  if (code <= 3) return [code < 3 ? '⛅' : '☁', code < 3 ? 'Leicht bewölkt' : 'Bewölkt'];
  if (code <= 48) return ['≋', 'Neblig']; if (code <= 67) return ['☂', 'Regen'];
  if (code <= 77) return ['❄', 'Schnee']; if (code <= 86) return ['☂', 'Schauer']; return ['ϟ', 'Gewitter'];
}
export function renderWidget(w, { editor = false, preview = false } = {}) {
  const card = el('article', `widget widget-${w.kind}`); card.dataset.id = w.id; position(card, w);
  const head = el('div', 'widget-header'); head.append(el('span', 'widget-heading', w.title), el('span', 'widget-symbol', icons[w.kind] || '◇')); card.append(head);
  const body = el('div', 'widget-body'); card.append(body);
  const s = w.settings;
  const fallback = (reason, url = s.url) => { body.replaceChildren(); const box = el('div', 'fallback-card'); box.append(el('div', 'fallback-icon', s.icon || icons[w.kind]), el('h3', '', w.title), el('p', 'muted', reason)); if (safeUrl(url)) box.append(openLink(url)); else box.append(el('span', 'small muted', 'In «Bearbeiten» einrichten')); body.append(box); };
  let dispose = () => {};
  if (editor) {
    body.classList.add('editor-card-body');
    const info = ['text','heading'].includes(w.kind) ? s.text : w.kind === 'weather' ? s.place : w.kind === 'clock' ? s.timezone : w.kind === 'code' ? 'HTML · CSS · JavaScript · Sandbox' : s.url || kinds[w.kind];
    body.append(el('span', 'editor-kind', kinds[w.kind]), el('p', 'muted editor-description', info));
  } else if (w.kind === 'clock') {
    const time = el('div', 'clock-time'); const date = el('div', 'clock-date'); const zone = el('div', 'clock-zone', '◷ ' + s.timezone);
    body.append(time, date, zone);
    const update = () => { try { const now = new Date(); time.textContent = now.toLocaleTimeString('de-CH', { timeZone: s.timezone, hour:'2-digit', minute:'2-digit' }); date.textContent = now.toLocaleDateString('de-CH',{timeZone:s.timezone, weekday:'long', day:'numeric',month:'long'}); } catch { time.textContent = 'Zeitzone prüfen'; } };
    update(); const interval = setInterval(update,1000); dispose=()=>clearInterval(interval);
  } else if (w.kind === 'weather') {
    body.append(el('p','muted','Wetter wird geladen …'));
    let alive = true;
    const update = async () => {
      try {
        const r = await api(preview ? `/api/editor/weather?latitude=${encodeURIComponent(s.latitude)}&longitude=${encodeURIComponent(s.longitude)}` : `/api/weather/${encodeURIComponent(w.id)}`); if (!alive) return;
        const f=r.data, info=weatherInfo(f.current.weather_code, f.current.is_day===1);
        body.replaceChildren();
        const current=el('div','weather-current'); const temperatures=el('div'); temperatures.append(el('span','weather-temperature',`${Math.round(f.current.temperature_2m)}°`),el('span','weather-condition',info[1])); current.append(temperatures,el('span','weather-art',info[0])); body.append(current);
        body.append(el('div','weather-meta',`${s.place}  ·  Wind ${Math.round(f.current.wind_speed_10m)} km/h  ·  ${Math.round(f.current.relative_humidity_2m)} % Luftfeuchte`));
        const times = f.hourly.time; let start=times.findIndex(t=>t>=f.current.time.slice(0,13)+':00'); if(start<0)start=0;
        const hourly=el('div','hourly-strip');
        for(let i=start;i<Math.min(start+12,times.length);i+=2){const h=el('div','hour-item'); h.append(el('span','small muted',i===start?'Jetzt':times[i].slice(11,16)),el('span','hour-icon',weatherInfo(f.hourly.weather_code[i]??3)[0]),el('strong','',`${Math.round(f.hourly.temperature_2m[i]??0)}°`));hourly.append(h);} body.append(hourly);
        const daily=el('div','daily-list');
        for(let i=0;i<5;i++){const d=el('div','daily-row');d.append(el('span','',i===0?'Heute':new Date(f.daily.time[i]+'T12:00:00').toLocaleDateString('de-CH',{weekday:'short'})),el('span','day-icon',weatherInfo(f.daily.weather_code[i]??3)[0]),el('span','rain-chance',`${f.daily.precipitation_probability_max[i]??'–'} %`),el('span','temp-range',`${Math.round(f.daily.temperature_2m_min[i]??0)}°`),el('strong','',`${Math.round(f.daily.temperature_2m_max[i]??0)}°`));daily.append(d);} body.append(daily);
        const footer=el('div','weather-credit');footer.append(el('span',r.stale?'stale':'',r.stale?'Offline · letzte bekannte Daten':`Aktualisiert ${new Date(r.fetched_at).toLocaleTimeString('de-CH',{hour:'2-digit',minute:'2-digit'})}`));const credit=openLink('https://open-meteo.com/','Open-Meteo');credit.className='credit-link';footer.append(credit);body.append(footer);
      } catch(e) { if(alive) { body.replaceChildren(el('div','widget-error',e.message)); } }
    };
    update(); const timer=setInterval(update,900000); dispose=()=>{alive=false;clearInterval(timer);};
  } else if (w.kind === 'embed') {
    fallback('Einbettung wird geprüft …'); let alive=true;
    const install=(status)=>{
      if(!alive)return; if(!status.allowed){fallback(status.reason);return;}
      body.replaceChildren(); const frame=el('iframe','embedded-frame');frame.title=w.title;frame.src=safeUrl(s.url);frame.referrerPolicy='no-referrer';frame.sandbox='allow-scripts allow-same-origin allow-forms allow-popups';frame.loading='lazy';
      const foot=el('div','embed-footer');foot.append(el('span','small muted','Inhalt nicht sichtbar?'),openLink(s.url));body.append(frame,foot);
      frame.addEventListener('error',()=>fallback('Einbettung nicht erreichbar. Im neuen Tab öffnen.'));
    };
    if(preview){install({allowed:s.embed_mode==='iframe',reason:'Vorschau: Einbettungen werden nach dem Speichern automatisch geprüft.'});}
    else api(`/api/embed/${encodeURIComponent(w.id)}`).then(install).catch(()=>fallback('Verbindung unterbrochen. Im neuen Tab öffnen.'));
    dispose=()=>{alive=false;};
  } else if (w.kind === 'text' || w.kind === 'heading') { body.append(el(w.kind==='heading'?'h2':'p','text-content',s.text || 'Deine Gedanken haben hier Platz.')); }
  else if (w.kind === 'link') { body.append(el('div','link-icon',s.icon||'↗'),openLink(s.url,w.title)); }
  else if (w.kind === 'image') { if(safeUrl(s.url)){const img=el('img','widget-image');img.src=safeUrl(s.url);img.alt=w.title;img.loading='lazy';img.referrerPolicy='no-referrer';img.addEventListener('error',()=>fallback('Bild konnte nicht geladen werden.'));body.append(img);}else fallback('Bild-URL im Editor eintragen.'); }
  else if (w.kind === 'code') {
    const frame=el('iframe','code-frame');frame.title=w.title;frame.sandbox='allow-scripts';frame.src='/sandbox';frame.addEventListener('load',()=>frame.contentWindow.postMessage({type:'homeboard-code',html:s.html,css:s.css,js:s.js},'*'));body.append(frame);
  } else if (w.kind === 'divider') body.append(el('hr'));
  return { node:card, dispose };
}
