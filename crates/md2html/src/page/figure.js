// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// A figure, enlarged over the page. A click lifts it out of the page to
// the size of the window; two more bring it closer; the next one, the
// dimmed edge, the cross or Escape puts it back, and the reader has not
// left their place. Larger than the window, the image follows the
// pointer: point at a corner and that corner comes into view. A finger
// drags it instead. With a modifier key the click is the browser's, and
// opens the file itself.
//
// A diagram md2html drew (crate::mermaid) enlarges the same way. Its
// drawing is copied into the enlargement as it stands, not made into an
// image: the copy takes its colours and its type from the page's style,
// in either theme and when the theme changes, and stays sharp at every
// step. What is copied is md2html's own drawing, already in the page; an
// author's HTML of the same shape has been through the sanitizer like
// the rest of the page, and a copied node runs no script. A picture's
// link takes the keyboard's focus; a diagram is given a button for it,
// here and not in the HTML, so that a page read without scripts (or a
// book) has no control that does nothing.
(function(){
  if(!window.HTMLDialogElement)return;
  var UI={};try{UI=JSON.parse(document.getElementById('md2html-ui').textContent)||{};}catch(e){}
  function say(i,d){return (UI.figure&&UI.figure[i])||d;}
  var STEPS=[1,2,3],EDGE=48,ROOM=64,box=null,shot,draw,big,note,tag,cross,from=null,back=null,w=0,h=0,lvl=0,tx=0,ty=0,px=.5,py=.5,
      touch=false,drag=null,dragged=false,closing=false;
  function unit(n){return n<0?0:n>1?1:n;}
  function still(){return window.matchMedia&&window.matchMedia('(prefers-reduced-motion:reduce)').matches;}
  // The shape of what is enlarged: a drawing's from its viewBox; an
  // image's own, the figure's if it has loaded, the enlargement's once it
  // has (a lazy image clicked before it loaded has none yet, and its box
  // is mostly padding).
  function shape(r){
    var v=from.viewBox&&from.viewBox.baseVal;if(v&&v.width&&v.height)return v.width/v.height;
    var nw=from.naturalWidth||shot.naturalWidth,nh=from.naturalHeight||shot.naturalHeight;
    return nw&&nh?nw/nh:(r.height>40?r.width/r.height:4/3);
  }
  // The first step: the whole figure in the window, when that is plainly
  // larger than the figure on the page; a quarter larger than the figure
  // when the window cannot give that (a low window, or large text).
  function size(){
    var r=from.getBoundingClientRect(),ar=shape(r);
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
    tag.textContent=say(3,'{i} of {n}').replace('{i}',lvl+1).replace('{n}',STEPS.length);
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
  // A copy of a drawing whose names (its title, its arrowheads) are its
  // own: what refers to them in the copy finds the copy's, and no name
  // stands twice in the page.
  function copy(svg){
    var c=svg.cloneNode(true);
    [c].concat([].slice.call(c.querySelectorAll('*'))).forEach(function(el){
      [].slice.call(el.attributes).forEach(function(at){
        var n=at.name,v=at.value,own;
        if(n==='id')own=v+'-z';
        else if(n==='aria-labelledby'||n==='aria-describedby')own=v.replace(/\S+/g,'$&-z');
        else own=v.replace(/url\(#([^)]+)\)/g,'url(#$1-z)');
        if(own!==v)el.setAttribute(n,own);
      });
    });
    return c;
  }
  // A figure's picture or drawing, enlarged; `ctl`, what opened it, has
  // the focus back when it is put back.
  function open(fig,e,ctl){
    if(!box)build();
    var svg=fig.querySelector(':scope>svg'),said=fig.querySelector(':scope>figcaption');
    back=ctl;big=svg?draw:shot;shot.hidden=!!svg;draw.hidden=!svg;
    if(svg){from=svg;draw.textContent='';draw.appendChild(copy(svg));}
    else{
      from=fig.querySelector('img');
      shot.onload=function(){if(box.open&&!closing&&big===shot){size();place();}};
      shot.src=from.currentSrc||from.src;shot.alt=from.alt;
    }
    note.textContent=said?said.textContent:'';
    lvl=0;closing=false;px=py=.5;
    // What opened it has the focus, as a clicked link has: the dialog
    // takes it from there and gives it back. Opened with the pointer,
    // neither shows the keyboard's ring; a click from the keyboard has no
    // place, and the figure opens at its middle.
    var keys=!e.detail;
    if(ctl&&document.activeElement!==ctl)ctl.focus({preventScroll:true,focusVisible:keys});
    box.classList.add('held');box.showModal();size();
    if(!keys&&cross.matches(':focus-visible')){cross.blur();cross.focus({focusVisible:false});}
    if(!touch&&!keys)point(e);
    big.style.transform=home();void big.offsetWidth;
    box.classList.remove('held');box.classList.add('in');from.style.visibility='hidden';
    place();
  }
  function shut(){
    if(!box||!box.open||closing)return;
    closing=true;box.classList.remove('in');big.style.transform=home();
    var done=function(){
      from.style.visibility='';box.close();closing=false;
      if(back&&back.isConnected)back.focus({preventScroll:true});
    };
    if(still()){done();}else{setTimeout(done,170);}
  }
  function build(){
    box=document.createElement('dialog');box.className='zoom';
    box.setAttribute('aria-label',say(0,'The figure, enlarged'));
    box.innerHTML='<img alt="" draggable="false"><div class="diagram"></div><p></p><span></span><button type="button">×</button>';
    cross=box.lastChild;cross.setAttribute('aria-label',say(1,'Close'));cross.title=say(2,'Close (Escape)');
    document.body.appendChild(box);shot=box.firstChild;draw=shot.nextSibling;note=draw.nextSibling;tag=note.nextSibling;
    box.addEventListener('cancel',function(e){e.preventDefault();shut();});
    box.addEventListener('click',function(e){
      if(dragged){dragged=false;return;}
      if(!big.contains(e.target)||lvl===STEPS.length-1||e.button){shut();return;}
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
  // Each diagram's button, before its drawing: seen when it has the focus.
  [].forEach.call(document.querySelectorAll('figure.diagram>svg'),function(svg){
    var b=document.createElement('button');b.type='button';b.className='btn enlarge';
    b.setAttribute('aria-haspopup','dialog');b.textContent=say(4,'Enlarge the diagram');
    svg.parentNode.insertBefore(b,svg);
  });
  document.addEventListener('pointerdown',function(e){if(!box||!box.open)touch=e.pointerType!=='mouse';},true);
  document.addEventListener('click',function(e){
    var on=e.target.closest('figure.fig>a,figure.diagram>svg,figure.diagram>.enlarge');
    if(!on||e.button||e.metaKey||e.ctrlKey||e.shiftKey||e.altKey)return;
    var fig=on.parentNode;
    e.preventDefault();open(fig,e,on.localName==='svg'?fig.querySelector(':scope>.enlarge'):on);
  });
})();
