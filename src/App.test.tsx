// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Conversation, GenerateRequest, GenerationEvent, Message } from './types';
import { defaultSettings } from './preferences';

const backend = vi.hoisted(() => ({
  browserStatus:vi.fn(),browserLayout:vi.fn(),browserControl:vi.fn(),browserTakeover:vi.fn(),browserClose:vi.fn(),onBrowserUpdated:vi.fn(),
  list: vi.fn(), settings: vi.fn(), status: vi.fn(), create: vi.fn(), generate: vi.fn(), messages: vi.fn(),
  saveSettings: vi.fn(), cancel: vi.fn(), storage: vi.fn(), archive: vi.fn(), archiveAll: vi.fn(), remove: vi.fn(), clearHistory: vi.fn(),
  selectAttachments:vi.fn(),discardAttachments:vi.fn(),
  exportPdf:vi.fn(),
  onMemoryUpdated:vi.fn(),memories:vi.fn(),undoMemories:vi.fn(),conversationMemory:vi.fn(),
}));
vi.mock('./api', () => ({ desktop: true, api: backend }));
import App from './App';

const chat: Conversation = { id: 'chat-1', title: 'Deneme', mode: 'chat', updatedAt: 1, archived: false };
beforeEach(() => {
  vi.clearAllMocks();
  backend.browserClose.mockResolvedValue(undefined);
  backend.browserStatus.mockResolvedValue({url:'',title:'',loading:false,automating:false,detail:'',steps:[]});backend.browserLayout.mockResolvedValue(undefined);backend.browserControl.mockResolvedValue(undefined);backend.browserTakeover.mockResolvedValue(undefined);backend.onBrowserUpdated.mockResolvedValue(()=>{});
  backend.onMemoryUpdated.mockResolvedValue(()=>{});backend.memories.mockResolvedValue({items:[],blockedKeys:[]});backend.undoMemories.mockResolvedValue(1);backend.conversationMemory.mockResolvedValue(undefined);
  Element.prototype.scrollIntoView = vi.fn();
  backend.list.mockResolvedValue([]); backend.settings.mockResolvedValue({ ...defaultSettings });
  backend.status.mockResolvedValue({ state: 'ready', detail: 'Hazır', model: 'Qwen3.5-9B' });
  backend.create.mockResolvedValue(chat); backend.messages.mockResolvedValue([]);
  backend.saveSettings.mockResolvedValue(undefined); backend.cancel.mockResolvedValue(undefined);
  backend.storage.mockResolvedValue({ dataDir: 'local', databaseBytes: 1024, modelBytes: 3000000000, conversationCount: 0, messageCount: 0 });
  backend.selectAttachments.mockResolvedValue([]);backend.discardAttachments.mockResolvedValue(undefined);
  backend.exportPdf.mockResolvedValue('C:\\Aevra\\yanit.pdf');
});
afterEach(cleanup);

