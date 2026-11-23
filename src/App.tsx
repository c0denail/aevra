import { useEffect, useRef, useState, type CSSProperties, type FormEvent } from 'react';
import { Archive, ArrowUp, ArrowUpRight, Calculator, ChevronDown, Code2, Download, FileText, Globe2, House, Languages, LockKeyhole, MessageSquare, PanelRightOpen, Paperclip, Plus, Search, Settings2, Sparkles, Square, Trash2, X } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import {normalizeMath,prepareMathAssets} from './mathPdf';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';
import AevraMark from './AevraMark';
import SettingsPanel from './SettingsPanel';
import ResearchBrowser from './ResearchBrowser';
import HomeDashboard from './HomeDashboard';
import { api, desktop } from './api';
import { defaultSettings, languages, playCompletionSound } from './preferences';
import { readableError, safeHttps } from './safety';
import type { Attachment, AppSettings, Conversation, EngineStatus, Message, MemoryNotice, Mode, Source } from './types';

const modes = {
  chat: { label: 'Sohbet', icon: MessageSquare, title: 'Düşüncelerine biraz alan aç.', subtitle: 'Bir soruyla başla. Birlikte düşünelim, yazalım, keşfedelim.', placeholder: 'Aklından ne geçiyor?' },
  translate: { label: 'Çeviri', icon: Languages, title: 'Kelimeler sınır tanımasın.', subtitle: 'Metnini ekle, hedef dili seç. Çevirin bu bilgisayarda kalsın.', placeholder: 'Çevirmek istediğin metni yaz veya yapıştır…' },
  research: { label: 'Araştırma', icon: Globe2, title: 'Merakının izini sür.', subtitle: 'Konuyu birlikte araştır, kaynakları karşılaştır ve sohbet ederek derinleş.', placeholder: 'Hangi konuyu araştırmak istersin?' },
  math: { label: 'Matematik', icon: Calculator, title: 'Problemi birlikte çözelim.', subtitle: 'Denklemler, hesaplamalar ve adım adım anlaşılır çözümler.', placeholder: 'Matematik sorunu veya işlemini yaz…' },
  code: { label: 'Kodlama', icon: Code2, title: 'Fikrini koda dönüştür.', subtitle: 'Kod yaz, hataları incele veya bir çözümün nasıl çalıştığını öğren.', placeholder: 'Ne geliştirmek istersin? Dili ve ihtiyacını yaz…' },
};
const suggestions = [
  { mode: 'chat' as Mode, icon: Sparkles, title: 'Bir fikri birlikte geliştirelim', text: 'Yeni bir proje fikri geliştirmeme yardım et. Önce ilgi alanlarımı sor.' },
  { mode: 'translate' as Mode, icon: Languages, title: 'Bir metni başka dile taşı', text: '' },
  { mode: 'research' as Mode, icon: Globe2, title: 'Kaynaklarıyla keşfe çık', text: '' },
];
function Markdown({ content, sources, onOpenSource }: { content: string; sources: Source[]; onError: (error: string) => void; onOpenSource:(url:string)=>void }) {
  return <ReactMarkdown remarkPlugins={[remarkMath]} rehypePlugins={[[rehypeKatex, { trust: false, strict: 'ignore' }]]} components={{
    img: ({ alt }) => <span className="muted">[Görsel: {alt}]</span>,
    a: ({ href, children }) => {
      const source = sources.find(s => s.url === href);
      return source && safeHttps(source.url) ? <button className="inline-link" onClick={() => onOpenSource(source.url)}>{children}</button> : <span>{children}{href && safeHttps(href) ? ` (${href})` : ''}</span>;
    },
  }}>{normalizeMath(content)}</ReactMarkdown>;
}

