// @vitest-environment jsdom
import {cleanup,fireEvent,render,screen} from '@testing-library/react';
import {afterEach,describe,expect,it,vi} from 'vitest';
import HomeDashboard from './HomeDashboard';
import {defaultSettings} from './preferences';
afterEach(cleanup);
const callbacks=()=>({onStart:vi.fn(),onOpen:vi.fn(),onSettings:vi.fn(),onFocus:vi.fn()});
describe('personal workspace dashboard',()=>{
  it('opens real working areas and prepares a starter without generating or inventing activity',()=>{
    const events=callbacks();render(<HomeDashboard preferences={defaultSettings} conversations={[]} disabled={false} {...events}/>);
    expect(screen.getByText('0 konuşma')).toBeTruthy();expect(screen.getByText('İlk fikrine yer açalım.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button',{name:'Araştırma alanını aç'}));expect(events.onStart).toHaveBeenCalledWith('research');
    fireEvent.click(screen.getByRole('button',{name:'Hızlı başlangıç: Bir şey geliştir'}));expect(events.onStart).toHaveBeenCalledWith('code',expect.stringContaining('proje'));
    fireEvent.click(screen.getByRole('button',{name:'Aevra’ya sor'}));expect(events.onFocus).toHaveBeenCalledOnce();
  });
  it('shows persisted preferences and opens an existing unarchived conversation',()=>{
    const events=callbacks();const item={id:'chat-1',mode:'chat' as const,title:'Birlikte yazalım',updatedAt:1,archived:false};
    render(<HomeDashboard preferences={{...defaultSettings,preferredName:'Deniz',memoryEnabled:false,translationLanguage:'Almanca',responseStyle:'detailed'}} conversations={[item,{...item,id:'old',title:'Arşivde',archived:true}]} disabled={false} {...events}/>);
    expect(screen.getByRole('heading',{level:1}).textContent).toContain('Deniz');expect(screen.getByText('Almanca')).toBeTruthy();expect(screen.getByText('Ayrıntılı')).toBeTruthy();expect(screen.queryByText('Arşivde')).toBeNull();
    fireEvent.click(screen.getByRole('button',{name:'Son konuşmayı aç: Birlikte yazalım'}));expect(events.onOpen).toHaveBeenCalledWith(item);
    fireEvent.click(screen.getByRole('button',{name:/Kişisel hafıza/}));expect(events.onSettings).toHaveBeenCalledWith('memory');
  });
  it('respects disabled actions, hidden suggestions and the web preference',()=>{
    render(<HomeDashboard preferences={{...defaultSettings,showSuggestions:false,webResearchEnabled:false}} conversations={[]} disabled {...callbacks()}/>);
    expect(screen.queryByRole('heading',{name:'Küçük bir başlangıç'})).toBeNull();expect(screen.getByText('Web kapalı')).toBeTruthy();expect((screen.getByRole('button',{name:'Araştırma alanını aç'}) as HTMLButtonElement).disabled).toBe(true);
  });
});
