// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// The theme switch and the text size shared by every handbook page:
// system theme by default, a choice stamped on the root and remembered
// per browser; the text size as a zoom of the page alone (never the
// browser's, which reaches every tab), in steps, remembered the same
// way. Both run before paint so a remembered choice never flashes the
// other. The tooltips say what is active and what the click does next.
// (Carried over from docenta's page layer as of 2026-10-11; D86.)
(function(){
  var root=document.documentElement;
  var UI={};try{UI=JSON.parse(document.getElementById('md2html-ui').textContent)||{};}catch(e){}
  function say(k,i,d){var v=UI[k];if(v&&i!=null)v=v[i];return v||d;}
  function fill(t,o){return t.replace(/\{(\w+)\}/g,function(m,k){return o[k]!=null?o[k]:m;});}
  var KEY='iv-theme';
  function saved(){try{return localStorage.getItem(KEY)||'';}catch(e){return '';}}
  function apply(v){if(v==='light'||v==='dark'){root.setAttribute('data-theme',v);}else{root.removeAttribute('data-theme');}}
  apply(saved());
  var SKEY='iv-size',SIZES=[85,100,115,130,150,175,200];
  function sizeSaved(){try{var n=parseInt(localStorage.getItem(SKEY),10);return SIZES.indexOf(n)>=0?n:100;}catch(e){return 100;}}
  function sizeApply(n){root.style.setProperty('--zoom',String(n/100));}
  function sizeStore(n){try{if(n===100){localStorage.removeItem(SKEY);}else{localStorage.setItem(SKEY,String(n));}}catch(e){}}
  sizeApply(sizeSaved());
  // The outline's remembered state (hidden or shown), stamped on the root
  // before paint so a hidden outline never flashes into view; the page's
  // own script owns the switch and the drawer.
  try{if(localStorage.getItem('iv-toc')==='off'){root.setAttribute('data-toc','off');}}catch(e){}
  document.addEventListener('DOMContentLoaded',function(){
    // The theme: System, Light, Dark, side by side; the one in force is pressed.
    var picks=document.querySelectorAll('.themes button[data-set]');
    function mark(){var v=saved();for(var i=0;i<picks.length;i++){picks[i].setAttribute('aria-pressed',String(picks[i].getAttribute('data-set')===v));}}
    for(var i=0;i<picks.length;i++){picks[i].addEventListener('click',function(){var v=this.getAttribute('data-set');
      try{if(v){localStorage.setItem(KEY,v);}else{localStorage.removeItem(KEY);}}catch(e){}apply(v);mark();});}
    mark();
    var up=document.getElementById('size-up'),down=document.getElementById('size-down'),show=document.getElementById('size');
    if(up&&down&&show){
      function sizeLabel(){
        var n=sizeSaved(),i=SIZES.indexOf(n);
        show.textContent=n+'%';
        show.title=(n===100?say('size',3,'Text size: 100%, the page as written.'):fill(say('size',4,'Text size: {n}%. Click to go back to 100%.'),{n:n}))+say('size',5,'');
        show.setAttribute('aria-label',show.title);
        down.disabled=i<=0;up.disabled=i>=SIZES.length-1;
        down.title=down.disabled?say('size',6,'Smallest text already'):fill(say('size',1,'Smaller text, {n}%'),{n:SIZES[i-1]});
        up.title=up.disabled?say('size',7,'Largest text already'):fill(say('size',2,'Larger text, {n}%'),{n:SIZES[i+1]});
        down.setAttribute('aria-label',down.title);up.setAttribute('aria-label',up.title);
      }
      function set(n){sizeStore(n);sizeApply(n);sizeLabel();}
      function step(d){var i=SIZES.indexOf(sizeSaved())+d;if(i>=0&&i<SIZES.length){set(SIZES[i]);}}
      sizeLabel();
      up.addEventListener('click',function(){step(1);});
      down.addEventListener('click',function(){step(-1);});
      show.addEventListener('click',function(){set(100);});
      // Command (Control on Windows) with + - 0: on this page these size
      // THIS page, and the browser's zoom, which reaches every tab, is not
      // invoked (the event is taken before the browser acts).
      document.addEventListener('keydown',function(e){
        if(!(e.metaKey||e.ctrlKey)||e.altKey||e.shiftKey&&e.code!=='Equal')return;
        if(e.code==='Equal'||e.code==='NumpadAdd'){step(1);e.preventDefault();}
        else if(e.code==='Minus'||e.code==='NumpadSubtract'){step(-1);e.preventDefault();}
        else if(e.code==='Digit0'||e.code==='Numpad0'){set(100);e.preventDefault();}
      });
    }
  });
})();
