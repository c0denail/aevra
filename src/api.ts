import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {MemoryState,MemoryNotice,BrowserStatus,BrowserRect,BrowserAction} from './types';
import type { Attachment, AppSettings, Conversation, EngineStatus, GenerateRequest, GenerationEvent, Message, Mode, StorageInfo } from './types';
import { previewSettings } from './preferences';

export const desktop = isTauri();
export const api = {
  browserStatus:()=>desktop?invoke<BrowserStatus>('browser_status'):Promise.resolve<BrowserStatus>({url:'',title:'',loading:false,automating:false,detail:'',steps:[]}),
  browserLayout:(rect:BrowserRect|null)=>desktop?invoke<void>('browser_layout',{rect}):Promise.resolve(),
  browserControl:(action:BrowserAction)=>invoke<void>('browser_control',{action}),
  browserTakeover:()=>invoke<void>('browser_takeover'),
  browserClose:()=>desktop?invoke<void>('browser_close'):Promise.resolve(),
  onBrowserUpdated:(callback:(status:BrowserStatus)=>void)=>desktop?listen<BrowserStatus>('research-browser',event=>callback(event.payload)):Promise.resolve(()=>{}),
  list: () => desktop ? invoke<Conversation[]>('list_conversations') : Promise.resolve([]),
  create: (mode: Mode,memoryDisabled=false) => invoke<Conversation>('create_conversation', { mode,memoryDisabled }),
  memories:()=>desktop?invoke<MemoryState>('get_memories'):Promise.resolve<MemoryState>({items:[],blockedKeys:[]}),
  editMemory:(key:string,value:string)=>invoke<void>('edit_memory',{key,value}),
  deleteMemory:(key:string)=>invoke<void>('delete_memory',{key}),
  clearMemories:()=>invoke<void>('clear_memories'),
  allowMemory:(key:string)=>invoke<void>('allow_memory',{key}),
  undoMemories:(batchId:string)=>invoke<number>('undo_memories',{batchId}),
  conversationMemory:(conversationId:string,disabled:boolean)=>invoke<void>('set_conversation_memory',{conversationId,disabled}),
  onMemoryUpdated:(callback:(notice:MemoryNotice)=>void)=>desktop?listen<MemoryNotice>('memory-updated',event=>callback(event.payload)):Promise.resolve(()=>{}),
  messages: (conversationId: string) => invoke<Message[]>('get_messages', { conversationId }),
  remove: (conversationId: string) => invoke<void>('delete_conversation', { conversationId }),
  status: () => desktop ? invoke<EngineStatus>('engine_status') : Promise.resolve<EngineStatus>({
    state: 'preview', model: 'Qwen3.5-9B · Q4_K_M',
    detail: 'Arayüz önizlemesi. Yerel model ve geçmiş için Aevra masaüstü uygulamasını açın.',
  }),
  settings: () => desktop ? invoke<AppSettings>('get_settings') : Promise.resolve(previewSettings()),
  saveSettings: (settings: AppSettings) => desktop ? invoke<void>('save_settings', { settings }) : Promise.resolve(localStorage.setItem('aevra-interface-settings', JSON.stringify(settings))),
  archive: (conversationId: string, archived: boolean) => invoke<void>('archive_conversation', { conversationId, archived }),
  archiveAll: () => invoke<void>('archive_all'),
  clearHistory: () => invoke<void>('clear_history'),
  storage: () => desktop ? invoke<StorageInfo>('storage_info') : Promise.resolve<StorageInfo>({ dataDir: '', databaseBytes: 0, modelBytes: 0, conversationCount: 0, messageCount: 0 }),
  openDataFolder: () => invoke<void>('open_data_folder'),
  exportHistory: () => invoke<string | null>('export_history'),
  importHistory: () => invoke<number | null>('import_history'),
  exportPdf:(conversationId:string,messageId:string,mathAssets:import('./mathPdf').MathAsset[]=[])=>invoke<string|null>('export_message_pdf',{conversationId,messageId,mathAssets}),
  selectAttachments: (currentIds: string[]) => invoke<Attachment[]>('select_attachments', { currentIds }),
  discardAttachments: (ids: string[]) => invoke<void>('discard_attachments', { ids }),
  cancel: (requestId: string) => invoke<void>('cancel_generation', { requestId }),
  openSource: (url: string) => invoke<void>('open_source', { url }),
  generate: (request: GenerateRequest, onEvent: (event: GenerationEvent) => void) => {
    const onEventChannel = new Channel<GenerationEvent>();
    onEventChannel.onmessage = onEvent;
    return invoke<Message>('generate', { request, onEvent: onEventChannel });
  },
};