function FileCards({files,onRemove}:{files:Attachment[];onRemove?:(id:string)=>void}) {
  return files.length>0 && <div className={`attachment-list ${onRemove?'draft-files':''}`} aria-label={onRemove?'Eklenecek dosyalar':'Mesaj ekleri'}>{files.map(file=><div className="attachment-card" key={file.id}>
    {file.images[0]?.dataUrl.startsWith('data:image/jpeg;base64,') ? <img src={file.images[0].dataUrl} alt={`${file.name} önizlemesi`} /> : <FileText size={22}/>}
    <div><strong title={file.name}>{file.name}</strong><small>{file.kind.toUpperCase()} · {file.bytes<1024*1024?`${Math.ceil(file.bytes/1024)} KB`:`${(file.bytes/1024/1024).toFixed(1)} MB`}{file.pages>0?` · ${file.pages} sayfa`:''}{file.kind==='zip'?` · ${file.entries} kayıt`:''}</small>{file.warnings.length>0 && <details><summary>Okuma kapsamı</summary>{file.warnings.map((w,i)=><p key={i}>{w}</p>)}</details>}</div>
    {onRemove && <button type="button" aria-label={`${file.name} ekini kaldır`} onClick={()=>onRemove(file.id)}><X size={14}/></button>}
  </div>)}</div>;
}

