import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ArrowLeft, ArrowRight, Globe2, Hand, LoaderCircle, PanelRightClose, RotateCw, Search, Trash2 } from 'lucide-react';
import { api, desktop } from './api';
import { readableError } from './safety';
import type { BrowserAction, BrowserStatus } from './types';
import './research-browser.css';

const empty: BrowserStatus = {url:'',title:'',loading:false,automating:false,detail:'',steps:[]};
export default function ResearchBrowser({ visible, suspended, enabled, active=true, onClose, onError }: { visible:boolean; suspended:boolean; enabled:boolean; active?:boolean; onClose:()=>void; onError:(message:string)=>void }) {
  const [status,setStatus]=useState<BrowserStatus>(empty);
  const [address,setAddress]=useState('');
  const [working,setWorking]=useState(false);
  const [historyOpen,setHistoryOpen]=useState(false);
  const viewport=useRef<HTMLDivElement>(null);
  const layoutQueue=useRef<Promise<void>>(Promise.resolve());
  const errorRef=useRef(onError); errorRef.current=onError;
  useEffect(()=>{
    let alive=true; let off:(()=>void)|undefined;
    void api.onBrowserUpdated(value=>{if(alive)setStatus(value);}).then(fn=>{if(alive)off=fn;else fn();}).catch(e=>errorRef.current(readableError(e)));
    void api.browserStatus().then(value=>{if(alive)setStatus(value);}).catch(e=>errorRef.current(readableError(e)));
    return()=>{alive=false;off?.();};
  },[]);
  useEffect(()=>{setAddress(status.url);},[status.url]);
  useEffect(()=>{if(!active)layoutQueue.current=layoutQueue.current.catch(()=>{}).then(()=>api.browserClose()).catch(e=>errorRef.current(readableError(e)));},[active]);
  useEffect(()=>{
    function update(){
      const box=visible&&!suspended&&enabled?viewport.current?.getBoundingClientRect():null;
      const rect=box&&box.width>=1&&box.height>=1?{x:box.x,y:box.y,width:box.width,height:box.height}:null;
      layoutQueue.current=layoutQueue.current.catch(()=>{}).then(()=>api.browserLayout(rect)).catch(e=>errorRef.current(readableError(e)));
    }
    update();
    const observer=typeof ResizeObserver!=='undefined'?new ResizeObserver(update):null;
    if(viewport.current)observer?.observe(viewport.current);
    window.addEventListener('resize',update);
    return()=>{observer?.disconnect();window.removeEventListener('resize',update);layoutQueue.current=layoutQueue.current.catch(()=>{}).then(()=>api.browserLayout(null)).catch(()=>{});};
  },[visible,suspended,enabled,historyOpen]);
  async function control(action:BrowserAction){setWorking(true);try{await api.browserControl(action);}catch(e){onError(readableError(e));}finally{setWorking(false);}}
  function navigate(event:FormEvent){event.preventDefault();const url=address.trim();if(url)void control({action:'navigate',url:/^https:\/\//i.test(url)?url:`https://${url}`});}
  async function takeover(){setWorking(true);try{await api.browserTakeover();}catch(e){onError(readableError(e));}finally{setWorking(false);}}
  if(!visible)return null;
  const disabled=!desktop||!enabled||working||status.automating;
  return <aside className="research-browser" aria-label="Araştırma tarayıcısı">
    <header className="browser-heading"><span><Globe2 size={16}/><strong>Araştırma tarayıcısı</strong></span><button type="button" onClick={onClose} title="Tarayıcıyı küçült" aria-label="Tarayıcıyı küçült"><PanelRightClose size={17}/></button></header>
    <div className="browser-navigation"><button type="button" aria-label="Önceki sayfa" disabled={disabled||!status.url} onClick={()=>void control({action:'back'})}><ArrowLeft size={15}/></button><button type="button" aria-label="Sonraki sayfa" disabled={disabled||!status.url} onClick={()=>void control({action:'forward'})}><ArrowRight size={15}/></button><button type="button" aria-label="Sayfayı yenile" disabled={disabled||!status.url} onClick={()=>void control({action:'reload'})}><RotateCw size={14}/></button><form onSubmit={navigate}><input aria-label="Web adresi" placeholder="Bir site adresi yaz…" value={address} disabled={disabled} onChange={event=>setAddress(event.target.value)}/><button type="submit" aria-label="Adresi aç" disabled={disabled||!address.trim()}><ArrowRight size={14}/></button></form></div>
    <div className="browser-activity" role="status">{status.automating||status.loading?<LoaderCircle className="browser-spinner" size={14}/>:<Search size={14}/>}<span title={status.detail}>{!enabled?'Web araştırması ayarlardan kapalı.':status.detail||'Konuyu söyle; Aevra burada araştırsın.'}</span></div>
    <div className="browser-viewport" ref={viewport} aria-label="Web sayfası">
      {(!status.url||!desktop||!enabled)&&<div className="browser-empty"><Globe2 size={33}/><h2>Araştırmayı takip et.</h2><p>Bir konu yazdığında Aevra arama yapar, sayfaları açar ve bulduklarını kaynaklarıyla açıklar.</p><small>{desktop?'İstersen bir site adresi açıp “Bu sayfayı incele” de diyebilirsin.':'Web sayfaları masaüstü uygulamasında açılır.'}</small></div>}
    </div>
    <div className="browser-controls"><button type="button" className="browser-takeover" disabled={!desktop||!status.automating||working} onClick={()=>void takeover()}><Hand size={14}/>Kontrolü al</button><button type="button" disabled={!status.steps.length} aria-expanded={historyOpen} onClick={()=>setHistoryOpen(value=>!value)}>İşlemler{status.steps.length?` · ${status.steps.length}`:''}</button><button type="button" title="Tarayıcı verilerini temizle" aria-label="Tarayıcı verilerini temizle" disabled={disabled||!status.url} onClick={()=>void control({action:'clear'})}><Trash2 size={14}/></button></div>
    {historyOpen&&<ol className="browser-history" aria-label="Araştırma adımları">{status.steps.map((entry,index)=><li key={index}>{entry.url?<button type="button" disabled={disabled} title={entry.url} onClick={()=>void control({action:'navigate',url:entry.url})}>{entry.text}</button>:<span>{entry.text}</span>}</li>)}</ol>}
    <p className="browser-footnote">{status.automating?'Aevra sayfaları inceliyor. Kontrolü alarak durdurabilirsin.':'Bu sayfayı sohbette sorabilir veya yeni bir araştırma başlatabilirsin.'}</p>
  </aside>;
}
