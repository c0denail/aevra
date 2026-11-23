import { isValidElement, useEffect, useRef, useState, type ReactNode } from 'react';
import { Archive, Bell, Check, ChevronRight, Cpu, Database, Download, FolderOpen, Globe2, Info, Keyboard, Monitor, Palette, RotateCcw, Search, Settings2, Shield, Trash2, Upload, UserRound, X } from 'lucide-react';
import { api, desktop } from './api';
import { defaultSettings, languages } from './preferences';
import { readableError } from './safety';
import type { AppSettings, Conversation, EngineStatus, StorageInfo } from './types';
import './settings.css';
import MemoryPanel from './MemoryPanel';
import {Brain} from 'lucide-react';

interface Props {
  initialSection?:string;
  preferences: AppSettings; onChange: (patch: Partial<AppSettings>) => void;
  conversations: Conversation[]; engine: EngineStatus | null; busy: boolean;
  saving: boolean; saveError: string; onRefresh: () => Promise<void>; onOpen: (chat: Conversation) => Promise<void>; onClose: () => void;
}

function Toggle({ title, checked, onChange }: { title: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <button className="settings-toggle" type="button" role="switch" aria-label={title} aria-checked={checked} onClick={() => onChange(!checked)}><span /></button>;
}
function Row({ title, description, children, stacked = false }: { title: string; description?: string; children: ReactNode; stacked?: boolean }) {
  return <div className={`preferences-row ${stacked ? 'stacked' : ''}`}><div className="preferences-label"><span>{title}</span>{description && <p>{description}</p>}</div><div className="preferences-control">{children}</div></div>;
}
function size(bytes: number) { return bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : bytes < 1024 ** 3 ? `${(bytes / 1024 ** 2).toFixed(1)} MB` : `${(bytes / 1024 ** 3).toFixed(2)} GB`; }
function searchableText(node: ReactNode): string {
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(searchableText).join(' ');
  if (isValidElement<{ title?: string; description?: string; children?: ReactNode; 'aria-label'?: string }>(node)) {
    return [node.props.title, node.props.description, node.props['aria-label'], searchableText(node.props.children)].filter(Boolean).join(' ');
  }
  return '';
}

export default function SettingsPanel(props: Props) {
  const { preferences: p, onChange, conversations, engine, busy, onClose } = props;
  const [active, setActive] = useState(props.initialSection||'general'); const [query, setQuery] = useState('');
  const [storage, setStorage] = useState<StorageInfo | null>(null);
  const [notice, setNotice] = useState(''); const [error, setError] = useState(''); const [working, setWorking] = useState(false);
  const [confirmation, setConfirmation] = useState<{ title: string; text: string; action: () => Promise<void>; danger?: boolean } | null>(null);
  const dialog = useRef<HTMLElement>(null); const search = useRef<HTMLInputElement>(null);

  useEffect(() => { void api.storage().then(setStorage).catch(e => setError(readableError(e))); }, []);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null; search.current?.focus();
    return () => previous?.focus();
  }, []);
  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (event.key === 'Escape') { event.stopPropagation(); if (confirmation) setConfirmation(null); else onClose(); }
      if (event.key !== 'Tab') return;
      const container = confirmation ? dialog.current?.querySelector('.settings-confirmation') : dialog.current;
      const items = Array.from(container?.querySelectorAll<HTMLElement>('button:not(:disabled), input, select, textarea, [tabindex="0"]') || []);
      const first = items[0]; const last = items.at(-1);
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
    document.addEventListener('keydown', keydown);
    if (confirmation) dialog.current?.querySelector<HTMLButtonElement>('.settings-confirmation button')?.focus();
    return () => document.removeEventListener('keydown', keydown);
  }, [confirmation, onClose]);

  async function action(operation: () => Promise<string | void>) {
    setWorking(true); setError(''); setNotice('');
    try { const result = await operation(); if (result) setNotice(result); await props.onRefresh(); setStorage(await api.storage()); }
    catch (e) { setError(readableError(e)); }
    finally { setWorking(false); }
  }
  const disabled = working || busy || !desktop;
  const toggle = (key: keyof AppSettings, title: string) => <Toggle title={title} checked={Boolean(p[key])} onChange={value => onChange({ [key]: value })} />;
  const archives = conversations.filter(chat => chat.archived);
  const sections = [
    { id: 'general', label: 'Genel', icon: Settings2, keywords: 'varsayılan mod enter gönder öneriler dil', content: <>
      <Row title="Varsayılan çalışma alanı" description="Aevra açıldığında gösterilecek ekran."><select aria-label="Varsayılan çalışma alanı" value={p.defaultMode} onChange={e => onChange({ defaultMode: e.target.value as AppSettings['defaultMode'] })}><option value="chat">Sohbet</option><option value="translate">Çeviri</option><option value="research">Araştırma</option><option value="math">Matematik</option><option value="code">Kodlama</option></select></Row>
      <Row title="Enter ile gönder" description="Kapalıyken Ctrl + Enter ile gönderilir; Enter yeni satır ekler.">{toggle('sendWithEnter', 'Enter ile gönder')}</Row>
      <Row title="Başlangıç önerileri" description="Ana ekranda örnek görev kartlarını göster.">{toggle('showSuggestions', 'Başlangıç önerileri')}</Row>
      <Row title="Uygulama dili"><span className="settings-value">Türkçe</span></Row>
    </> },
    { id: 'notifications', label: 'Bildirimler', icon: Bell, keywords: 'ses yanıt tamamlanma', content: <>
      <Row title="Yanıt tamamlanma sesi" description="Yanıt bittiğinde uygulama içinde kısa bir ses çal.">{toggle('completionSound', 'Yanıt tamamlanma sesi')}</Row>
      <div className="settings-hint">Bildirim sesi bu uygulamanın içindedir; Windows bildirim izni istemez.</div>
    </> },
    { id: 'import', label: 'İçe aktar', icon: Upload, keywords: 'json yedek sohbet aktarım', content: <>
      <Row title="Sohbet yedeğini içe aktar" description="Aevra JSON yedeğindeki sohbetleri ekle. Aynı kimliğe sahip mevcut sohbetler değiştirilmez."><button className="preferences-button" disabled={disabled} onClick={() => void action(async () => { const count = await api.importHistory(); if (count !== null) return `${count} konuşma içe aktarıldı.`; })}><Upload size={15} />Dosya seç</button></Row>
      <div className="settings-hint">Yalnızca seçtiğin yerel dosya okunur. En fazla 50 MB.</div>
    </> },
    { id: 'appearance', label: 'Görünüm', icon: Palette, keywords: 'tema karanlık koyu aydınlık açık yazı boyutu sıkışık hareket zaman', content: <>
      <Row title="Ana ekran teması" description="Tema ana ekran ve sohbet alanına uygulanır."><div className="theme-picker"><button className={p.theme === 'light' ? 'chosen' : ''} aria-pressed={p.theme === 'light'} onClick={() => onChange({ theme: 'light' })}><Monitor size={15} />Açık</button><button className={p.theme === 'dark' ? 'chosen' : ''} aria-pressed={p.theme === 'dark'} onClick={() => onChange({ theme: 'dark' })}><Monitor size={15} />Koyu</button></div></Row>
      <Row title="Sohbet yazı boyutu"><select aria-label="Sohbet yazı boyutu" value={p.fontSize} onChange={e => onChange({ fontSize: Number(e.target.value) })}><option value={13}>Küçük</option><option value={14}>Normal</option><option value={16}>Büyük</option><option value={17}>Çok büyük</option></select></Row>
      <Row title="Kompakt sohbet düzeni" description="Mesajların arasındaki boşluğu azalt.">{toggle('compactLayout', 'Kompakt sohbet düzeni')}</Row>
      <Row title="Mesaj saatleri">{toggle('showTimestamps', 'Mesaj saatleri')}</Row>
      <Row title="Hareketleri azalt" description="Animasyonları ve yumuşak kaydırmayı kapat.">{toggle('reduceMotion', 'Hareketleri azalt')}</Row>
    </> },
    { id: 'personalization', label: 'Kişiselleştirme', icon: UserRound, keywords: 'profil isim ad talimat yanıt tarz uzun kısa', content: <>
      <Row title="Sana nasıl hitap edelim?" stacked><input aria-label="Tercih edilen ad" value={p.preferredName} maxLength={80} placeholder="Adın veya takma adın" onChange={e => onChange({ preferredName: e.target.value })} /></Row>
      <Row title="Yanıt tarzı"><select aria-label="Yanıt tarzı" value={p.responseStyle} onChange={e => onChange({ responseStyle: e.target.value as AppSettings['responseStyle'] })}><option value="concise">Kısa ve doğrudan</option><option value="balanced">Dengeli</option><option value="detailed">Ayrıntılı</option></select></Row>
      <Row title="Özel talimatlar" description="Sohbet yanıtlarında dikkate alınmasını istediğin tercihler." stacked><textarea aria-label="Özel talimatlar" rows={5} maxLength={2000} value={p.customInstructions} placeholder="Örneğin: Teknik konuları örneklerle açıkla." onChange={e => onChange({ customInstructions: e.target.value })} /><small>{p.customInstructions.length} / 2000</small></Row>
    </> },
    { id: 'model', label: 'Model ve çeviri', icon: Cpu, keywords: 'yerel qwen cpu gpu otomatik donanım ekran kartı işlemci sıcaklık yaratıcılık uzunluk token hedef dil', content: <>
      <Row title={engine?.model || 'Qwen3.5-9B · Q4_K_M'} description={engine?.state === 'starting' ? 'Model hazırlanıyor…' : engine?.detail || 'Durum okunuyor…'}><span className={`model-state ${engine?.state === 'ready' ? 'ready' : ''}`}><span />{engine?.state === 'ready' ? 'Hazır' : engine?.state === 'starting' ? 'Hazırlanıyor' : 'Yerel'}</span></Row>
      <Row title="Çalıştırma donanımı" description="Seçim kaydedildiğinde model yeniden hazırlanır. Otomatik, uygun ekran kartını kullanır; gerekirse CPU'ya döner."><select aria-label="Çalıştırma donanımı" disabled={busy || props.saving} value={p.computeDevice} onChange={e => onChange({ computeDevice: e.target.value as AppSettings['computeDevice'] })}><option value="auto">Otomatik</option><option value="cpu">CPU · İşlemci</option><option value="gpu">GPU · Ekran kartı</option></select></Row>
      {engine?.fallbackReason && <div className="settings-hint">{engine.fallbackReason}</div>}
      <Row title="Yanıt yaratıcılığı" description="Düşük değerler daha tutarlı, yüksek değerler daha çeşitli yanıtlar üretir."><div className="range-control"><input aria-label="Yanıt yaratıcılığı" type="range" min={0} max={1.5} step={0.1} value={p.temperature} onChange={e => onChange({ temperature: Number(e.target.value) })} /><output>{p.temperature.toFixed(1)}</output></div></Row>
      <Row title="Yanıt uzunluğu sınırı" description="Uzun yanıtlarda üretim gerektiğinde otomatik devam eder; yanıt tamamlanmadan bitti sayılmaz."><select aria-label="Yanıt uzunluğu sınırı" value={p.maxTokens} onChange={e => onChange({ maxTokens: Number(e.target.value) })}><option value={256}>Kısa · 256 token</option><option value={512}>Orta · 512 token</option><option value={1024}>Normal · 1024 token</option><option value={2048}>Uzun · 2048 token</option><option value={4096}>Çok uzun · 4096 token</option><option value={8192}>En uzun · 8192 token</option></select></Row>
      <Row title="Varsayılan çeviri dili"><select aria-label="Varsayılan çeviri dili" value={p.translationLanguage} onChange={e => onChange({ translationLanguage: e.target.value })}>{languages.map(language => <option key={language}>{language}</option>)}</select></Row>
    </> },
    { id: 'research', label: 'Araştırma', icon: Globe2, keywords: 'web internet kaynak google arama', content: <>
      <Row title="Web araştırması" description="Araştırma sohbetinde yeni bilgi gerektiğinde web'de arar. Kapalıyken önceki kaynaklarla yerelde konuşabilirsin.">{toggle('webResearchEnabled', 'Web araştırması')}</Row>
      <Row title="En fazla kaynak sayısı"><select aria-label="En fazla kaynak sayısı" value={p.researchSourceLimit} onChange={e => onChange({ researchSourceLimit: Number(e.target.value) })}>{[1,2,3,4,5].map(count => <option key={count} value={count}>{count} kaynak</option>)}</select></Row>
      <Row title="Arama sağlayıcısı"><span className="settings-value">Google</span></Row>
      <div className="settings-hint">Araştırma, uygulamanın içindeki tarayıcıda yürür. Sorgularda güncel sorunun, açık sayfanın ve kaynak başlıklarının sözcükleri kullanılabilir; sohbet geçmişinin metni arama hizmetine gönderilmez. Siteler bağlantı bilgilerini görebilir.</div>
    </> },
    {id:'memory',label:'Kişisel hafıza',icon:Brain,keywords:'otomatik hatırla unut kişisel hafıza bilgi tercih',content:<>
      <Row title="Otomatik kişisel hafıza" description="Yeni mesajlarından önemli bilgileri yerelde hatırlar. Kapalıyken hafıza kullanılmaz ve yeni kayıt eklenmez.">{toggle('memoryEnabled','Otomatik kişisel hafıza')}</Row><MemoryPanel/>
    </>},
    { id: 'archives', label: 'Arşivlenmiş sohbetler', icon: Archive, keywords: `geçmiş geri yükle ${archives.map(c => c.title).join(' ')}`, content: <>
      {archives.length ? <div className="archived-list">{archives.map(chat => <div className="archived-chat" key={chat.id}><button disabled={busy} className="archived-title" onClick={() => { void props.onOpen(chat); onClose(); }}>{chat.title}<small>{new Date(chat.updatedAt).toLocaleDateString('tr-TR')}</small></button><button disabled={disabled} title="Arşivden çıkar" aria-label={`${chat.title} arşivden çıkar`} onClick={() => void action(async () => { await api.archive(chat.id, false); return 'Konuşma arşivden çıkarıldı.'; })}><RotateCcw size={16} /></button><button disabled={disabled} title="Konuşmayı sil" aria-label={`${chat.title} sil`} onClick={() => setConfirmation({ title: 'Konuşma silinsin mi?', text: `“${chat.title}” kalıcı olarak silinecek.`, danger: true, action: async () => { await api.remove(chat.id); } })}><Trash2 size={16} /></button></div>)}</div> : <div className="settings-empty"><Archive size={28} /><strong>Henüz arşivlenmiş sohbet yok.</strong><p>Bir sohbeti açıp üstteki arşiv düğmesini kullanabilirsin.</p></div>}
    </> },
    { id: 'storage', label: 'Depolama', icon: Database, keywords: 'disk klasör sqlite dosya bellek boyut', content: <>
      <div className="storage-cards"><div><span>Sohbet verileri</span><strong>{desktop && storage ? size(storage.databaseBytes) : '—'}</strong></div><div><span>Yerel model</span><strong>{desktop && storage ? size(storage.modelBytes) : '—'}</strong></div></div>
      <Row title="Kayıtlı konuşmalar"><span className="settings-value">{desktop ? storage?.conversationCount ?? '…' : '—'}</span></Row><Row title="Kayıtlı mesajlar"><span className="settings-value">{desktop ? storage?.messageCount ?? '…' : '—'}</span></Row>
      <Row title="Veri klasörü" description={desktop ? storage?.dataDir : 'Masaüstü uygulamasında kullanılabilir.'}><button className="preferences-button" disabled={!desktop} onClick={() => void action(() => api.openDataFolder())}><FolderOpen size={15} />Klasörü aç</button></Row>
    </> },
    { id: 'data', label: 'Veri kontrolleri', icon: Shield, keywords: 'dışa aktar json yedek sil bütün sohbet arşivle sıfırla', content: <>
      <Row title="Sohbetleri dışa aktar" description="Geçmişi ve kaynakları yerel bir JSON yedeğine kaydet."><button className="preferences-button" disabled={working || !desktop} onClick={() => void action(async () => { const path = await api.exportHistory(); if (path) return 'Sohbet yedeği kaydedildi.'; })}><Download size={15} />Dışa aktar</button></Row>
      <Row title="Tüm sohbetleri arşivle" description="Konuşmaları silmeden ana ekrandan kaldır."><button className="preferences-button" disabled={disabled || !conversations.length} onClick={() => void action(async () => { await api.archiveAll(); return 'Sohbetler arşivlendi.'; })}>Arşivle</button></Row>
      <Row title="Tüm sohbetleri sil" description="Arşivlenmiş sohbetler dahil bütün geçmişi kaldır."><button className="preferences-button danger" disabled={disabled || !conversations.length} onClick={() => setConfirmation({ title: 'Tüm sohbetler silinsin mi?', text: 'Bütün mesajlar ve araştırma kayıtları kalıcı olarak silinecek. Bu işlem geri alınamaz.', danger: true, action: () => api.clearHistory() })}>Tümünü sil</button></Row>
      <Row title="Ayarları sıfırla" description="Sohbetleri koruyarak tercihleri başlangıç değerlerine döndür."><button className="preferences-button" onClick={() => setConfirmation({ title: 'Ayarlar sıfırlansın mı?', text: 'Görünüm, yanıt ve araştırma tercihleri varsayılan değerlere dönecek. Sohbetlerin korunacak.', action: async () => { onChange({ ...defaultSettings }); } })}>Sıfırla</button></Row>
    </> },
    { id: 'shortcuts', label: 'Klavye kısayolları', icon: Keyboard, keywords: 'ctrl enter escape yeni konuşma ara ayarlar', content: <>
      {[['Yeni konuşma', 'Ctrl + N'], ['Ayarları aç / kapat', 'Ctrl + ,'], ['Konuşmalarda ara', 'Ctrl + K'], ['Mesaj gönder', p.sendWithEnter ? 'Enter' : 'Ctrl + Enter'], ['Yeni satır', p.sendWithEnter ? 'Shift + Enter' : 'Enter'], ['Pencereyi kapat', 'Escape']].map(([title, keys]) => <Row title={title} key={title}><kbd>{keys}</kbd></Row>)}
    </> },
    { id: 'about', label: 'Hakkında', icon: Info, keywords: 'aevra sürüm 0.1.0 uygulama', content: <>
      <div className="settings-about"><span>Aevra</span><p>Kişisel zekâ, tam kontrol.</p></div><Row title="Uygulama sürümü"><span className="settings-value">0.1.0</span></Row><Row title="Çalışma motoru"><span className="settings-value">llama.cpp · {engine?.backend === 'vulkan' ? 'GPU (Vulkan)' : engine?.backend === 'cpu' ? 'CPU' : 'Otomatik / CPU / GPU'}</span></Row><Row title="Model"><span className="settings-value">Qwen3.5-9B · Q4_K_M</span></Row>
    </> },
  ];
  const normalizedQuery = query.trim().toLocaleLowerCase('tr');
  const displayed = normalizedQuery ? sections.filter(s => `${s.label} ${s.keywords} ${searchableText(s.content)}`.toLocaleLowerCase('tr').includes(normalizedQuery)) : sections.filter(s => s.id === active);

  return <div className="settings-overlay" onMouseDown={e => { if (e.target === e.currentTarget) onClose(); }}><section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" ref={dialog}>
    <aside className="settings-sidebar"><h2 id="settings-title">Ayarlar</h2><label className="settings-search"><Search size={14} /><input ref={search} value={query} onChange={e => setQuery(e.target.value)} placeholder="Arama" aria-label="Ayarlarda ara" /></label><nav aria-label="Ayar kategorileri">{sections.map(section => <button key={section.id} className={active === section.id && !query ? 'selected' : ''} onClick={() => { setActive(section.id); setQuery(''); setNotice(''); setError(''); }}><section.icon size={16} /><span>{section.label}</span>{section.id === 'archives' && archives.length > 0 && <small>{archives.length}</small>}</button>)}</nav><div className="settings-save-status" role="status">{props.saveError ? 'Kaydedilemedi' : props.saving ? 'Kaydediliyor…' : <><Check size={12} />Değişiklikler kaydedildi</>}</div></aside>
    <div className="settings-main"><header><div><span className="settings-breadcrumb">Aevra <ChevronRight size={11} /> Ayarlar</span><h2>{query ? 'Arama sonuçları' : sections.find(s => s.id === active)?.label}</h2></div><button className="settings-close" aria-label="Ayarları kapat" onClick={onClose}><X size={20} /></button></header><div className="settings-scroll">
      {(error || props.saveError) && <div className="settings-feedback error" role="alert">{error || props.saveError}</div>}{notice && <div className="settings-feedback" role="status"><Check size={15} />{notice}</div>}
      {!desktop && <div className="settings-hint">Arayüz görünümü bu pencerede denenebilir. Sohbet yönetimi ve model seçenekleri masaüstü uygulamasında çalışır.</div>}
      {displayed.length ? displayed.map(section => <section className="settings-section" key={section.id}>{query && <h3>{section.label}</h3>}{section.content}</section>) : <div className="settings-empty"><Search size={26} /><strong>Bu aramayla eşleşen ayar yok.</strong></div>}
    </div></div>
    {confirmation && <div className="settings-confirmation-backdrop"><div className="settings-confirmation" role="alertdialog" aria-modal="true" aria-label={confirmation.title}><h3>{confirmation.title}</h3><p>{confirmation.text}</p><div><button className="preferences-button" onClick={() => setConfirmation(null)}>Vazgeç</button><button className={`preferences-button ${confirmation.danger ? 'danger' : 'primary'}`} onClick={() => { const operation = confirmation.action; setConfirmation(null); void action(async () => { await operation(); return 'İşlem tamamlandı.'; }); }}>{confirmation.danger ? 'Sil' : 'Onayla'}</button></div></div></div>}
  </section></div>;
}
