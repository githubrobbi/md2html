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
  // Holds a block at a height on the screen while the page settles: images
  // above it that load late (lazily, or slowly) would push it away, and not
  // every browser anchors the scroll itself (Safari does not). Each time the
  // page grows, the block is put back; until the reader scrolls, or a few
  // seconds have passed.
  window.ivPin=function(el,off){
    var stop=false,put=function(){if(!stop)scrollTo(0,Math.max(0,el.getBoundingClientRect().top+scrollY-off));};
    put();var ro=window.ResizeObserver?new ResizeObserver(put):null;if(ro)ro.observe(document.body);
    addEventListener('load',put);
    // A jump the reader did not make (a browser handling the address, a late
    // layout) is undone too; the reader's own wheel, touch or keys end the hold.
    var want=function(){return Math.max(0,el.getBoundingClientRect().top+scrollY-off);};
    var back=function(){if(!stop&&Math.abs(scrollY-want())>4)requestAnimationFrame(put);};
    addEventListener('scroll',back,{passive:true});
    var end=function(){if(stop)return;stop=true;if(ro)ro.disconnect();removeEventListener('scroll',back);
      ['wheel','touchstart','keydown','mousedown'].forEach(function(t){removeEventListener(t,end,true);});};
    ['wheel','touchstart','keydown','mousedown'].forEach(function(t){addEventListener(t,end,true);});
    setTimeout(end,6000);
  };
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
    share(here()+'#'+id,(h.textContent||'').trim());
  });
  // A table row: the share symbol at its end while the pointer is on it.
  var rowBtn=null,rowOn=null;
  document.addEventListener('mouseover',function(e){
    var tr=e.target.closest&&e.target.closest('article tbody tr[id]');
    if(!tr||tr===rowOn)return;rowOn=tr;
    var glyph=document.querySelector('a.anchor svg');if(!glyph)return;
    if(!rowBtn){rowBtn=document.createElement('button');rowBtn.type='button';rowBtn.id='rowshare';rowBtn.className='btn icon';
      rowBtn.setAttribute('aria-label',(UI.share&&UI.share[2])||'Link to this row');rowBtn.title=rowBtn.getAttribute('aria-label');
      rowBtn.appendChild(glyph.cloneNode(true));document.body.appendChild(rowBtn);
      rowBtn.addEventListener('click',function(){if(!rowOn)return;var id=rowOn.id;
        try{history.replaceState(null,'','#'+id);}catch(x){}
        share(here()+'#'+id,(rowOn.cells[0]&&rowOn.cells[0].textContent||document.title).trim());});}
    // At the row's end: the right, or the left where the page reads right to left.
    var r=tr.getBoundingClientRect(),box=tr.closest('.scroll'),b=box?box.getBoundingClientRect():r,rtl=getComputedStyle(tr).direction==='rtl';
    var x=rtl?Math.max(r.left,b.left)-34:Math.min(r.right,b.right)+6;
    x=Math.max(4,Math.min(x,innerWidth-34));
    rowBtn.style.top=(r.top+scrollY+r.height/2-14)+'px';rowBtn.style.left=(x+scrollX)+'px';rowBtn.hidden=false;
  });
  document.addEventListener('mouseover',function(e){if(rowBtn&&!e.target.closest('article tbody tr[id],#rowshare')){rowBtn.hidden=true;rowOn=null;}});
  // Placed for one window size: a new size puts it away until the pointer comes back.
  addEventListener('resize',function(){if(rowBtn){rowBtn.hidden=true;rowOn=null;}if(chip)chip.hidden=true;});
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
    if(!chip){chip=document.createElement('button');chip.type='button';chip.id='passage';chip.className='btn';
      // The heading's share glyph, then the words: the same action, for a passage.
      var glyph=document.querySelector('a.anchor svg');if(glyph)chip.appendChild(glyph.cloneNode(true));
      var words=document.createElement('span');words.textContent=SAY[1];chip.appendChild(words);document.body.appendChild(chip);
      chip.addEventListener('click',function(){var q=passage();if(q)share(q.url,document.title);hide();});}
    chip.hidden=false;chip.style.top=Math.max(8,p.rect.top+scrollY-44)+'px';
    chip.style.left=Math.min(Math.max(8,p.rect.left+scrollX+p.rect.width/2-chip.offsetWidth/2),scrollX+innerWidth-chip.offsetWidth-8)+'px';},0);});
  document.addEventListener('keydown',function(e){if(e.key==='Escape')hide();});
})();