describe('Aevra conversation and settings flows', () => {
  it('opens a recent conversation from home and prepares a new working area without sending it',async()=>{
    backend.list.mockResolvedValue([chat]);backend.messages.mockResolvedValue([{id:'saved',conversationId:chat.id,role:'assistant',content:'Kayıtlı bir yanıt',status:'complete',sources:[],createdAt:1}]);
    render(<App/>);fireEvent.click(await screen.findByRole('button',{name:'Son konuşmayı aç: Deneme'}));expect(await screen.findByText('Kayıtlı bir yanıt')).toBeTruthy();expect(screen.queryByRole('region',{name:'Kişisel çalışma alanı'})).toBeNull();
    fireEvent.click(screen.getByRole('button',{name:'Ana sayfa'}));fireEvent.click(await screen.findByRole('button',{name:'Hızlı başlangıç: Bir şey geliştir'}));
    expect((screen.getByRole('textbox',{name:'Ne geliştirmek istersin? Dili ve ihtiyacını yaz…'}) as HTMLTextAreaElement).value).toContain('Yeni bir proje');expect(backend.generate).not.toHaveBeenCalled();
  });
  it('starts a memory-free conversation and persists its choice',async()=>{
    backend.generate.mockResolvedValue({status:'complete'});render(<App/>);await waitFor(()=>expect((screen.getByRole('button',{name:'Hafıza açık'}) as HTMLButtonElement).disabled).toBe(false));fireEvent.click(screen.getByRole('button',{name:'Hafıza açık'}));expect(screen.getByRole('button',{name:'Hafızasız sohbet'}).getAttribute('aria-pressed')).toBe('true');fireEvent.change(screen.getByRole('textbox',{name:'Aklından ne geçiyor?'}),{target:{value:'Merhaba'}});fireEvent.click(screen.getByRole('button',{name:'Gönder'}));await waitFor(()=>expect(backend.create).toHaveBeenCalledWith('chat',true));
  });
  it('shows an automatic memory notice, supports undo and opens memory settings',async()=>{
    let notify!:(n:{batchId:string;count:number})=>void;backend.onMemoryUpdated.mockImplementation(async fn=>{notify=fn;return()=>{};});render(<App/>);await waitFor(()=>expect(backend.onMemoryUpdated).toHaveBeenCalled());act(()=>notify({batchId:'update-1',count:1}));expect(screen.getByText('Hafıza güncellendi · 1 bilgi')).toBeTruthy();fireEvent.click(screen.getByRole('button',{name:'Geri al'}));await waitFor(()=>expect(backend.undoMemories).toHaveBeenCalledWith('update-1'));act(()=>notify({batchId:'update-2',count:1}));fireEvent.click(screen.getByRole('button',{name:'Görüntüle'}));expect(await screen.findByRole('switch',{name:'Otomatik kişisel hafıza'})).toBeTruthy();
  });
  it('sends the selected German target language to the translation command',async()=>{
    backend.generate.mockResolvedValue({status:'complete'});render(<App/>);await waitFor(()=>expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole('button',{name:'Çeviri'}));fireEvent.change(screen.getByRole('combobox',{name:'Hedef dil'}),{target:{value:'Almanca'}});fireEvent.change(screen.getByRole('textbox',{name:'Çevirmek istediğin metni yaz veya yapıştır…'}),{target:{value:'Good morning'}});fireEvent.click(screen.getByRole('button',{name:'Gönder'}));await waitFor(()=>expect(backend.generate).toHaveBeenCalledOnce());expect(backend.generate.mock.calls[0][0].targetLanguage).toBe('Almanca');
  });
  it('exports the stored assistant response through the narrow PDF command',async()=>{
    const message:Message={id:'answer-pdf',conversationId:chat.id,role:'assistant',content:'Türkçe ve Almanca: Straße',status:'complete',sources:[],createdAt:1};backend.list.mockResolvedValue([chat]);backend.messages.mockResolvedValue([message]);render(<App/>);
    fireEvent.click(await screen.findByRole('button',{name:'Deneme'}));fireEvent.click(await screen.findByRole('button',{name:'PDF olarak kaydet'}));await waitFor(()=>expect(backend.exportPdf).toHaveBeenCalledWith(chat.id,'answer-pdf',[]));expect((await screen.findByText(/PDF kaydedildi:/)).textContent).toContain('yanit.pdf');
  });
  it('labels an unfinished limited answer and offers continuation',async()=>{
    backend.list.mockResolvedValue([chat]);backend.messages.mockResolvedValue([{id:'limited',conversationId:chat.id,role:'assistant',content:'Yarım yanıt',status:'limited',sources:[],createdAt:1}]);render(<App/>);fireEvent.click(await screen.findByRole('button',{name:'Deneme'}));expect(await screen.findByRole('button',{name:'Devam et'})).toBeTruthy();expect(screen.getByText(/Yanıt devam sınırına ulaştı/)).toBeTruthy();
  });
  it('adds and removes local files, sends only attachment IDs, and reloads persisted cards',async()=>{
    const file={id:'file-1',name:'belge.pdf',kind:'pdf',bytes:1024,pages:2,entries:1,warnings:['Yalnızca seçilen bölümler'],images:[]};
    backend.selectAttachments.mockResolvedValue([file]);
    const response:Message={id:'answer',conversationId:chat.id,role:'assistant',content:'Belge özeti',status:'complete',sources:[],createdAt:1};
    backend.generate.mockResolvedValue(response);backend.messages.mockResolvedValue([{...response,id:'user',role:'user',content:'Özetle',attachments:[file]},response]);backend.list.mockResolvedValue([chat]);
    render(<App/>);await waitFor(()=>expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole('button',{name:'Dosya ekle'}));await screen.findByText('belge.pdf');
    fireEvent.click(screen.getByRole('button',{name:'belge.pdf ekini kaldır'}));expect(backend.discardAttachments).toHaveBeenCalledWith(['file-1']);
    fireEvent.click(screen.getByRole('button',{name:'Dosya ekle'}));await screen.findByText('belge.pdf');
    fireEvent.change(screen.getByRole('textbox',{name:'Aklından ne geçiyor?'}),{target:{value:'Özetle'}});fireEvent.click(screen.getByRole('button',{name:'Gönder'}));
    await waitFor(()=>expect(backend.generate).toHaveBeenCalledOnce());expect(backend.generate.mock.calls[0][0]).toEqual(expect.objectContaining({attachmentIds:['file-1']}));
    expect('attachments' in backend.generate.mock.calls[0][0]).toBe(false);await screen.findByText('Belge özeti');expect(screen.getByText('belge.pdf')).toBeTruthy();
  });
  it('keeps attachments local and blocks the picker in research mode',async()=>{
    render(<App/>);await waitFor(()=>expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole('button',{name:'Araştırma'}));expect((screen.getByRole('button',{name:'Dosya ekle'}) as HTMLButtonElement).disabled).toBe(true);expect(backend.selectAttachments).not.toHaveBeenCalled();
  });
  it('shows reader failures without clearing the draft',async()=>{
    backend.selectAttachments.mockRejectedValueOnce('Arşivin açılmış boyutu sınırı aşıldı.');render(<App/>);
    const input=screen.getByRole('textbox',{name:'Aklından ne geçiyor?'});await waitFor(()=>expect((input as HTMLTextAreaElement).disabled).toBe(false));fireEvent.change(input,{target:{value:'Taslağım'}});
    fireEvent.click(screen.getByRole('button',{name:'Dosya ekle'}));await screen.findByRole('alert');expect((input as HTMLTextAreaElement).value).toBe('Taslağım');expect(screen.getByRole('alert').textContent).toContain('boyutu sınırı');
  });
  it.each([
    ['math', 'Matematik', 'Matematik sorunu veya işlemini yaz…', '2x + 5 = 17', 'Sonuç: $x = 6$.'],
    ['code', 'Kodlama', 'Ne geliştirmek istersin? Dili ve ihtiyacını yaz…', 'Rust ile kare fonksiyonu yaz', '```rust\nfn square(n: i32) -> i32 { n * n }\n```'],
  ])('creates a persistent %s conversation with formatted output', async (mode, label, placeholder, question, answer) => {
    backend.create.mockResolvedValue({ ...chat, mode });
    const response = { id:'answer',conversationId:chat.id,role:'assistant',content:answer,status:'complete',sources:[],createdAt:1 };
    backend.generate.mockResolvedValue(response);
    backend.messages.mockResolvedValue([{...response,id:'user',role:'user',content:question},response]);
    backend.list.mockResolvedValue([{...chat,mode}]);
    render(<App />);
    await waitFor(() => expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole('button',{name:label}));
    fireEvent.change(screen.getByRole('textbox',{name:placeholder}),{target:{value:question}});
    fireEvent.click(screen.getByRole('button',{name:'Gönder'}));
    await waitFor(() => expect(backend.generate).toHaveBeenCalledOnce());
    expect(backend.create).toHaveBeenCalledWith(mode);
    if (mode === 'math') await waitFor(() => expect(document.querySelector('.katex')).toBeTruthy());
    else await waitFor(() => expect(document.querySelector('pre code.language-rust')?.textContent).toContain('fn square'));
  });

  it('saves CPU/GPU choices and shows the active backend', async () => {
    backend.status.mockResolvedValue({state:'ready',detail:'GPU · Vulkan · RTX 3070 Ti',model:'Qwen3.5-9B',backend:'vulkan',deviceName:'RTX 3070 Ti'});
    render(<App />);
    await waitFor(() => expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    expect(screen.getByRole('button',{name:/Hazır · GPU/})).toBeTruthy();
    fireEvent.click(screen.getByRole('button',{name:'Ayarlar ve gizlilik'}));
    fireEvent.click(screen.getByRole('button',{name:'Model ve çeviri'}));
    const hardware = screen.getByRole('combobox',{name:'Çalıştırma donanımı'});
    fireEvent.change(hardware,{target:{value:'cpu'}});
    await waitFor(() => expect(backend.saveSettings).toHaveBeenLastCalledWith(expect.objectContaining({computeDevice:'cpu'})));
    await waitFor(() => expect((hardware as HTMLSelectElement).disabled).toBe(false));
    fireEvent.change(hardware,{target:{value:'gpu'}});
    await waitFor(() => expect(backend.saveSettings).toHaveBeenLastCalledWith(expect.objectContaining({computeDevice:'gpu'})));
  });

  it('restores the persisted hardware choice when switching to GPU fails', async () => {
    backend.settings.mockResolvedValue({...defaultSettings,computeDevice:'cpu'});
    backend.saveSettings.mockRejectedValueOnce('GPU başlatılamadı. Önceki donanım seçimi korundu.');
    render(<App />);
    await waitFor(() => expect((screen.getByRole('button',{name:'Ayarlar ve gizlilik'}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole('button',{name:'Ayarlar ve gizlilik'}));
    fireEvent.click(screen.getByRole('button',{name:'Model ve çeviri'}));
    const hardware = screen.getByRole('combobox',{name:'Çalıştırma donanımı'});
    fireEvent.change(hardware,{target:{value:'gpu'}});
    await waitFor(() => expect((hardware as HTMLSelectElement).value).toBe('cpu'));
    expect(screen.getByRole('alert').textContent).toContain('Önceki donanım seçimi korundu');
  });
  it('clears sent input immediately, preserves a new draft, and reloads the stopped response', async () => {
    let finish!: (message: Message) => void;
    let emit!: (event: GenerationEvent) => void;
    backend.generate.mockImplementation((_request: GenerateRequest, callback: typeof emit) => { emit = callback; return new Promise<Message>(resolve => { finish = resolve; }); });
    render(<App />);
    const input = await screen.findByRole('textbox', { name: 'Aklından ne geçiyor?' });
    await waitFor(() => expect((input as HTMLTextAreaElement).disabled).toBe(false));
    fireEvent.change(input, { target: { value: 'İlk mesaj' } }); fireEvent.click(screen.getByRole('button', { name: 'Gönder' }));
    expect((input as HTMLTextAreaElement).value).toBe('');
    await waitFor(() => expect(backend.generate).toHaveBeenCalledOnce());
    fireEvent.change(input, { target: { value: 'Bir sonraki taslağım' } });
    act(() => emit({ type: 'delta', text: 'Yarım yanıt' }));
    fireEvent.click(screen.getByRole('button', { name: 'Yanıtı durdur' }));
    expect(backend.cancel).toHaveBeenCalledWith(expect.any(String));
    const response: Message = { id: 'answer-1', conversationId: chat.id, role: 'assistant', content: 'Yarım yanıt', status: 'interrupted', sources: [], createdAt: 1 };
    backend.list.mockResolvedValue([chat]);
    backend.messages.mockResolvedValue([{ ...response, id: 'user-1', role: 'user', content: 'İlk mesaj', status: 'complete' }, response]);
    await act(async () => finish(response));
    expect(await screen.findByText('Yanıt durduruldu · Yazılan bölüm kaydedildi')).toBeTruthy();
    expect(screen.getByText('İlk mesaj')).toBeTruthy();
    expect((input as HTMLTextAreaElement).value).toBe('Bir sonraki taslağım');
  });

  it('starts web research directly with no confirmation field or dialog', async () => {
    backend.generate.mockResolvedValue({ status: 'complete' });
    backend.create.mockImplementation((mode: string) => Promise.resolve({ ...chat, mode }));
    render(<App />);
    await waitFor(() => expect(backend.settings).toHaveBeenCalledOnce());
    await screen.findByRole('button', { name: 'Ayarlar ve gizlilik' });
    fireEvent.click(screen.getByRole('button', { name: 'Araştırma' }));
    const input = screen.getByRole('textbox', { name: 'Hangi konuyu araştırmak istersin?' });
    await waitFor(() => expect((input as HTMLTextAreaElement).disabled).toBe(false));
    fireEvent.change(input, { target: { value: 'Rust resmi belgeleri' } }); fireEvent.click(screen.getByRole('button', { name: 'Araştır' }));
    await waitFor(() => expect(backend.generate).toHaveBeenCalledOnce());
    expect(backend.create).toHaveBeenCalledWith('research');
    const request = backend.generate.mock.calls[0][0];
    expect(request.text).toBe('Rust resmi belgeleri'); expect('researchConsent' in request).toBe(false);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('offers categorized settings and a saved main-window theme without the old start button', async () => {
    render(<App />);
    const settings = screen.getByRole('button', { name: 'Ayarlar ve gizlilik' });
    await waitFor(() => expect((settings as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(settings);
    expect(await screen.findByRole('dialog', { name: 'Ayarlar' })).toBeTruthy();
    expect(screen.queryByText('Yerel modeli başlat')).toBeNull();
    expect(screen.queryByText(/yerel SQLite veritabanına kaydedilir/)).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Görünüm' }));
    fireEvent.click(screen.getByRole('button', { name: 'Koyu' }));
    expect(document.querySelector('.app-shell')?.getAttribute('data-theme')).toBe('dark');
    await waitFor(() => expect(backend.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ theme: 'dark' })));
    expect(document.querySelector('.settings-dialog')?.closest('.app-shell')).toBeNull();
    expect(screen.queryByText(/0.1.0 ÖNİZLEME/)).toBeNull();
    fireEvent.change(screen.getByRole('textbox', { name: 'Ayarlarda ara' }), { target: { value: 'özel talimat' } });
    expect(screen.getByRole('textbox', { name: 'Özel talimatlar' })).toBeTruthy();
  });
});

