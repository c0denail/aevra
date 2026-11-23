import type { AppSettings } from './types';

export const defaultSettings: AppSettings = {
  computeDevice: 'auto',
  theme: 'light', defaultMode: 'chat', sendWithEnter: true, fontSize: 14,
  compactLayout: false, reduceMotion: false, showTimestamps: true, showSuggestions: true,
  completionSound: false, preferredName: '', responseStyle: 'balanced', customInstructions: '',
  temperature: 0.7, maxTokens: 1024, translationLanguage: 'İngilizce',
  webResearchEnabled: true, researchSourceLimit: 3,
  memoryEnabled:true,
};
export const languages = ['Türkçe', 'İngilizce', 'Almanca', 'Fransızca', 'İspanyolca', 'İtalyanca', 'Arapça', 'Japonca', 'Korece', 'Çince', 'Rusça'];

export function previewSettings(): AppSettings {
  try { return { ...defaultSettings, ...JSON.parse(localStorage.getItem('aevra-interface-settings') || '{}') }; }
  catch { return { ...defaultSettings }; }
}

export function playCompletionSound() {
  try {
    const context = new AudioContext(); const oscillator = context.createOscillator(); const gain = context.createGain();
    oscillator.connect(gain); gain.connect(context.destination); oscillator.frequency.value = 660;
    gain.gain.setValueAtTime(0.035, context.currentTime); gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + 0.15);
    oscillator.start(); oscillator.stop(context.currentTime + 0.16); oscillator.onended = () => { void context.close(); };
  } catch { /* A muted or unavailable audio device does not interrupt a conversation. */ }
}