export default function App() {
  const [browserOpen,setBrowserOpen]=useState(true);
  const [homeOpen,setHomeOpen]=useState(true);
  const [memoryDisabled,setMemoryDisabled]=useState(false); const [memoryNotice,setMemoryNotice]=useState<MemoryNotice|null>(null);const [settingsSection,setSettingsSection]=useState('general');
  const [mode, setMode] = useState<Mode>('chat');
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState(''); const [language, setLanguage] = useState('İngilizce');
  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [busy, setBusy] = useState(false); const [loading, setLoading] = useState(false);
  const [stream, setStream] = useState(''); const [pendingText, setPendingText] = useState('');
  const [pendingSources, setPendingSources] = useState<Source[]>([]); const [progress, setProgress] = useState('');
  const [error, setError] = useState(''); const [settingsOpen, setSettingsOpen] = useState(false);
  const [deleting, setDeleting] = useState<Conversation | null>(null); const [filter, setFilter] = useState('');
  const [attachments,setAttachments]=useState<Attachment[]>([]); const [pendingFiles,setPendingFiles]=useState<Attachment[]>([]); const [preparingFiles,setPreparingFiles]=useState(false);
  const [savingPdf,setSavingPdf]=useState<string|null>(null);const [pdfNotice,setPdfNotice]=useState('');
  const [preferences, setPreferences] = useState<AppSettings>({ ...defaultSettings });
  const [preferencesReady, setPreferencesReady] = useState(false);
  const [savingSettings, setSavingSettings] = useState(false); const [settingsError, setSettingsError] = useState('');
  const activeRequest = useRef<string | null>(null); const selectionVersion = useRef(0);
  const selectedRef = useRef<string | null>(null); const preferencesRef = useRef(preferences);
  const saveQueue = useRef<Promise<void>>(Promise.resolve()); const saveSequence = useRef(0);
  const bottom = useRef<HTMLDivElement>(null); const input = useRef<HTMLTextAreaElement>(null);
  const historySearch = useRef<HTMLInputElement>(null); const lastEngineError = useRef('');

  useEffect(() => {
    let mounted = true;
    void Promise.all([api.list(), api.settings()]).then(([items, prefs]) => {
      if (!mounted) return;
      setConversations(items); preferencesRef.current = prefs; setPreferences(prefs); setMode(prefs.defaultMode); setHomeOpen(prefs.defaultMode==='chat'); setLanguage(prefs.translationLanguage); setPreferencesReady(true);
    }).catch(e => { if (mounted) { setError(readableError(e)); setPreferencesReady(true); } });
    function poll() {
      void api.status().then(status => {
        if (!mounted) return;
        setEngine(status);
        if (status.state === 'error' && lastEngineError.current !== status.detail) { lastEngineError.current = status.detail; setError(status.detail); }
        if (status.state === 'ready') lastEngineError.current = '';
      }).catch(e => { if (mounted) setError(readableError(e)); });
    }
    poll(); const timer = window.setInterval(poll, 1500);
    return () => { mounted = false; window.clearInterval(timer); };
  }, []);
  useEffect(() => { bottom.current?.scrollIntoView({ behavior: preferences.reduceMotion ? 'auto' : 'smooth', block: 'end' }); }, [messages, stream, progress, preferences.reduceMotion]);
  useEffect(() => { setLanguage(preferences.translationLanguage); }, [preferences.translationLanguage]);
  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.isComposing) return;
      if (event.key === ',') { event.preventDefault(); if (preferencesReady) setSettingsOpen(value => !value); }
      if (settingsOpen || deleting) return;
      if (event.key.toLowerCase() === 'n') { event.preventDefault(); fresh(); }
      if (event.key.toLowerCase() === 'k') { event.preventDefault(); (historySearch.current || input.current)?.focus(); }
    }
    document.addEventListener('keydown', keydown); return () => document.removeEventListener('keydown', keydown);
  });
  useEffect(() => {
    if (!deleting) return;
    const previous = document.activeElement as HTMLElement | null;
    const modal = document.querySelector<HTMLElement>('.delete-modal');
    modal?.querySelector<HTMLButtonElement>('button')?.focus();
    function keydown(event: KeyboardEvent) {
      if (event.key === 'Escape') setDeleting(null);
      if (event.key !== 'Tab') return;
      const buttons = modal?.querySelectorAll<HTMLButtonElement>('button'); if (!buttons?.length) return;
      if (event.shiftKey && document.activeElement === buttons[0]) { event.preventDefault(); buttons[buttons.length - 1].focus(); }
      else if (!event.shiftKey && document.activeElement === buttons[buttons.length - 1]) { event.preventDefault(); buttons[0].focus(); }
    }
    document.addEventListener('keydown', keydown); return () => { document.removeEventListener('keydown', keydown); previous?.focus(); };
  }, [deleting]);

  useEffect(()=>{let alive=true;let off:(()=>void)|undefined;void api.onMemoryUpdated(notice=>{if(alive)setMemoryNotice(notice.batchId||notice.error?notice:null);}).then(fn=>{if(alive)off=fn;else fn();}).catch(()=>{});return()=>{alive=false;off?.();};},[]);
  useEffect(()=>{if(!memoryNotice)return;const timer=setTimeout(()=>setMemoryNotice(null),10*60*1000);return()=>clearTimeout(timer);},[memoryNotice]);
  async function toggleConversationMemory(){const next=!memoryDisabled;try{if(selected)await api.conversationMemory(selected,next);setMemoryDisabled(next);setConversations(prev=>prev.map(c=>c.id===selected?{...c,memoryDisabled:next}:c));}catch(e){setError(readableError(e));}}
  async function undoMemory(){if(!memoryNotice?.batchId)return;try{const count=await api.undoMemories(memoryNotice.batchId);if(!count)setError('Bu güncelleme artık geri alınamıyor. Kayıtları Kişisel hafıza ekranından yönetebilirsin.');setMemoryNotice(null);}catch(e){setError(readableError(e));}}
  function choose(id: string | null) { selectedRef.current = id; setSelected(id); }
  function fresh(nextMode: Mode = mode) {
    if (busy || preparingFiles) return;
    if(attachments.length) void api.discardAttachments(attachments.map(f=>f.id)).catch(()=>{});setAttachments([]);
    selectionVersion.current += 1; setLoading(false); choose(null); setMessages([]); setMode(nextMode); setHomeOpen(false); setDraft(''); setError(''); input.current?.focus();
  }
  function goHome(){if(busy||preparingFiles)return;fresh('chat');setHomeOpen(true);}
  function startArea(nextMode:Mode,text=''){fresh(nextMode);setDraft(text);input.current?.focus();}
  function openSettings(section='general'){setSettingsSection(section);setSettingsOpen(true);}
  function updatePreferences(patch: Partial<AppSettings>) {
    const next = { ...preferencesRef.current, ...patch }; preferencesRef.current = next; setPreferences(next);
    setSavingSettings(true); setSettingsError(''); const sequence = ++saveSequence.current;
    saveQueue.current = saveQueue.current.catch(() => {}).then(() => api.saveSettings(next)).then(() => {
      if (sequence === saveSequence.current) setSavingSettings(false);
    }).catch(async e => {
      if (sequence === saveSequence.current) {
        setSettingsError(readableError(e)); setSavingSettings(false);
        try { const actual = await api.settings(); if (sequence === saveSequence.current) { preferencesRef.current = actual; setPreferences(actual); } } catch { /* Keep the visible save error. */ }
      }
    });
  }
  async function refreshData() {
    const items = await api.list(); setConversations(items);
    const id = selectedRef.current;
    if (id && !items.some(item => item.id === id && !item.archived)) { selectionVersion.current += 1; choose(null); setMessages([]); }
  }
  async function select(item: Conversation) {
    if (busy || preparingFiles) return;
    if(attachments.length) void api.discardAttachments(attachments.map(f=>f.id)).catch(()=>{});setAttachments([]);
    const version = ++selectionVersion.current; setLoading(true); setError('');
    try { const items = await api.messages(item.id); if (version !== selectionVersion.current) return; choose(item.id); setMode(item.mode); setHomeOpen(false); setMessages(items); setMemoryDisabled(Boolean(item.memoryDisabled)); setDraft(''); }
    catch (e) { if (version === selectionVersion.current) setError(readableError(e)); }
    finally { if (version === selectionVersion.current) setLoading(false); }
  }
  async function send(text: string) {
    if (busy || preparingFiles || !text.trim() || !desktop || !preferencesReady) return;
    
    if((attachments.length || ((mode==='chat'||mode==='math'||mode==='code') && messages.some(m=>m.attachments?.length))) && Array.from(text).length>2000){setError('Dosya ekli sorular en fazla 2000 karakter olabilir.');return;}
    const files=attachments;setAttachments([]);setPendingFiles(files);
    setHomeOpen(false); setDraft(''); setBusy(true); setError(''); setStream(''); setPendingSources([]); setPendingText(text); setProgress('Hazırlanıyor…');
    const requestId = crypto.randomUUID(); activeRequest.current = requestId; let id = selected;
    try {
      await saveQueue.current;
      if (!id) { const chat = memoryDisabled ? await api.create(mode,true) : await api.create(mode); id = chat.id; choose(id); setConversations(previous => [chat, ...previous]); }
      const response = await api.generate({ requestId, conversationId: id, text, targetLanguage: mode === 'translate' ? language : null, attachmentIds:files.map(f=>f.id) }, event => {
        if (activeRequest.current !== requestId) return;
        if (event.type === 'delta') setStream(previous => previous + event.text);
        if (event.type === 'status') setProgress(event.text);
        if (event.type === 'sources') setPendingSources(event.sources);
      });
      if (response.status === 'complete' && preferencesRef.current.completionSound) playCompletionSound();
    } catch (e) { setError(readableError(e)); }
    finally {
      if (id) {
        try { const [savedMessages, items] = await Promise.all([api.messages(id), api.list()]); setMessages(savedMessages); setConversations(items); }
        catch (e) { setError(readableError(e)); }
      }
      if(files.length) void api.discardAttachments(files.map(f=>f.id)).catch(()=>{});
      activeRequest.current = null; setBusy(false); setPendingText(''); setPendingFiles([]); setStream(''); setPendingSources([]); setProgress(''); input.current?.focus();
    }
  }
  function submit(event?: FormEvent) { event?.preventDefault(); if (!busy && (draft.trim() || attachments.length)) void send(draft.trim() || (mode==='translate'?'Ekli dosyanın içeriğini çevir.':'Ekli dosyaların içeriğini incele ve özetle.')); }
  async function addFiles(){if(preparingFiles || busy)return;setPreparingFiles(true);setError('');try{setAttachments(await api.selectAttachments(attachments.map(f=>f.id)));}catch(e){setError(readableError(e));}finally{setPreparingFiles(false);}}
  function removeFile(id:string){setAttachments(files=>files.filter(f=>f.id!==id));void api.discardAttachments([id]).catch(e=>setError(readableError(e)));}
  async function savePdf(message:Message){setSavingPdf(message.id);setError('');setPdfNotice('');try{const assets=await prepareMathAssets(message.content);const path=await api.exportPdf(message.conversationId,message.id,assets);if(path)setPdfNotice(`PDF kaydedildi: ${path}`);}catch(e){setError(readableError(e));}finally{setSavingPdf(null);}}
  async function remove(item: Conversation) {
    setDeleting(null); setError('');
    try { await api.remove(item.id); await refreshData(); } catch (e) { setError(readableError(e)); }
  }
  async function archiveSelected() {
    if (!selected || busy) return;
    try { await api.archive(selected, !conversations.find(c => c.id === selected)?.archived); await refreshData(); }
    catch (e) { setError(readableError(e)); }
  }
  const modeInfo = modes[mode];
  const activeConversations = conversations.filter(c => !c.archived);
  const visibleConversations = activeConversations.filter(c => c.title.toLocaleLowerCase('tr').includes(filter.toLocaleLowerCase('tr')));
  const hasMessages = messages.length > 0 || busy; const hasFileContext=attachments.length>0 || ((mode==='chat'||mode==='math'||mode==='code') && messages.some(m=>m.attachments?.length)); const maxLength = mode === 'research' ? 2000 : hasFileContext ? 2000 : 6000;
  const engineLabel = engine?.state === 'ready' ? `Hazır${engine.backend ? engine.backend === 'vulkan' ? ' · GPU' : ' · CPU' : ''}` : engine?.state === 'starting' ? 'Hazırlanıyor' : engine?.state === 'error' ? 'Başlatılamadı' : desktop ? 'Yerel model' : 'Tarayıcı görünümü';
  function openSource(url:string){setBrowserOpen(true);void api.openSource(url).catch(e=>setError(readableError(e)));}
  function sourceList(sources: Source[]) {
    return sources.length > 0 && <div className="sources"><span className="eyebrow">KAYNAKLAR · {sources.length}</span>{sources.map(source => <button key={source.id} className="source" disabled={busy} onClick={() => openSource(source.url)} title={source.url}>
      <span className="source-number">{source.id}</span><span><strong>{source.title}</strong><small>{new URL(source.url).hostname} · {new Date(source.accessedAt).toLocaleDateString('tr-TR')}</small></span><ArrowUpRight size={15} />
    </button>)}</div>;
  }
  return <><div className={`app-shell ${homeOpen&&!hasMessages?'home-screen':''} ${mode==='research'&&browserOpen?'research-open':''}`} data-theme={preferences.theme} data-compact={preferences.compactLayout} data-reduce-motion={preferences.reduceMotion} style={{ '--chat-font-size': `${preferences.fontSize}px` } as CSSProperties}>
    <aside className="sidebar">
      <button className="brand" aria-label="Profil ve kişiselleştirme" aria-haspopup="dialog" disabled={busy||preparingFiles} onClick={()=>openSettings('personalization')}><span className="brand-symbol"><AevraMark/><i className={engine?.state==='ready'?'is-ready':''}/></span><span className="brand-identity">{preferences.preferredName.trim()||'Aevra'}<small>{engine?.state==='ready'?'Model hazır':'Kişisel çalışma alanın'}</small></span><ChevronDown size={13}/></button>
      <div className="workspace-label">KİŞİSEL ALANIN</div>
      <button className="new-chat" onClick={() => fresh()} disabled={busy || preparingFiles}><Plus size={17} />Yeni konuşma<span className="shortcut">＋</span></button>
      <nav aria-label="Çalışma modu"><button type="button" className={`nav-item home-nav ${homeOpen&&!hasMessages?'active':''}`} disabled={busy||preparingFiles} onClick={goHome}><House size={18}/>Ana sayfa</button>{(Object.entries(modes) as [Mode, typeof modes.chat][]).map(([key, item]) => { const Icon = item.icon; return <button key={key} className={`nav-item ${mode === key && !homeOpen ? 'active' : ''}`} disabled={busy || preparingFiles} onClick={() => fresh(key)}><Icon size={18} />{item.label}{mode === key && !homeOpen && <span className="active-dot" />}</button>; })}</nav>
      <div className="history-heading"><span>Konuşmaların</span><span>{activeConversations.length || '—'}</span></div>
      {activeConversations.length > 0 && <label className="history-search"><Search size={14} /><input ref={historySearch} aria-label="Geçmişte ara" value={filter} onChange={e => setFilter(e.target.value)} placeholder="Konuşma ara…" /></label>}
      <div className="history-list">{visibleConversations.length ? visibleConversations.map(item => <div className={`history-item ${item.id === selected ? 'selected' : ''}`} key={item.id}><button disabled={busy || preparingFiles} onClick={() => void select(item)} title={item.title}><i className={`history-dot tone-${item.mode}`} aria-hidden="true"/><span>{item.title}</span></button><button className="delete-chat" disabled={busy || preparingFiles} aria-label={`${item.title} konuşmasını sil`} onClick={() => setDeleting(item)}><Trash2 size={13} /></button></div>) : <div className="history-empty"><span>{filter ? 'Eşleşen konuşma yok.' : 'Yeni fikirlerin için yer var.'}</span>{!filter && <small>Konuşmaların burada saklanacak.</small>}</div>}</div>
      <div className="sidebar-bottom"><button className="settings-button" aria-label="Ayarlar ve gizlilik" disabled={!preferencesReady} onClick={()=>openSettings()}><Settings2 size={16}/><span>Ayarlar ve gizlilik</span></button><div className="sidebar-invitation"><div><AevraMark/><strong>Aevra</strong><Sparkles size={14}/></div><p>Fikirlerine biraz alan aç.<br/>Kendi hızında düşün, üret, keşfet.</p><button type="button" disabled={busy||preparingFiles} onClick={()=>startArea('chat','Yeni bir fikir geliştirmeme yardım et. Önce ilgi alanlarımı sor.')}><Plus size={12}/>Yeni bir fikir</button></div><span className="version">AEVRA · 0.1.0</span></div>
    </aside>
    <main className="main-panel">
      <header className="topbar"><div><span className="breadcrumb">Kişisel alanın</span><span className="breadcrumb-divider">/</span><span>{homeOpen&&!hasMessages?'Genel bakış':modeInfo.label}</span></div><div className="topbar-actions">{mode==='research'&&<button type="button" className="browser-toggle" aria-label={browserOpen?'Tarayıcıyı gizle':'Tarayıcıyı göster'} aria-expanded={browserOpen} onClick={()=>setBrowserOpen(value=>!value)}><PanelRightOpen size={15}/>{browserOpen?'Tarayıcı':'Tarayıcıyı göster'}</button>}<button className={`memory-mode ${memoryDisabled?'memory-off':''}`} type="button" disabled={busy||preparingFiles||loading||!desktop} aria-pressed={memoryDisabled} title="Bu sohbet için hafızayı kullanma veya güncelleme. Sohbet geçmişi kaydedilmeye devam eder." onClick={()=>void toggleConversationMemory()}>{memoryDisabled?'Hafızasız sohbet':preferences.memoryEnabled?'Hafıza açık':'Hafıza kapalı'}</button>{selected && messages.length > 0 && <button className="archive-current" title="Sohbeti arşivle veya arşivden çıkar" aria-label={conversations.find(c => c.id === selected)?.archived ? 'Sohbeti arşivden çıkar' : 'Sohbeti arşivle'} disabled={busy || preparingFiles} onClick={() => void archiveSelected()}><Archive size={16} /></button>}<span className={`local-badge ${mode === 'research' ? 'web' : ''}`}>{mode === 'research' ? <Globe2 size={13} /> : <LockKeyhole size={13} />}{mode === 'research' ? 'Web araştırması' : 'Yerel çalışma'}</span></div></header>
      {!desktop && <div className="preview-banner">Tarayıcı görünümü · Sohbet için Aevra masaüstü uygulamasını aç.</div>}
      <div className={`content-area ${hasMessages ? 'conversation-view' : homeOpen ? 'dashboard-view' : ''}`}>
        {loading ? <div className="loading-state">Konuşma açılıyor…</div> : hasMessages ? <section className="messages" aria-label="Konuşma">
          {messages.map(message => <article className={`message ${message.role}`} key={message.id}><div className="message-avatar">{message.role === 'assistant' ? <AevraMark /> : 'S'}</div><div className="message-body"><div className="message-label">{message.role === 'assistant' ? 'Aevra' : 'Sen'}{preferences.showTimestamps && <span>{new Date(message.createdAt).toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' })}</span>}</div><FileCards files={message.attachments || []}/>{message.content && <div className="prose"><Markdown content={message.content} sources={message.sources} onError={setError} onOpenSource={openSource} /></div>}{message.role === 'assistant' && message.status !== 'complete' && <div className={`response-state ${message.status}`}>{message.status === 'interrupted' ? 'Yanıt durduruldu' : message.status === 'limited' ? 'Yanıt devam sınırına ulaştı' : 'Yanıt tamamlanamadı'}{message.content ? ' · Yazılan bölüm kaydedildi' : ''}</div>}{sourceList(message.sources)}{message.role==='assistant' && message.content && <div className="message-actions"><button type="button" disabled={!desktop || busy || savingPdf!==null} onClick={()=>void savePdf(message)}><Download size={14}/>{savingPdf===message.id?'PDF hazırlanıyor…':'PDF olarak kaydet'}</button>{message.status==='limited' && mode!=='translate' && mode!=='research' && <button type="button" disabled={busy} onClick={()=>void send('Önceki yanıtını kesildiği yerden devam ettir ve tamamla; yazılanları tekrarlama.')}>Devam et</button>}</div>}</div></article>)}
          {busy && <><article className="message user"><div className="message-body"><div className="message-label">Sen</div><FileCards files={pendingFiles}/><div className="plain-text">{pendingText}</div></div></article><article className="message assistant"><div className="message-avatar"><AevraMark /></div><div className="message-body"><div className="message-label">Aevra <span className="working-dot" /></div>{stream ? <div className="prose"><Markdown content={stream} sources={pendingSources} onError={setError} onOpenSource={openSource} /></div> : <p className="progress" role="status">{progress}</p>}{sourceList(pendingSources)}</div></article></>}
          <div ref={bottom} />
        </section> : homeOpen&&mode==='chat'?<HomeDashboard preferences={preferences} conversations={conversations} disabled={busy||preparingFiles||!preferencesReady} onStart={startArea} onOpen={item=>void select(item)} onSettings={openSettings} onFocus={()=>input.current?.focus()}/>:<section className="welcome"><div className="welcome-emblem"><AevraMark /><span className="emblem-spark">✦</span></div><div className="welcome-kicker"><span />Kişisel zekâ, tam kontrol.</div><h1>{modeInfo.title}</h1><p className="welcome-subtitle">{modeInfo.subtitle}</p>{preferences.showSuggestions && <div className="suggestions">{suggestions.map((suggestion, i) => <button key={suggestion.mode} onClick={() => { fresh(suggestion.mode); setDraft(suggestion.text); input.current?.focus(); }}><suggestion.icon size={19} /><span>{suggestion.title}</span><ArrowUpRight size={15} /><small>0{i + 1}</small></button>)}</div>}<div className="welcome-privacy"><LockKeyhole size={13} /><span>Sohbet ve çeviri için internet gerekmez.</span></div></section>}
      </div>
      <div className="composer-area">{memoryNotice&&<div className="memory-notice" role="status"><span>{memoryNotice.error||`Hafıza güncellendi · ${memoryNotice.count} bilgi`}</span>{memoryNotice.batchId&&<button type="button" onClick={()=>void undoMemory()}>Geri al</button>}<button type="button" onClick={()=>{setSettingsSection('memory');setSettingsOpen(true);}}>Görüntüle</button><button type="button" aria-label="Hafıza bildirimini kapat" onClick={()=>setMemoryNotice(null)}><X size={14}/></button></div>}{pdfNotice && <div className="pdf-notice" role="status">{pdfNotice}<button type="button" aria-label="PDF bildirimini kapat" onClick={()=>setPdfNotice('')}><X size={14}/></button></div>}{error && <div className="error-banner" role="alert"><span>{error}</span><button aria-label="Bildirimi kapat" onClick={() => setError('')}><X size={15} /></button></div>}<form className={`composer ${busy ? 'is-busy' : ''}`} onSubmit={submit}>
        {mode === 'translate' && <div className="translation-toolbar"><span>Otomatik algıla</span><ArrowUpRight size={14} /><label>Hedef dil <select value={language} disabled={busy || preparingFiles} onChange={e => setLanguage(e.target.value)}>{languages.map(l => <option key={l}>{l}</option>)}</select></label></div>}
        <FileCards files={attachments} onRemove={removeFile}/>{attachments.length>0 && <div className="attachment-hint">En fazla 4 dosya · 20 MB/dosya · PDF çevirisinde sayfalar sırayla işlenir.</div>}{preparingFiles && <div className="attachment-hint" role="status">Dosyalar bu bilgisayarda hazırlanıyor…</div>}<textarea ref={input} value={draft} disabled={loading || !preferencesReady} maxLength={maxLength} onChange={e => setDraft(e.target.value)} placeholder={modeInfo.placeholder} aria-label={modeInfo.placeholder} rows={2} onKeyDown={e => { if (e.key !== 'Enter' || e.nativeEvent.isComposing) return; const shouldSend = preferences.sendWithEnter ? !e.shiftKey && !e.ctrlKey && !e.metaKey : e.ctrlKey || e.metaKey; if (shouldSend) { e.preventDefault(); submit(); } }} />
        <div className="composer-toolbar"><button type="button" className="attach-button" aria-label="Dosya ekle" title={mode==='research'?'Dosyalar yerel çalışma alanlarına eklenebilir':'Görsel, PDF, Word (.docx/.doc), metin, kod veya ZIP ekle'} disabled={!desktop || busy || preparingFiles || attachments.length>=4 || mode==='research'} onClick={()=>void addFiles()}><Paperclip size={18}/></button><span className="composer-mode"><modeInfo.icon size={14} />{modeInfo.label}</span><span className="character-count">{draft.length ? `${Array.from(draft).length} / ${maxLength}` : preferences.sendWithEnter ? 'Enter ile gönder · Shift + Enter ile yeni satır' : 'Ctrl + Enter ile gönder · Enter ile yeni satır'}</span>{busy ? <button type="button" className="send-button stop" aria-label="Yanıtı durdur" onClick={() => { if (activeRequest.current) void api.cancel(activeRequest.current).catch(e => setError(readableError(e))); }}><Square size={15} fill="currentColor" /></button> : <button className="send-button" type="submit" disabled={(!draft.trim() && !attachments.length) || preparingFiles || !desktop || loading || !preferencesReady} aria-label={mode === 'research' ? 'Araştır' : 'Gönder'}><ArrowUp size={20} /></button>}</div>
      </form><div className="footer-status"><button className="engine-status" disabled={!preferencesReady} onClick={() => openSettings('model')}><span className={`status-dot ${engine?.state === 'ready' ? 'ready' : ''} ${engine?.state === 'starting' ? 'starting' : ''}`} />{engine?.model || 'Qwen3.5-9B'}<span className="status-divider">·</span>{engineLabel}</button><span>Aevra hata yapabilir. Önemli bilgileri doğrula.</span></div></div>
    </main>
    <ResearchBrowser active={mode==='research'} visible={mode==='research'&&browserOpen} suspended={settingsOpen||deleting!==null} enabled={preferences.webResearchEnabled} onClose={()=>setBrowserOpen(false)} onError={setError}/>
  </div>
  {settingsOpen && <SettingsPanel initialSection={settingsSection} preferences={preferences} onChange={updatePreferences} conversations={conversations} engine={engine} busy={busy || preparingFiles} saving={savingSettings} saveError={settingsError} onRefresh={refreshData} onOpen={select} onClose={() => setSettingsOpen(false)} />}
  {deleting && <div className="modal-backdrop"><section className="modal compact delete-modal" role="dialog" aria-modal="true" aria-labelledby="delete-title"><h2 id="delete-title">Konuşma silinsin mi?</h2><p>“{deleting.title}” ve içindeki mesajlar geçmişten kaldırılacak.</p><div className="modal-actions"><button className="secondary-button" onClick={() => setDeleting(null)}>Vazgeç</button><button className="danger-button" onClick={() => void remove(deleting)}>Konuşmayı sil</button></div></section></div>}
  </>;
}
