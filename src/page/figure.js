// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// A figure, enlarged over the page. A click lifts it out of the page to
// the size of the window; two more bring it closer; the next one, the
// dimmed edge, the cross or Escape puts it back, and the reader has not
// left their place. Larger than the window, the image follows the
// pointer: point at a corner and that corner comes into view. A finger
// drags it instead. With a modifier key the click is the browser's, and
// opens the file itself.
(function(){
  if(!window.HTMLDialogElement)return;
  var STEPS=[1,2,3],EDGE=48,ROOM=64,box=null,big,note,tag,from=null,w=0,h=0,lvl=0,tx=0,ty=0,px=.5,py=.5,
      touch=false,drag=null,dragged=false,closing=false;
  function unit(n){return n<0?0:n>1?1:n;}
  function still(){return window.matchMedia&&window.matchMedia('(prefers-reduced-motion:reduce)').matches;}
  // The first step: the whole image in the window, when that is plainly
  // larger than the figure on the page; a quarter larger than the figure
  // when the window cannot give that (a low window, or large text).
  function size(){
    var r=from.getBoundingClientRect(),ar=from.naturalWidth&&from.naturalHeight?from.naturalWidth/from.naturalHeight:r.width/r.height;
    w=Math.min(box.clientWidth-24,(box.clientHeight-12-ROOM)*ar);if(w<r.width*1.1)w=r.width*1.25;h=w/ar;
    big.style.width=w+'px';big.style.height=h+'px';
  }
  // Where the figure sits on the page, as the enlargement's own transform:
  // it grows out of that place and goes back into it.
  function home(){var r=from.getBoundingClientRect();return 'translate('+r.left+'px,'+r.top+'px) scale('+(r.width/w)+')';}
  function axis(len,room,p){return len<=room?(room-len)/2:-(len-room)*p;}
  function place(){
    var s=STEPS[lvl],iw=w*s,ih=h*s,W=box.clientWidth,H=box.clientHeight,fits=iw<=W&&ih<=H-ROOM;
    tx=axis(iw,W,px);ty=fits?12+(H-12-ROOM-ih)/2:axis(ih,H,py);
    big.style.transform='translate('+tx+'px,'+ty+'px) scale('+s+')';
    box.classList.toggle('fits',fits);box.classList.toggle('last',lvl===STEPS.length-1);
    tag.textContent=(lvl+1)+' of '+STEPS.length;
  }
  function point(e){
    px=unit((e.clientX-EDGE)/(box.clientWidth-2*EDGE));py=unit((e.clientY-EDGE)/(box.clientHeight-2*EDGE));
  }
  // A step closer. Under a pointer the image already follows it; under a
  // finger the spot that was tapped stays under the finger.
  function closer(e){
    var s=STEPS[lvl],qx=(e.clientX-tx)/(w*s),qy=(e.clientY-ty)/(h*s),W=box.clientWidth,H=box.clientHeight;
    lvl++;s=STEPS[lvl];
    if(touch){
      px=w*s>W?unit((qx*w*s-e.clientX)/(w*s-W)):.5;py=h*s>H?unit((qy*h*s-e.clientY)/(h*s-H)):.5;
    }else{point(e);}
    place();
  }
  function open(a,e){
    if(!box)build();
    from=a.querySelector('img');var said=a.parentNode.querySelector('figcaption');
    big.src=from.currentSrc||from.src;big.alt=from.alt;note.textContent=said?said.textContent:'';
    lvl=0;closing=false;px=py=.5;
    box.classList.add('held');box.showModal();size();
    if(!touch)point(e);
    big.style.transform=home();void big.offsetWidth;
    box.classList.remove('held');box.classList.add('in');from.style.visibility='hidden';
    place();
  }
  function shut(){
    if(!box||!box.open||closing)return;
    closing=true;box.classList.remove('in');big.style.transform=home();
    var done=function(){from.style.visibility='';box.close();closing=false;};
    if(still()){done();}else{setTimeout(done,170);}
  }
  function build(){
    box=document.createElement('dialog');box.className='zoom';
    box.setAttribute('aria-label','The figure, enlarged');
    box.innerHTML='<img alt="" draggable="false"><p></p><span></span><button type="button" aria-label="Close" title="Close (Escape)">×</button>';
    document.body.appendChild(box);big=box.firstChild;note=big.nextSibling;tag=note.nextSibling;
    box.addEventListener('cancel',function(e){e.preventDefault();shut();});
    box.addEventListener('click',function(e){
      if(dragged){dragged=false;return;}
      if(e.target!==big||lvl===STEPS.length-1||e.button){shut();return;}
      closer(e);
    });
    box.addEventListener('pointerdown',function(e){
      touch=e.pointerType!=='mouse';dragged=false;drag=touch?{x:e.clientX,y:e.clientY}:null;
    });
    box.addEventListener('pointermove',function(e){
      if(closing)return;
      if(e.pointerType==='mouse'){touch=false;point(e);place();return;}
      if(!drag)return;
      var dx=e.clientX-drag.x,dy=e.clientY-drag.y,s=STEPS[lvl],W=box.clientWidth,H=box.clientHeight;
      if(!dragged&&Math.abs(dx)+Math.abs(dy)<8)return;
      dragged=true;drag={x:e.clientX,y:e.clientY};box.classList.add('held');
      if(w*s>W)px=unit(px-dx/(w*s-W));
      if(h*s>H)py=unit(py-dy/(h*s-H));
      place();
    });
    var lift=function(){drag=null;box.classList.remove('held');};
    box.addEventListener('pointerup',lift);box.addEventListener('pointercancel',lift);
    // The page behind stays where it is: the wheel and the scrolling keys
    // are the enlargement's while it is open.
    box.addEventListener('wheel',function(e){e.preventDefault();},{passive:false});
    box.addEventListener('keydown',function(e){
      var k=e.key;
      if(e.metaKey||e.ctrlKey||e.altKey)return;
      if(k==='+'||k==='='){if(lvl<STEPS.length-1)lvl++;}
      else if(k==='-'){if(lvl>0)lvl--;}
      else if(k==='ArrowLeft'){px=unit(px-.15);}
      else if(k==='ArrowRight'){px=unit(px+.15);}
      else if(k==='ArrowUp'){py=unit(py-.15);}
      else if(k==='ArrowDown'){py=unit(py+.15);}
      else if(k!=='PageUp'&&k!=='PageDown'&&k!=='Home'&&k!=='End'){return;}
      e.preventDefault();place();
    });
    addEventListener('resize',function(){if(box.open&&!closing){size();place();}});
  }
  document.addEventListener('pointerdown',function(e){if(!box||!box.open)touch=e.pointerType!=='mouse';},true);
  document.addEventListener('click',function(e){
    var a=e.target.closest('figure.fig>a');
    if(!a||e.button||e.metaKey||e.ctrlKey||e.shiftKey||e.altKey)return;
    e.preventDefault();open(a,e);
  });
})();
