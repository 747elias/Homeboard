import {$,el,api,applyAppearance,renderWidget} from './common.js';
let cleanup=[];
function render(c, preview=false){cleanup.forEach(fn=>fn());cleanup=[];applyAppearance(c.appearance);const board=$('#board');board.replaceChildren();for(const w of c.widgets){const r=renderWidget(w,{preview});board.append(r.node);cleanup.push(r.dispose);}if(!c.widgets.length)board.append(el('div','notice','Noch keine Widgets. Im Editor kannst du dein Homeboard gestalten.'));}
const now=new Date();$('#today').textContent=now.toLocaleDateString('de-CH',{timeZone:'Europe/Zurich',weekday:'long',day:'numeric',month:'long'});
const hour=Number(new Intl.DateTimeFormat('de-CH',{timeZone:'Europe/Zurich',hour:'2-digit',hourCycle:'h23'}).formatToParts(now).find(p=>p.type==='hour').value);$('#greeting').textContent=hour<11?'GUTEN MORGEN':hour<18?'HALLO, SCHÖN BIST DU DA':'EINEN SCHÖNEN ABEND';
if(location.pathname==='/preview'){
 $('#preview-banner').classList.remove('hidden');$('#close-preview').addEventListener('click',()=>window.close());
 window.addEventListener('message',e=>{if(e.origin===location.origin && e.source===window.opener && e.data?.type==='homeboard-preview')render(e.data.config,true);});
 if(window.opener)window.opener.postMessage({type:'homeboard-preview-ready'},location.origin);else $('#page-error').textContent='Vorschau bitte aus dem Editor öffnen.';
}else{
 let revision;
 const refresh=async()=>{try{const c=await api('/api/dashboard');if(revision!==c.revision){render(c);revision=c.revision;}$('#page-error').classList.add('hidden');}catch(e){$('#page-error').textContent=e.message;$('#page-error').classList.remove('hidden');}};
 refresh();setInterval(refresh,60000);
}
