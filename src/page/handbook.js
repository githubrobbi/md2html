// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// The page's own behaviour: the progress line, the outline that follows
// the reader (and its drawer), the crumb in the bar, the search and the
// words it found, lit where it lands. Everything reads without it.
(function(){
var $=function(s,r){return (r||document).querySelector(s);},$$=function(s,r){return [].slice.call((r||document).querySelectorAll(s));};
var UI={};try{UI=JSON.parse($('#md2html-ui').textContent)||{};}catch(e){}
function say(k,i,d){var v=UI[k];if(v&&i!=null)v=v[i];return v||d;}
var bar=$('#bar');addEventListener('scroll',function(){var h=document.documentElement;var p=h.scrollTop/((h.scrollHeight-h.clientHeight)||1);bar.style.width=(p*100)+'%';},{passive:true});
var land=window.ivLand||function(){};
var root=document.documentElement,nav=$('nav.toc'),tg=$('#toggle'),edge=$('#edge'),TKEY='iv-toc';
var wide=function(){return !(window.matchMedia&&window.matchMedia('(max-width:980px)').matches);};
var hoverable=function(){return window.matchMedia&&window.matchMedia('(hover:hover) and (pointer:fine)').matches;};
function tocOff(){return root.getAttribute('data-toc')==='off';}
function tocLabel(){var shown=wide()?!tocOff():nav.classList.contains('open');tg.setAttribute('aria-expanded',String(shown));
var tip=(shown?say('outline',0,'Hide the outline'):say('outline',1,'Show the outline'))+say('outline',2,' (⌘\\, Control+\\ on Windows)');tg.title=tip;tg.setAttribute('aria-label',tip);}
function tocSet(off){if(off){root.setAttribute('data-toc','off');}else{root.removeAttribute('data-toc');}
try{if(off){localStorage.setItem(TKEY,'off');}else{localStorage.removeItem(TKEY);}}catch(e){}nav.classList.remove('open');tocLabel();}
function drawer(open){nav.classList.toggle('open',open);tocLabel();}
$('#dock').addEventListener('click',function(e){e.preventDefault();tocSet(false);});
function tocToggle(){if(wide()){tocSet(!tocOff());}else{drawer(!nav.classList.contains('open'));}}
tocLabel();tg.addEventListener('click',tocToggle);
var peek=null;
edge.addEventListener('click',function(){drawer(true);});
edge.addEventListener('mouseenter',function(){if(!hoverable())return;peek=setTimeout(function(){drawer(true);},220);});
edge.addEventListener('mouseleave',function(){clearTimeout(peek);});
nav.addEventListener('mouseleave',function(){if(tocOff()&&wide()&&hoverable())drawer(false);});
document.addEventListener('click',function(e){if(e.target.closest('#dock'))return;var t=e.target.closest('nav.toc a');if(t){drawer(false);return;}
if(nav.classList.contains('open')&&!e.target.closest('nav.toc,#toggle,#edge'))drawer(false);});
document.addEventListener('keydown',function(e){
if(e.key==='Escape'){if(nav.classList.contains('open')){drawer(false);}
// Escape takes a word's explanation away without moving the reader.
var a=document.activeElement;if(a&&a.matches&&a.matches('.term,.fnref'))a.blur();}
else if((e.metaKey||e.ctrlKey)&&!e.altKey&&e.code==='Backslash'){e.preventDefault();tocToggle();}});
// A table or code block wider than the column takes the keyboard's focus,
// so it can be scrolled without a mouse.
function reachable(){var n=0;$$('article .scroll,article pre,article .math,article .diagram').forEach(function(el){var wider=el.scrollWidth>el.clientWidth+1;
if(wider&&!el.hasAttribute('tabindex')){el.tabIndex=0;if(el.classList.contains('scroll')){el.setAttribute('role','region');n++;el.setAttribute('aria-label',say('table',null,'Table')+' '+n);}else if(el.classList.contains('diagram')){var t=el.querySelector('title');el.setAttribute('role','region');el.setAttribute('aria-label',t?t.textContent:'Diagram');}}});}
addEventListener('load',reachable);
var links={},crumb=$('#crumb'),heads=$$('article h2[id],article h3[id]');
$$('nav.toc a[href^="#"]').forEach(function(a){links[a.getAttribute('href').slice(1)]=a;});
var current=null;
function mark(h){if(!h||h===current)return;current=h;
$$('nav.toc a.on').forEach(function(a){a.classList.remove('on');a.removeAttribute('aria-current');});$$('nav.toc li.open').forEach(function(li){li.classList.remove('open');});
var a=links[h.id];if(!a)return;a.classList.add('on');a.setAttribute('aria-current','location');var li=a.parentNode;var top=li.closest('nav.toc>ol>li');if(top){top.classList.add('open');var ta=top.querySelector(':scope>a');if(ta)ta.classList.add('on');}
var ch=top?top.querySelector(':scope>a').textContent:'';var sec=h.tagName==='H3'?(h.firstChild.textContent||''):'';
crumb.textContent=sec?ch+'  ›  '+sec:ch;a.scrollIntoView({block:'nearest'});}
function spy(){var y=scrollY+120,best=null;for(var i=0;i<heads.length;i++){if(heads[i].offsetTop<=y)best=heads[i];else break;}mark(best||heads[0]);}
addEventListener('scroll',spy,{passive:true});addEventListener('load',spy);spy();
// The language switch. Another version keeps the reader's place (the n-th
// heading: a translation keeps the chapters); the reader's own browser, where
// it can, translates into any other language on the device itself.
var langs=$('#langs');
if(langs){
$$('#langs ul.authored a').forEach(function(a){a.addEventListener('click',function(){var i=heads.indexOf(current);
if(i>0)a.setAttribute('href',a.getAttribute('href').split('#')[0]+'#~'+i);});});
var dev=$('#langs .device');
if(dev&&('Translator' in self)){dev.hidden=false;
dev.addEventListener('click',function(e){var b=e.target.closest('button[data-lang]');if(!b)return;
if(langs.hidePopover)langs.hidePopover();translate(b.getAttribute('data-lang'),b.textContent);});}}
var pr=$('#print');if(pr)pr.addEventListener('click',function(){var p=$('#keep');if(p&&p.hidePopover)p.hidePopover();setTimeout(function(){print();},50);});
var place=/^#~(\d+)$/.exec(location.hash);if(place&&heads[+place[1]])heads[+place[1]].scrollIntoView();
var originals=null,mt=null;
function mtSay(text,undo){if(!mt){mt=document.createElement('div');mt.className='mt';mt.setAttribute('role','status');var h=$('article header.hero');if(h)h.after(mt);}
mt.textContent='';var s=document.createElement('span');s.textContent=text;mt.appendChild(s);
if(undo){var b=document.createElement('button');b.type='button';b.className='btn';b.textContent=say('lang',5,'Show the original');b.addEventListener('click',restore);mt.appendChild(b);}}
function restore(){if(originals){originals.forEach(function(o){o[0].nodeValue=o[1];});originals=null;}
var art=$('article');art.removeAttribute('lang');art.removeAttribute('dir');if(mt){mt.remove();mt=null;}}
async function translate(code,name){restore();var src=(document.documentElement.lang||'en').split('-')[0],cannot=say('lang',7,'This browser cannot translate into {lang}.').replace('{lang}',name);
try{var av=await Translator.availability({sourceLanguage:src,targetLanguage:code});if(av==='unavailable'){mtSay(cannot);return;}
mtSay(say('lang',6,'Translating…'));
var tr=await Translator.create({sourceLanguage:src,targetLanguage:code,monitor:function(m){m.addEventListener('downloadprogress',function(e){mtSay(say('lang',6,'Translating…')+' '+Math.round(e.loaded*100)+'%');});}});
var art=$('article'),walk=document.createTreeWalker(art,NodeFilter.SHOW_TEXT,{acceptNode:function(n){return n.nodeValue.trim()&&!n.parentNode.closest('pre,code,kbd,script,style,svg,math,.anchor,.mt')?1:2;}}),nodes=[],t;
while((t=walk.nextNode()))nodes.push(t);originals=[];
for(var i=0;i<nodes.length;i++){var n=nodes[i],v=n.nodeValue,lead=v.match(/^\s*/)[0],trail=v.match(/\s*$/)[0];
var out=await tr.translate(v.trim());originals.push([n,v]);n.nodeValue=lead+out+trail;}
art.setAttribute('lang',code);if(['he','yi','ar','fa','ur'].indexOf(code)>=0)art.setAttribute('dir','rtl');
mtSay(say('lang',4,'A machine translation by this browser, on this device: nothing leaves it.'),true);}
catch(e){if(originals)restore();mtSay(cannot);}}
var dlg=$('#find'),q=$('#q'),hits=$('#hits'),count=$('#hits-n'),idx=[];try{idx=JSON.parse($('#idx').textContent);}catch(e){}
var sel=0,found=[],words=[];
function esc(s){return s.replace(/[&<>]/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;'}[c];});}
function rx(w){return w.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');}
function snippet(x,words){var lx=x.toLowerCase(),at=-1;for(var i=0;i<words.length;i++){var p=lx.indexOf(words[i]);if(p>=0&&(at<0||p<at))at=p;}
if(at<0)at=0;var s=Math.max(0,at-60),e=Math.min(x.length,at+120),out=esc(x.slice(s,e));
words.forEach(function(w){if(!w)return;out=out.replace(new RegExp('('+rx(esc(w))+')','ig'),'<mark>$1</mark>');});
return (s>0?'…':'')+out+(e<x.length?'…':'');}
function choose(i){sel=i;$$('#hits li[role=option]').forEach(function(li,j){li.classList.toggle('on',j===sel);li.setAttribute('aria-selected',String(j===sel));});
var on=hits.children[sel];if(on&&on.id){q.setAttribute('aria-activedescendant',on.id);on.scrollIntoView({block:'nearest'});}else{q.removeAttribute('aria-activedescendant');}}
function search(){words=q.value.toLowerCase().split(/\s+/).filter(Boolean);found=[];hits.innerHTML='';sel=0;q.removeAttribute('aria-activedescendant');
q.setAttribute('aria-expanded','false');if(!words.length){count.textContent='';return;}
var phrase=words.join(' ');
idx.forEach(function(s){var t=s.t.toLowerCase(),x=s.x.toLowerCase(),score=0;for(var i=0;i<words.length;i++){var w=words[i],inT=t.indexOf(w)>=0,n=Math.min(x.split(w).length-1,4);if(!inT&&!n){score=0;break;}score+=(inT?6:0)+n;}
if(score){if(t===phrase)score+=20;if(words.length>1&&(t.indexOf(phrase)>=0||x.indexOf(phrase)>=0))score+=8;found.push({s:s,score:score});}});
found.sort(function(a,b){return b.score-a.score;});
var total=found.length;found=found.slice(0,14);
count.textContent=total===1?say('result',null,'1 result'):say('results',null,'{n} results').replace('{n}',total);
if(!found.length){hits.innerHTML='<li class="none">'+esc(say('none',null,'Nothing on this page matches that.'))+'</li>';return;}
q.setAttribute('aria-expanded','true');
found.forEach(function(f,i){var li=document.createElement('li');li.setAttribute('role','option');li.id='hit-'+i;if(i===0)li.className='on';li.setAttribute('aria-selected',String(i===0));
li.innerHTML='<span class="c">'+esc(f.s.c)+'</span><span class="h">'+esc(f.s.t)+'</span><span class="s">'+snippet(f.s.x,words)+'</span>';
li.addEventListener('click',function(){go(i);});hits.appendChild(li);});choose(0);}
// The words searched for, lit in the part of the page the search landed
// on, until the next search.
function unlight(){$$('mark.hit').forEach(function(m){var p=m.parentNode;p.replaceChild(document.createTextNode(m.textContent),m);p.normalize();});}
function region(el){var out=[el];if(el.tagName==='DT'){if(el.nextElementSibling)out.push(el.nextElementSibling);return out;}
if(/^H[2-6]$/.test(el.tagName)||el.id==='opening'){var n=el.nextElementSibling;while(n&&!/^H[23]$/.test(n.tagName)&&!(n.tagName==='SECTION')){out.push(n);n=n.nextElementSibling;}}return out;}
function light(id,ws){unlight();var el=document.getElementById(id);if(!el||!ws.length)return;
var re=new RegExp('('+ws.filter(function(w){return w.length>1;}).map(rx).join('|')+')','ig');if(re.source==='()')return;var budget=200;
region(el).forEach(function(part){var walk=document.createTreeWalker(part,NodeFilter.SHOW_TEXT,null),nodes=[],t;
while((t=walk.nextNode()))nodes.push(t);
nodes.forEach(function(node){if(budget<=0||!node.nodeValue.trim()||node.parentNode.closest('script,style,mark,.anchor'))return;
re.lastIndex=0;if(!re.test(node.nodeValue))return;re.lastIndex=0;var frag=document.createDocumentFragment(),last=0,m,s=node.nodeValue;
while((m=re.exec(s))&&budget>0){frag.appendChild(document.createTextNode(s.slice(last,m.index)));var k=document.createElement('mark');k.className='hit';k.textContent=m[0];frag.appendChild(k);last=m.index+m[0].length;budget--;}
frag.appendChild(document.createTextNode(s.slice(last)));node.parentNode.replaceChild(frag,node);});});}
function go(i){var f=found[i];if(!f)return;dlg.close();location.hash='#'+f.s.id;land(f.s.id);light(f.s.id,words);}
function open(){if(!dlg.open){dlg.showModal();q.select();}}
$('#open-find').addEventListener('click',open);
q.addEventListener('input',search);
q.addEventListener('keydown',function(e){var n=$$('#hits li[role=option]').length;if(e.key==='ArrowDown'&&n){e.preventDefault();choose((sel+1)%n);}else if(e.key==='ArrowUp'&&n){e.preventDefault();choose((sel-1+n)%n);}else if(e.key==='Enter'){e.preventDefault();go(sel);}});
dlg.addEventListener('click',function(e){if(e.target===dlg)dlg.close();});
// Escape always closes the search: the browser's own close request can be held back
// (Chrome groups close requests that come without a fresh user activation).
dlg.addEventListener('keydown',function(e){if(e.key==='Escape'){e.preventDefault();e.stopPropagation();dlg.close();}});
document.addEventListener('keydown',function(e){if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==='k'){e.preventDefault();open();}
else if(e.key==='/'&&!dlg.open&&!/input|textarea|select/i.test(document.activeElement.tagName)&&!document.activeElement.isContentEditable){e.preventDefault();open();}});
if(location.hash)land(decodeURIComponent(location.hash.slice(1)));
})();
