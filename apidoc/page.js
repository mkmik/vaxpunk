
const q=document.getElementById('q'),hits=document.getElementById('hits');let res=[],sel=0;
function score(e,t){const n=e[0].toLowerCase(),b=n.replace(/^[a-z]*\$_?/,'');t=t.replace(/^sys\$/,'$');
 if(n===t||b===t)return 0;if(n.startsWith(t)||b.startsWith(t))return 1;if(n.includes(t))return 2;
 const h=(n+' '+e[1]+' '+e[2]).toLowerCase();return t.split(/\s+/).every(w=>h.includes(w))?3:-1}
function esc(s){return s.replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]))}
function show(){const t=q.value.trim().toLowerCase();if(!t){hits.classList.remove('on');return}
 res=IDX.map(e=>[score(e,t),e]).filter(x=>x[0]>=0).sort((a,b)=>a[0]-b[0]||a[1][0].length-b[1][0].length).slice(0,40).map(x=>x[1]);
 sel=0;hits.innerHTML=res.length?res.map((e,i)=>`<li${i?'':' class=sel'}><a href="#${e[3]}"><b>${esc(e[0])}</b><span class=k>${esc(e[1])}</span><span class=s>${esc(e[2])}</span></a></li>`).join(''):'<li><a>No match</a></li>';
 hits.classList.add('on')}
function mark(){[...hits.children].forEach((li,i)=>li.classList.toggle('sel',i===sel));hits.children[sel]?.scrollIntoView({block:'nearest'})}
function go(e){location.hash=e[3];hits.classList.remove('on');q.blur()}
q.addEventListener('input',show);q.addEventListener('focus',show);
q.addEventListener('keydown',ev=>{if(ev.key==='ArrowDown'){sel=Math.min(sel+1,res.length-1);mark();ev.preventDefault()}
 else if(ev.key==='ArrowUp'){sel=Math.max(sel-1,0);mark();ev.preventDefault()}
 else if(ev.key==='Enter'&&res[sel])go(res[sel]);else if(ev.key==='Escape'){q.value='';hits.classList.remove('on');q.blur()}});
hits.addEventListener('click',()=>{hits.classList.remove('on');q.blur()});
document.addEventListener('click',ev=>{if(!ev.target.closest('.search'))hits.classList.remove('on')});
document.addEventListener('keydown',ev=>{if((ev.key==='/'||(ev.key==='k'&&(ev.metaKey||ev.ctrlKey)))&&document.activeElement!==q){q.focus();q.select();ev.preventDefault()}});
const th=document.getElementById('theme');function setvt(on){document.documentElement.classList.toggle('vt',on);th.textContent=on?'PAPER':'VT220';localStorage.setItem('vt',on?'1':'')}
setvt(!!localStorage.getItem('vt'));th.addEventListener('click',()=>setvt(!document.documentElement.classList.contains('vt')));
const links=new Map([...document.querySelectorAll('#toc a')].map(a=>[a.getAttribute('href').slice(1),a]));
const spy=new IntersectionObserver(es=>{for(const e of es)if(e.isIntersecting){const a=links.get(e.target.id);if(!a)continue;
 document.querySelectorAll('#toc a.on').forEach(x=>x.classList.remove('on'));a.classList.add('on');a.scrollIntoView({block:'nearest'})}},
 {rootMargin:'0px 0px -85% 0px'});
document.querySelectorAll('main section[id]').forEach(el=>spy.observe(el));
