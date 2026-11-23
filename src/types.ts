export type Mode = 'chat' | 'translate' | 'research' | 'math' | 'code';
export type ComputeDevice = 'auto' | 'cpu' | 'gpu';
export interface Conversation { id: string; title: string; mode: Mode; updatedAt: number; archived: boolean; memoryDisabled?:boolean }
export interface Source { id: number; title: string; url: string; excerpt: string; accessedAt: number }
export interface Attachment { id: string; name: string; kind: string; bytes: number; pages: number; entries: number; warnings: string[]; images: {label: string; dataUrl: string}[] }
export interface Message { id: string; conversationId: string; role: 'user' | 'assistant'; content: string; createdAt: number; sources: Source[]; status: 'complete' | 'interrupted' | 'failed' | 'limited'; attachments?: Attachment[] }
export interface EngineStatus { state: 'ready' | 'starting' | 'error' | 'stopped' | 'missing' | 'preview'; detail: string; model: string; requestedDevice?: ComputeDevice; backend?: 'cpu' | 'vulkan' | null; deviceName?: string | null; fallbackReason?: string | null }
export interface GenerateRequest { requestId: string; conversationId: string; text: string; targetLanguage: string | null; attachmentIds?: string[] }
export type GenerationEvent = { type: 'delta' | 'status'; text: string } | { type: 'sources'; sources: Source[] };
export interface AppSettings {
  computeDevice: ComputeDevice;
  theme: 'light' | 'dark'; defaultMode: Mode; sendWithEnter: boolean; fontSize: number;
  compactLayout: boolean; reduceMotion: boolean; showTimestamps: boolean; showSuggestions: boolean;
  completionSound: boolean; preferredName: string; responseStyle: 'concise' | 'balanced' | 'detailed';
  customInstructions: string; temperature: number; maxTokens: number; translationLanguage: string;
  webResearchEnabled: boolean; researchSourceLimit: number;
  memoryEnabled:boolean;
}
export interface PersonalMemory {key:string;label:string;value:string;certainty:'explicit'|'inferred'|'pending'|'edited';evidenceCount:number;createdAt:number;updatedAt:number;expiresAt:number|null;sourceQuote:string;sourceConversationId:string;autoUpdateDisabled:boolean}
export interface MemoryState {items:PersonalMemory[];blockedKeys:string[]}
export interface MemoryNotice {batchId:string;count:number;error?:string}
export interface StorageInfo { dataDir: string; databaseBytes: number; modelBytes: number; conversationCount: number; messageCount: number }
export interface BrowserStatus { url: string; title: string; loading: boolean; automating: boolean; detail: string; steps: {text: string; url: string}[] }
export interface BrowserRect { x: number; y: number; width: number; height: number }
export type BrowserAction = {action:'navigate';url:string} | {action:'back'|'forward'|'reload'|'clear'};
