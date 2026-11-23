// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BrowserStatus } from './types';
const backend=vi.hoisted(()=>({browserStatus:vi.fn(),browserLayout:vi.fn(),browserControl:vi.fn(),browserTakeover:vi.fn(),browserClose:vi.fn(),onBrowserUpdated:vi.fn()}));
vi.mock('./api',()=>({desktop:true,api:backend}));
import ResearchBrowser from './ResearchBrowser';
const empty:BrowserStatus={url:'',title:'',loading:false,automating:false,detail:'',steps:[]};
beforeEach(()=>{vi.clearAllMocks();backend.browserStatus.mockResolvedValue(empty);backend.browserLayout.mockResolvedValue(undefined);backend.browserControl.mockResolvedValue(undefined);backend.browserTakeover.mockResolvedValue(undefined);backend.onBrowserUpdated.mockResolvedValue(()=>{});});
afterEach(cleanup);
describe('visible research browser',()=>{
  it('closes the remote view when leaving research, while collapsing it only hides the panel',async()=>{
    backend.browserClose.mockResolvedValue(undefined);
    const props={visible:true,suspended:false,enabled:true,onClose:vi.fn(),onError:vi.fn()};const view=render(<ResearchBrowser {...props}/>);
    view.rerender(<ResearchBrowser {...props} visible={false}/>);await waitFor(()=>expect(backend.browserLayout).toHaveBeenLastCalledWith(null));expect(backend.browserClose).not.toHaveBeenCalled();
    view.rerender(<ResearchBrowser {...props} visible={false} active={false}/>);await waitFor(()=>expect(backend.browserClose).toHaveBeenCalledOnce());
  });
  it('shows automatic navigation and gives the user control without requiring step-by-step instructions',async()=>{
    let emit!:(status:BrowserStatus)=>void;backend.onBrowserUpdated.mockImplementation(async callback=>{emit=callback;return()=>{};});
    render(<ResearchBrowser visible suspended={false} enabled onClose={vi.fn()} onError={vi.fn()}/>);
    await waitFor(()=>expect(backend.browserStatus).toHaveBeenCalledOnce());
    act(()=>emit({...empty,url:'https://example.org/article',title:'Makale',automating:true,detail:'Açılıyor · Makale',steps:[{text:'Aranıyor · Batarya',url:'https://example.org/search'}]}));
    expect((screen.getByRole('textbox',{name:'Web adresi'}) as HTMLInputElement).value).toBe('https://example.org/article');
    expect((screen.getByRole('button',{name:'Sayfayı yenile'}) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('button',{name:'Kontrolü al'}));await waitFor(()=>expect(backend.browserTakeover).toHaveBeenCalledOnce());
    act(()=>emit({...empty,url:'https://example.org/article'}));
    fireEvent.change(screen.getByRole('textbox',{name:'Web adresi'}),{target:{value:'www.rust-lang.org/learn'}});fireEvent.click(screen.getByRole('button',{name:'Adresi aç'}));
    await waitFor(()=>expect(backend.browserControl).toHaveBeenCalledWith({action:'navigate',url:'https://www.rust-lang.org/learn'}));
  });
  it('hides the native view for settings, collapsed panels and disabled web research',async()=>{
    const measure=vi.spyOn(Element.prototype,'getBoundingClientRect').mockReturnValue({x:600,y:150,width:400,height:500,top:150,left:600,right:1000,bottom:650,toJSON:()=>({})});
    const props={visible:true,suspended:false,enabled:true,onClose:vi.fn(),onError:vi.fn()};const view=render(<ResearchBrowser {...props}/>);
    await waitFor(()=>expect(backend.browserLayout).toHaveBeenLastCalledWith({x:600,y:150,width:400,height:500}));
    view.rerender(<ResearchBrowser {...props} suspended/>);await waitFor(()=>expect(backend.browserLayout).toHaveBeenLastCalledWith(null));
    view.rerender(<ResearchBrowser {...props} visible={false}/>);expect(screen.queryByRole('complementary')).toBeNull();
    view.rerender(<ResearchBrowser {...props} enabled={false}/>);await waitFor(()=>expect(backend.browserLayout).toHaveBeenLastCalledWith(null));
    expect(screen.getByRole('status').textContent).toContain('kapalı');measure.mockRestore();
  });
});
