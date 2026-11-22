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
})();
