import { ArrowUpRight, BrainCircuit, Calculator, Code2, Globe2, Languages, Layers3, LockKeyhole, MessageSquare, Plus, Settings2, ShieldCheck, Sparkles } from 'lucide-react';
import type { AppSettings, Conversation, Mode } from './types';
import AevraMark from './AevraMark';
import './dashboard.css';

const areas = [
  {mode:'chat' as Mode, label:'Sohbet', description:'Birlikte düşün, yaz ve keşfet.', icon:MessageSquare},
  {mode:'translate' as Mode, label:'Çeviri', description:'Kelimelerini başka dillere taşı.', icon:Languages},
  {mode:'research' as Mode, label:'Araştırma', description:'Kaynaklarıyla daha fazlasını öğren.', icon:Globe2},
  {mode:'math' as Mode, label:'Matematik', description:'Problemleri adım adım çöz.', icon:Calculator},
  {mode:'code' as Mode, label:'Kodlama', description:'Fikirlerini çalışan çözümlere dönüştür.', icon:Code2},
];
const starters = [
  {mode:'translate' as Mode,label:'Metin çevir',detail:'Yeni bir dilde anlat',icon:Languages,text:''},
  {mode:'research' as Mode,label:'Bir konuyu araştır',detail:'Kaynaklarıyla keşfet',icon:Globe2,text:''},
  {mode:'math' as Mode,label:'Bir problem çöz',detail:'Adım adım ilerle',icon:Calculator,text:'Bir matematik problemini adım adım çözmeme yardım et.'},
  {mode:'code' as Mode,label:'Bir şey geliştir',detail:'Fikrini koda dönüştür',icon:Code2,text:'Yeni bir proje geliştirmek istiyorum. Başlamak için bana birkaç soru sor.'},
];
interface Props {
  preferences:AppSettings;
  conversations:Conversation[];
  disabled:boolean;
  onStart:(mode:Mode,text?:string)=>void;
  onOpen:(conversation:Conversation)=>void;
  onSettings:(section:string)=>void;
  onFocus:()=>void;
}
export default function HomeDashboard({preferences,conversations,disabled,onStart,onOpen,onSettings,onFocus}:Props){
  const recent=conversations.filter(item=>!item.archived).slice(0,3);
  const date=new Intl.DateTimeFormat('tr-TR',{day:'numeric',month:'long',weekday:'long',timeZone:'Europe/Istanbul'}).format(new Date());
  const name=preferences.preferredName.trim();
  const style={concise:'Kısa ve net',balanced:'Dengeli',detailed:'Ayrıntılı'}[preferences.responseStyle];
  return <section className="dashboard" aria-label="Kişisel çalışma alanı">
    <header className="dashboard-hero">
      <div className="dashboard-date">{date}</div>
      <div className="hero-bottom"><div><h1>Merhaba{name?`, ${name}`:''}<span className="greeting-dot">.</span></h1><p>Bugün nasıl yardımcı olabilirim?</p></div>
        <div className="hero-actions"><button className="ask-aevra" type="button" disabled={disabled} onClick={onFocus}><Sparkles size={14}/>Aevra’ya sor</button>{preferences.showSuggestions&&<><button type="button" disabled={disabled} onClick={()=>onStart('translate')}>Metin çevir<ArrowUpRight size={12}/></button><button type="button" disabled={disabled} onClick={()=>onStart('research')}>Web’de araştır<ArrowUpRight size={12}/></button></>}</div>
      </div>
    </header>
    <div className="dashboard-grid">
      <div className="dashboard-column">
        <section className="dashboard-card areas-card" aria-labelledby="areas-title">
          <header className="card-heading"><h2 id="areas-title"><Layers3 size={16}/>Çalışma alanların</h2><span>5 alan</span></header>
          <div className="area-table-label"><span>Nereden başlamak istersin?</span><span>Çalışma şekli</span></div>
          <div className="area-list">{areas.map(area=><button type="button" key={area.mode} disabled={disabled} className="area-row" aria-label={`${area.label} alanını aç`} onClick={()=>onStart(area.mode)}>
            <span className={`area-icon tone-${area.mode}`}><area.icon size={16}/></span><span className="area-title"><strong>{area.label}</strong><small>{area.description}</small></span>
            <span className={`area-tag ${area.mode==='research'?'web-tag':''}`}>{area.mode==='research'?preferences.webResearchEnabled?'Google':'Web kapalı':'Yerel'}</span><ArrowUpRight className="area-arrow" size={14}/>
          </button>)}</div>
          <button className="card-footer-action" disabled={disabled} type="button" onClick={()=>onStart('chat')}><Plus size={13}/>Yeni bir konuşma başlat</button>
        </section>
        <section className="dashboard-card recent-card" aria-labelledby="recent-title">
          <header className="card-heading"><h2 id="recent-title"><MessageSquare size={15}/>Kaldığın yerden devam et</h2><span>{conversations.filter(item=>!item.archived).length} konuşma</span></header>
          {recent.length?<div className="dashboard-recents">{recent.map(item=><button type="button" key={item.id} disabled={disabled} aria-label={`Son konuşmayı aç: ${item.title}`} onClick={()=>onOpen(item)}><span className={`recent-dot tone-${item.mode}`} aria-hidden="true"/><span><strong>{item.title}</strong><small>{areas.find(area=>area.mode===item.mode)?.label} · {new Date(item.updatedAt).toLocaleDateString('tr-TR',{day:'numeric',month:'short'})}</small></span><ArrowUpRight size={14}/></button>)}</div>:<div className="recent-empty"><span className="empty-orbit"><MessageSquare size={17}/></span><div><strong>İlk fikrine yer açalım.</strong><p>Konuşmaların burada seni bekleyecek.</p></div><button type="button" disabled={disabled} onClick={onFocus} aria-label="İlk konuşmana başla"><ArrowUpRight size={17}/></button></div>}
        </section>
      </div>
      <div className="dashboard-column">
        {preferences.showSuggestions&&<section className="dashboard-card quick-card" aria-labelledby="quick-title"><header className="card-heading"><h2 id="quick-title"><Sparkles size={16}/>Küçük bir başlangıç</h2><span>Birlikte üretelim</span></header><div className="quick-grid">{starters.map(starter=><button type="button" key={starter.mode} disabled={disabled} aria-label={`Hızlı başlangıç: ${starter.label}`} onClick={()=>onStart(starter.mode,starter.text)}><span className={`quick-glyph tone-${starter.mode}`}><starter.icon size={21}/></span><span><strong>{starter.label}</strong><small>{starter.detail}</small></span></button>)}</div></section>}
        <section className="dashboard-card preference-card" aria-labelledby="preference-title"><header className="card-heading"><h2 id="preference-title"><Settings2 size={16}/>Tam sana göre</h2><button type="button" disabled={disabled} aria-label="Kişiselleştirme ayarlarını aç" onClick={()=>onSettings('personalization')}><ArrowUpRight size={15}/></button></header>
          <button className="preference-line" type="button" disabled={disabled} onClick={()=>onSettings('personalization')}><span>Yanıt tarzı</span><strong>{style}</strong><ArrowUpRight size={12}/></button>
          <button className="preference-line" type="button" disabled={disabled} onClick={()=>onSettings('model')}><span>Çeviri dili</span><strong>{preferences.translationLanguage}</strong><ArrowUpRight size={12}/></button>
          <button className="preference-line" type="button" disabled={disabled} onClick={()=>onSettings('memory')}><span>Kişisel hafıza</span><strong className={`preference-state ${preferences.memoryEnabled?'enabled':''}`}><span/>{preferences.memoryEnabled?'Açık':'Kapalı'}</strong><ArrowUpRight size={12}/></button>
        </section>
        <section className="dashboard-card local-card"><span className="local-card-symbol"><AevraMark/></span><div><h2>Senin alanın. Senin kontrolün.</h2><p>Sohbet geçmişin bu bilgisayarda kalır.</p><span><ShieldCheck size={12}/>Yerel model<LockKeyhole size={12}/>Kalıcı geçmiş</span></div><button type="button" disabled={disabled} aria-label="Veri kontrollerini aç" onClick={()=>onSettings('data')}><ArrowUpRight size={16}/></button></section>
        <button className="memory-shortcut" type="button" disabled={disabled} onClick={()=>onSettings('memory')}><BrainCircuit size={14}/><span>Aevra’yı zamanla sana daha yakın hâle getir.</span><ArrowUpRight size={13}/></button>
      </div>
    </div>
  </section>;
}
