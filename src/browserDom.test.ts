import { describe, expect, it, vi } from 'vitest';
import { JSDOM } from 'jsdom';
import script from '../src-tauri/src/browser_dom.js?raw';

interface DomResult {text:string;links:{url:string}[];searchFields:unknown[];ok?:boolean;error?:string;blocked:boolean;searchResults:boolean}
function fixture(html:string,url='https://example.org/'){const dom=new JSDOM(html,{url,runScripts:'outside-only'});dom.window.HTMLElement.prototype.scrollIntoView=vi.fn();const run=dom.window.eval(`(${script})`) as (command:Record<string,unknown>)=>DomResult;return{dom,run};}
describe('remote browser DOM operations',()=>{
  it('extracts visible public evidence without input values, scripts or hidden navigation',()=>{
    const {dom,run}=fixture('<title>Kaynak</title><main><p>Okunacak makale</p><script>SECRET_SCRIPT</script><input name="q" value="PRIVATE_DRAFT"><textarea>PRIVATE_TEXT</textarea><nav>PRIVATE_NAV</nav><p hidden>HIDDEN</p><a href="https://example.net/page">Kaynak bağlantısı</a><a href="file:///C:/secret">Yerel</a></main>');
    const page=run({action:'snapshot'});expect(page.text).toContain('Okunacak makale');for(const secret of ['SECRET_SCRIPT','PRIVATE_DRAFT','PRIVATE_TEXT','PRIVATE_NAV','HIDDEN'])expect(page.text).not.toContain(secret);
    expect(page.links).toHaveLength(1);expect(page.links[0].url).toBe('https://example.net/page');dom.window.close();
  });
  it('fills only recognized GET search forms and safely preserves literal query text',()=>{
    const {dom,run}=fixture('<form action="/search"><input name="q" aria-label="Site içinde ara"><button type="submit">Ara</button></form>');
    const submit=vi.fn();dom.window.HTMLFormElement.prototype.requestSubmit=submit;
    const query='batarya "ömrü"; window.alert(1)';
    expect(run({action:'snapshot'}).searchFields).toHaveLength(1);
    expect(run({action:'search',index:0,query,expectedUrl:'https://example.org/'}).ok).toBe(true);
    expect(dom.window.document.querySelector('input')?.value).toBe(query);expect(submit).toHaveBeenCalledOnce();dom.window.close();
  });
  it('excludes login, POST, upload, email and hidden forms and rejects changed pages and cross-site search',()=>{
    const {dom,run}=fixture('<form method="post"><input name="q"></form><form><input name="search"><input type="password"></form><form><input name="query"><input type="file"></form><form><input name="s"><input type="email"></form><form hidden><input type="search"></form><form action="https://other.example/search"><input name="q"></form>');
    expect(run({action:'snapshot'}).searchFields).toHaveLength(1);
    expect(run({action:'search',index:0,query:'test',expectedUrl:'https://changed.example/'}).error).toContain('değişti');
    expect(run({action:'search',index:0,query:'test',expectedUrl:'https://example.org/'}).error).toContain('başka bir site');dom.window.close();
  });
  it('recognizes result pages and challenge pages instead of treating them as evidence',()=>{
    const {dom,run}=fixture('<title>Just a moment...</title><div class="result"><a class="result__a" href="https://example.net/">Sonuç</a></div>','https://html.duckduckgo.com/html/');
    const page=run({action:'snapshot'});expect(page.blocked).toBe(true);expect(page.searchResults).toBe(true);dom.window.close();
  });
  it('supports recognized POST search forms while keeping ordinary articles readable',()=>{
    const {dom,run}=fixture('<title>Bir makale</title><article><h2><a href="https://example.net/">Başlık</a></h2><p>Makale metni</p></article><form action="/search" method="post"><input type="search" name="q"></form>');
    expect(run({action:'snapshot'}).searchResults).toBe(false);expect(run({action:'snapshot'}).searchFields).toHaveLength(1);dom.window.close();
  });
  it('reads Google result links and fills its multiline search field',()=>{
    const {dom,run}=fixture('<title>Google</title><form action="/search"><input type="file" hidden><textarea name="q" aria-label="Ara"></textarea><button type="submit">Google Ara</button></form><div id="search"><a href="https://doc.rust-lang.org/book/"><h3>The Rust Book</h3></a></div>','https://www.google.com/search?q=rust');
    const submit=vi.fn();dom.window.HTMLFormElement.prototype.requestSubmit=submit;
    const page=run({action:'snapshot'});expect(page.searchResults).toBe(true);expect(page.links[0].url).toBe('https://doc.rust-lang.org/book/');expect(page.searchFields).toHaveLength(1);
    expect(run({action:'search',index:0,query:'Türkçe karakter: ömür',expectedUrl:'https://www.google.com/search?q=rust'}).ok).toBe(true);
    expect(dom.window.document.querySelector('textarea')?.value).toBe('Türkçe karakter: ömür');expect(submit).toHaveBeenCalledOnce();
    Object.defineProperty(dom.window.document.querySelector('input[type=file]'),'files',{value:[new dom.window.File(['private'],'private.txt')]});
    expect(run({action:'snapshot'}).searchFields).toHaveLength(0);dom.window.close();
  });
  it('does not treat Google consent or unusual-traffic screens as research evidence',()=>{
    for(const url of ['https://www.google.com/sorry/index','https://consent.google.com/m']){
      const {dom,run}=fixture('<title>Google</title><main>'+ 'Uzun doğrulama metni. '.repeat(100) +'</main>',url);expect(run({action:'snapshot'}).blocked).toBe(true);dom.window.close();
    }
  });
  it('keeps Google documentation readable and distinguishes search results about traffic warnings',()=>{
    const {dom,run}=fixture('<title>Google Search Help</title><main>Resolve unusual traffic from your computer network.</main>','https://support.google.com/websearch/answer/86640');
    expect(run({action:'snapshot'}).searchResults).toBe(false);expect(run({action:'snapshot'}).blocked).toBe(false);dom.window.close();
    const search=fixture('<title>Search</title><div id="search"><a href="https://support.google.com/websearch/answer/86640"><h3>Resolve unusual traffic from your computer network</h3></a></div>','https://www.google.com/search?q=unusual+traffic');
    expect(search.run({action:'snapshot'}).blocked).toBe(false);search.dom.window.close();
  });
});
