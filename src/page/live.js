// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// Live pages: a page that changes reloads itself and keeps the reader's
// place. Served by `md2html --serve`, it reloads the moment its Markdown
// is saved (the local server says so). With `--live` the page has its own
// time to live: when it runs out (and when the reader comes back to the
// tab) the page asks the server it came from for the first kilobyte of
// itself, where its fingerprint is, and reloads only if that changed;
// opened from a file, which a page may not read, it simply reloads.
// Offline, the question simply goes unanswered.
//
// The place is kept best effort: the block at the top of the screen (a
// paragraph, an item, a table...), found again by its text in its section,
// else by its text anywhere, else by the closest text in the section, else
// by its position there; else the section itself, or the nearest one
// before it that is still there; else, the page having changed beyond
// recognition, the top.
(function(){
  var root=document.documentElement,mode=root.getAttribute('data-live'),art=document.querySelector('article');
  if(!mode||!art)return;
  var KEY='md2html-place:'+location.pathname,DONE='md2html-updated:'+location.pathname,BAR=62;
  var UI={};try{UI=JSON.parse(document.getElementById('md2html-ui').textContent)||{};}catch(e){}
  var SEL='h2[id],h3[id],h4[id],p,li,pre,table,figure,blockquote,.callout,.diagram,dt,dd';
  function norm(t){return (t||'').replace(/\s+/g,' ').trim().slice(0,160);}
  // The blocks in reading order, each with the section (h2 or h3) it is in.
  function blocks(){var out=[],sec=null;
    [].forEach.call(art.querySelectorAll(SEL),function(el){
      if(el.closest('nav,footer.fine,.credit,#glossary'))return;
      if(/^H[23]$/.test(el.tagName))sec=el.id;
      out.push({el:el,sec:sec,text:norm(el.textContent)});});
    return out;}
  function heads(){return [].map.call(art.querySelectorAll('h2[id],h3[id]'),function(h){return h.id;});}
  function save(){
    var st={top:true};
    if(scrollY>8){
      var list=blocks();
      for(var i=0;i<list.length;i++){var r=list[i].el.getBoundingClientRect();
        if(r.height&&r.bottom>BAR+4){var b=list[i],same=list.filter(function(x){return x.sec===b.sec;});
          st={sec:b.sec,heads:heads(),text:b.text,tag:b.el.tagName,idx:same.indexOf(b),off:r.top};break;}}}
    try{sessionStorage.setItem(KEY,JSON.stringify(st));}catch(e){}
  }
  function go(el,off){var y=el.getBoundingClientRect().top+scrollY-(off==null?BAR+18:off);scrollTo(0,Math.max(0,y));}
  function restore(st){
    if(!st||st.top){scrollTo(0,0);return;}
    var list=blocks(),same=list.filter(function(x){return x.sec===st.sec;}),hit=null;
    // 1. The same text in the same section.
    same.forEach(function(x){if(!hit&&x.text===st.text&&x.el.tagName===st.tag)hit=x;});
    // 2. The same text anywhere, when it is there once.
    if(!hit){var all=list.filter(function(x){return x.text&&x.text===st.text;});if(all.length===1)hit=all[0];}
    // 3. The closest text in the section: the longest common start, if it is long enough to mean anything.
    if(!hit&&same.length){var best=null,len=23;same.forEach(function(x){var k=0;while(k<x.text.length&&k<st.text.length&&x.text[k]===st.text[k])k++;if(k>len){len=k;best=x;}});hit=best;}
    // 4. The same place in the section.
    if(!hit&&same.length&&st.idx>=0)hit=same[Math.min(st.idx,same.length-1)];
    if(hit){go(hit.el,st.off);return;}
    // 5. The section, or the nearest one before it that is still there.
    for(var i=st.heads.indexOf(st.sec);i>=0;i--){var h=document.getElementById(st.heads[i]);if(h){go(h);return;}}
    // 6. Nothing is where it was: from the top.
    scrollTo(0,0);
  }
  var saved=null;try{saved=JSON.parse(sessionStorage.getItem(KEY)||'null');sessionStorage.removeItem(KEY);}catch(e){}
  if(saved){
    if('scrollRestoration' in history)history.scrollRestoration='manual';
    var again=function(){restore(saved);};
    if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',again);else again();
    addEventListener('load',again);
    var updated=false;try{updated=sessionStorage.getItem(DONE)==='1';sessionStorage.removeItem(DONE);}catch(e){}
    if(updated)setTimeout(function(){if(window.ivTell)window.ivTell(UI.live||'Updated');},350);
  }
  // A reload waits while the reader is busy: a selection, an open menu, search or a figure.
  function busy(){var s=getSelection();
    return (s&&!s.isCollapsed)||!!document.querySelector(':popover-open,dialog[open],.zoomed,#zoom.on');}
  function reload(){if(busy()){setTimeout(reload,2500);return;}
    save();try{sessionStorage.setItem(DONE,'1');}catch(e){}location.reload();}
  if(mode==='serve'&&window.EventSource){
    var first=null,es=new EventSource('/__md2html/events');
    es.onmessage=function(e){if(first===null){first=e.data;return;}if(e.data!==first)reload();};
  } else if(mode==='check'){
    // The page's own time to live. On a server, when it runs out, the page
    // asks the server it came from whether it changed (its fingerprint, in
    // the first kilobyte) and reloads only if so. Opened from a file,
    // where a page may not read files, it simply reloads, keeping the place.
    var ttl=Math.max(5,parseInt(root.getAttribute('data-ttl'),10)||120)*1000;
    if(/^https?:$/.test(location.protocol)&&window.fetch){
      // The page's fingerprint is in its first kilobyte: only that is asked for.
      var own=(document.querySelector('meta[name=md2html-page]')||{}).content,url=location.href.split('#')[0];
      var ask=function(){if(document.hidden||!own)return;
        fetch(url,{headers:{Range:'bytes=0-1023'},cache:'no-store',credentials:'same-origin'}).then(function(r){
          return r.ok?r.text():'';}).then(function(t){
          var m=/name="md2html-page" content="([0-9a-f]+)"/.exec(t||'');if(m&&m[1]!==own)reload();}).catch(function(){});};
      setInterval(ask,ttl);
      document.addEventListener('visibilitychange',function(){if(!document.hidden)ask();});
    } else {
      var due=Date.now()+ttl;
      setInterval(function(){if(!document.hidden&&Date.now()>=due){due=Date.now()+ttl;reload();}},1000);
    }
  }
})();
