// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
//
// The currency switch, on a page whose author asked for one (`currency` and
// `rates`): the page's amounts in another currency, at the rates of one day
// that are in the page. The page shows its own amounts until the reader
// picks another currency; the first offered is the one of the language the
// page is read in. When the language changes, a currency that was the
// default of the language left becomes the default of the language read
// now (US dollars in English, euros in German); any other stays (yen, read
// in English, stay yen in German). The choice is kept for the next page.
// Nothing is fetched: the rates are the author's.
(function(){
'use strict';
var UI={};try{UI=JSON.parse(document.getElementById('md2html-ui').textContent)||{};}catch(e){}
function say(list,i,dflt){var w=UI[list];return w&&w[i]||dflt;}
var D=null;try{D=JSON.parse(document.getElementById('md2html-money').textContent);}catch(e){}
var btn=document.getElementById('money'),panel=document.getElementById('moneys'),art=document.querySelector('article');
if(!D||!D.rates||!btn||!panel||!art||!window.Intl||!Intl.NumberFormat)return;
var MAIN=D.main,KEY='md2html-currency',SKIP='pre,code,kbd,script,style,svg,math,textarea,select,option,.anchor,.mt,.money';
// What one unit of the base buys of `code`; how much of `code` one unit of the page's currency is.
function of(code){return code===D.base?1:+D.rates[code]||0;}
function per(code){return of(code)/of(MAIN);}
if(!of(MAIN))return;
function lang(){return art.getAttribute('lang')||document.documentElement.lang||'en';}
function digits(code){try{return new Intl.NumberFormat('en',{style:'currency',currency:code}).resolvedOptions().maximumFractionDigits;}catch(e){return 2;}}
function nameOf(code,in_){try{return new Intl.DisplayNames([in_],{type:'currency'}).of(code)||code;}catch(e){return code;}}
// The ways the page's currency is written: its code, and its signs and names
// as the page's language, English and the languages that count in it write them.
var signs=[MAIN],langs=[document.documentElement.lang||'en','en'];
Object.keys(D.home||{}).forEach(function(l){if(D.home[l]===MAIN)langs.push(l);});
langs.forEach(function(l){['symbol','narrowSymbol','name'].forEach(function(how){[1,5].forEach(function(n){
  try{new Intl.NumberFormat(l,{style:'currency',currency:MAIN,currencyDisplay:how}).formatToParts(n).forEach(function(p){
    var t=p.value.trim();if(p.type==='currency'&&t&&signs.indexOf(t)<0)signs.push(t);});}catch(e){}});});});
signs.sort(function(a,b){return b.length-a.length;});
function rx(s){return s.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');}
var SIGN='(?:'+signs.map(rx).join('|')+')',NUM='\\d{1,3}(?:[,.\\u00a0\\u202f]\\d{3})+(?:[.,]\\d+)?|\\d+(?:[.,]\\d+)?',DASH='\\s?[\\u2013\\u2014-]\\s?';
// A sign before the number (¥215,378, ¥37,000–48,000) or after it (12,000円, 5–8 €);
// a sign of letters is not the end of a word (Rp, not "grip").
var AMOUNT=new RegExp('(?<![\\p{L}\\d])'+SIGN+'\\s?('+NUM+')(?:'+DASH+'(?:'+SIGN+'\\s?)?('+NUM+'))?(?![\\d])'+
  '|(?<![\\p{L}\\d.,])('+NUM+')(?:'+DASH+'('+NUM+'))?\\s?'+SIGN+'(?![\\p{L}])','gu');
// A number as written: the last of a comma and a point is the decimal one,
// a lone separator before three digits groups them.
function num(s){s=s.replace(/[\s  ]/g,'');var c=s.lastIndexOf(','),p=s.lastIndexOf('.'),d=Math.max(c,p);
  if(d<0)return +s;
  if(c>=0&&p>=0)return +(s.slice(0,d).replace(/[.,]/g,'')+'.'+s.slice(d+1));
  var sep=s.charAt(d),parts=s.split(sep);
  if(parts.length>2||parts[1].length===3)return +parts.join('');
  return +(parts[0]+'.'+parts[1]);}
// Each amount in the text under `root`, wrapped once so it can be shown in another currency.
function wrap(root){var nodes=[],t,walk=document.createTreeWalker(root,NodeFilter.SHOW_TEXT,{acceptNode:function(n){
  return n.nodeValue.trim()&&/\d/.test(n.nodeValue)&&!(n.parentNode.closest&&n.parentNode.closest(SKIP))?1:2;}});
  while((t=walk.nextNode()))nodes.push(t);
  nodes.forEach(function(node){var s=node.nodeValue,m,last=0,frag=null;AMOUNT.lastIndex=0;
    while((m=AMOUNT.exec(s))){var a=m[1]||m[3],b=m[2]||m[4];if(!frag)frag=document.createDocumentFragment();
      frag.appendChild(document.createTextNode(s.slice(last,m.index)));
      var k=document.createElement('span');k.className='money';k.textContent=m[0];k.setAttribute('data-o',m[0]);k.setAttribute('data-v',String(num(a)));
      if(b)k.setAttribute('data-w',String(num(b)));frag.appendChild(k);last=m.index+m[0].length;}
    if(frag){frag.appendChild(document.createTextNode(s.slice(last)));node.parentNode.replaceChild(frag,node);}});}
var shown='',note=null;
function fmt(code,v){var big=Math.abs(v)>=100,n=big?0:digits(code);
  return new Intl.NumberFormat(lang(),{style:'currency',currency:code,minimumFractionDigits:n,maximumFractionDigits:n});}
// The amounts in `code`, or as written for none.
function show(code){shown=code;var f=per(code);
  document.querySelectorAll('article .money').forEach(function(k){var o=k.getAttribute('data-o');
    if(!code){k.textContent=o;k.removeAttribute('title');k.classList.remove('on');return;}
    var v=+k.getAttribute('data-v')*f,w=k.getAttribute('data-w'),x=fmt(code,Math.max(v,w?+w*f:0));
    k.textContent=w!==null&&x.formatRange?x.formatRange(v,+w*f):w!==null?x.format(v)+'–'+x.format(+w*f):x.format(v);
    k.title=o;k.classList.add('on');});
  btn.querySelector('.lc').textContent=code||MAIN;say_note(code);build();}
function rateText(code){var f=per(code),one=new Intl.NumberFormat(lang(),{maximumSignificantDigits:5});
  return f>=1?'1 '+MAIN+' = '+one.format(f)+' '+code:'1 '+code+' = '+one.format(1/f)+' '+MAIN;}
function noteText(code){var t=D.day?say('money',1,'Converted at the rate of {date}').replace('{date}',D.day):'';return (t?t+' · ':'')+rateText(code);}
function say_note(code){if(note){note.remove();note=null;}if(!code)return;
  note=document.createElement('div');note.className='mt fx';note.setAttribute('role','status');
  var s=document.createElement('span');s.textContent=nameOf(code,lang())+' · '+noteText(code);note.appendChild(s);
  var b=document.createElement('button');b.type='button';b.className='btn';b.textContent=say('lang',5,'Show the original');
  b.addEventListener('click',function(){choose('');});note.appendChild(b);
  var h=art.querySelector('header.hero');if(h)h.after(note);else art.prepend(note);}
function keep(v){try{if(v)localStorage.setItem(KEY,v);else localStorage.removeItem(KEY);}catch(e){}}
function kept(){try{return localStorage.getItem(KEY)||'';}catch(e){return '';}}
// The currency of the language the page is read in, when the rates have it.
function home(l){var h=D.home||{},low=(l||lang()).toLowerCase(),code='';
  Object.keys(h).forEach(function(k){if(k.toLowerCase()===low)code=h[k];});
  if(!code&&/^zh-(tw|hk|mo)/.test(low))code=h['zh-Hant'];
  if(!code)Object.keys(h).forEach(function(k){if(!code&&k.split('-')[0].toLowerCase()===low.split('-')[0])code=h[k];});
  return code&&code!==MAIN&&of(code)?code:'';}
// The reader's choice: none (the amounts as written), `home` (the currency
// of the language read in, whichever that is) or a currency's code. It is
// kept for the next page, and resolved against the language shown.
var choice='';
function target(v){var c=v==='home'?home():v;return c&&c!==MAIN&&of(c)?c:'';}
function choose(v){choice=v;keep(v);quiet(function(){show(target(v));});}
// The language changed: a currency the reader picked becomes the new
// language's own when it is (euros picked, then German), and follows the
// language from then on; `home` follows it anyway; none stays none.
function moved(){if(choice&&choice!=='home'&&choice===home()){choice='home';keep(choice);}show(target(choice));}
function item(ul,code,v,extra){var li=document.createElement('li'),b=document.createElement('button');b.type='button';
  b.setAttribute('data-cur',v);b.textContent=nameOf(code,lang());var sm=document.createElement('small');sm.textContent=code+(extra?' · '+extra:'');
  b.appendChild(document.createTextNode(' '));b.appendChild(sm);if((shown||MAIN)===code)b.setAttribute('aria-current','true');
  b.addEventListener('click',function(){choose(v);try{panel.hidePopover();}catch(e){}});li.appendChild(b);ul.appendChild(li);}
function build(){var label=say('money',0,'Currency'),open=panel.querySelector('details[open]');panel.textContent='';
  var p=document.createElement('p');p.className='lh';p.textContent=label;panel.appendChild(p);
  var ul=document.createElement('ul');panel.appendChild(ul);item(ul,MAIN,'',say('lang',1,'Original'));
  var h=home();if(h)item(ul,h,'home','');
  var rest=Object.keys(D.rates).concat([D.base]).filter(function(c,i,all){return c!==MAIN&&c!==h&&of(c)&&all.indexOf(c)===i;});
  if(rest.length){var d=document.createElement('details'),s=document.createElement('summary');s.textContent=say('money',2,'More currencies');d.appendChild(s);
    var more=document.createElement('ul');rest.map(function(c){return [nameOf(c,lang()),c];}).sort(function(a,b){return a[0].localeCompare(b[0],lang());})
      .forEach(function(x){item(more,x[1],x[1],'');});d.appendChild(more);if(open||(shown&&shown!==h))d.open=true;panel.appendChild(d);}
  if(shown){var n=document.createElement('p');n.className='note';n.textContent=noteText(shown);panel.appendChild(n);}}
wrap(art);
// Text the page's own scripts write later (a calculator's result) is converted as it comes.
var watch=new MutationObserver(function(list){var again=false;
  list.forEach(function(r){if(r.type==='attributes'){again=true;return;}
    var el=r.target.nodeType===3?r.target.parentNode:r.target;if(!el||!el.closest||el.closest(SKIP))return;
    if(r.type==='characterData'||r.addedNodes.length){wrap(el);again=true;}});
  if(again)quiet(function(){if(lang()!==was){was=lang();moved();}else show(shown);});});
function observe(){watch.observe(art,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['lang']});}
// The switch's own changes to the article are not news to it.
function quiet(fn){watch.disconnect();fn();watch.takeRecords();observe();}
var was=lang();btn.hidden=false;choice=kept();show(target(choice));observe();
})();
