// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// The behaviour every handbook page shares, in ONE place: the glossary's
// way back, the copy buttons, the landing glow. A page adds only what is
// its own (the handbook: search and the rail). Timing lives in the
// stylesheet's --glow-time token; nothing here carries a duration.
(function(){
  function land(id){
    var el=document.getElementById(id);if(!el)return;
    el.classList.remove('landed');void el.offsetWidth;el.classList.add('landed');
    el.addEventListener('animationend',function(){el.classList.remove('landed');},{once:true});
  }
  window.ivLand=land;
  var UI={};try{UI=JSON.parse(document.getElementById('md2html-ui').textContent)||{};}catch(e){}
  var COPY=(UI.copy&&UI.copy[0])||'copy',COPIED=(UI.copy&&UI.copy[1])||'copied';
  document.addEventListener('click',function(e){
    var a=e.target.closest('a.term');
    if(a){var b=document.querySelector('a.back[data-back="'+a.dataset.key+'"]');if(b)b.setAttribute('href','#'+a.id);return;}
    // A footnote's way back returns to the reference that was followed.
    var f=e.target.closest('a.fnref');
    if(f){var li=document.getElementById(f.getAttribute('href').slice(1)),back=li&&li.querySelector('a.back');if(back)back.setAttribute('href','#'+f.id);return;}
    var c=e.target.closest('.copy');
    if(c){var pre=c.parentNode.querySelector('pre');
      if(pre&&navigator.clipboard){navigator.clipboard.writeText(pre.innerText).then(function(){
        c.textContent=COPIED;c.classList.add('done');
        setTimeout(function(){c.textContent=COPY;c.classList.remove('done');},1400);});}}
  });
  // Sharing. The # beside a heading copies the section's link (on a phone,
  // the system's share sheet opens); a selection offers a link to exactly
  // that passage (a text fragment, #:~:text=). A short notice says so,
  // read aloud too. Nothing leaves the page but what the reader sends.
  var SAY=UI.share||['Link copied','Link to this passage'],note=null,timer=0;
  window.ivTell=function(text){tell(text);};
  function tell(text){if(!note){note=document.createElement('div');note.id='toast';note.setAttribute('role','status');document.body.appendChild(note);}
    note.textContent=text;note.classList.add('on');clearTimeout(timer);timer=setTimeout(function(){note.classList.remove('on');},1800);}
  function put(url){
    if(navigator.clipboard&&navigator.clipboard.writeText){return navigator.clipboard.writeText(url).then(function(){tell(SAY[0]);},function(){fallback(url);});}
    fallback(url);return null;}
  function fallback(url){var t=document.createElement('textarea');t.value=url;t.setAttribute('readonly','');t.style.position='fixed';t.style.opacity='0';
    document.body.appendChild(t);t.select();try{if(document.execCommand('copy'))tell(SAY[0]);}catch(e){}t.remove();}
  var touch=window.matchMedia&&matchMedia('(hover:none) and (pointer:coarse)').matches;
  function share(url,title){
    if(touch&&navigator.share){navigator.share({title:title,url:url}).catch(function(){});return;}
    put(url);}
  var here=function(){return location.href.split('#')[0];};
  document.addEventListener('click',function(e){
    var a=e.target.closest('a.anchor');if(!a)return;
    var id=a.getAttribute('href').slice(1),h=a.parentNode;e.preventDefault();
    try{history.replaceState(null,'','#'+id);}catch(x){}
    share(here()+'#'+id,(h.textContent||'').replace(/#$/,'').trim());
  });
  // A passage: the start and end of the selection, as the text fragment spec writes them.
  var chip=null;
  function frag(t){return encodeURIComponent(t).replace(/-/g,'%2D').replace(/,/g,'%2C').replace(/&/g,'%26');}
  function passage(){var sel=getSelection();if(!sel||sel.isCollapsed||!sel.rangeCount)return null;
    var text=sel.toString().replace(/\s+/g,' ').trim();if(text.length<3)return null;
    var art=document.querySelector('article');if(!art||!art.contains(sel.getRangeAt(0).commonAncestorContainer))return null;
    var words=text.split(' ');
    var spec=words.length<=8?frag(text):frag(words.slice(0,4).join(' '))+','+frag(words.slice(-4).join(' '));
    return {url:here()+'#:~:text='+spec,rect:sel.getRangeAt(0).getBoundingClientRect(),text:text};}
  function hide(){if(chip)chip.hidden=true;}
  document.addEventListener('selectionchange',function(){if(!passage())hide();});
  document.addEventListener('mouseup',function(e){if(e.target.closest('#passage'))return;setTimeout(function(){
    var p=passage();if(!p){hide();return;}
    if(!chip){chip=document.createElement('button');chip.type='button';chip.id='passage';chip.className='btn';chip.textContent=SAY[1];document.body.appendChild(chip);
      chip.addEventListener('click',function(){var q=passage();if(q)share(q.url,document.title);hide();});}
    chip.hidden=false;chip.style.top=Math.max(8,p.rect.top+scrollY-44)+'px';
    chip.style.left=Math.min(Math.max(8,p.rect.left+scrollX+p.rect.width/2-chip.offsetWidth/2),scrollX+innerWidth-chip.offsetWidth-8)+'px';},0);});
  document.addEventListener('keydown',function(e){if(e.key==='Escape')hide();});
})();
