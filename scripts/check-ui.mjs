// Optional visual review of the real production frontend; no model, user data or external pages.
import {chromium} from 'playwright';
import {preview} from 'vite';
import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const output=path.join(root,'output/ui');await mkdir(output,{recursive:true});
const server=await preview({root,preview:{host:'127.0.0.1',port:1431,strictPort:true},logLevel:'error'});
let browser;
const errors=[];const captures=[];
try {
  browser=await chromium.launch({channel:process.env.AEVRA_TEST_BROWSER||'chrome',headless:true});
  const context=await browser.newContext({viewport:{width:1240,height:820},deviceScaleFactor:1,locale:'tr-TR',timezoneId:'Europe/Istanbul',reducedMotion:'reduce'});
  const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
  await page.route('**/*',route=>new URL(route.request().url()).hostname==='127.0.0.1'?route.continue():route.abort());
  await page.goto('http://127.0.0.1:1431');
  await page.getByRole('region',{name:'Kişisel çalışma alanı'}).waitFor();await page.evaluate(()=>document.fonts.ready);
  // Desktop Tauri has no browser-preview band. Hide only that band to review its real geometry.
  await page.addStyleTag({content:'.preview-banner{display:none!important}'});
  async function capture(name){
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),true,`${name}: horizontal overflow`);
    const composer=await page.locator('.composer').boundingBox();assert(composer&&composer.y+composer.height<=page.viewportSize().height,`${name}: composer clipped`);
    await page.screenshot({path:path.join(output,`${name}.png`),animations:'disabled'});captures.push(name);
  }
  await capture('aevra-home-light');
  await page.getByRole('button',{name:'Profil ve kişiselleştirme',exact:true}).click();
  await page.getByRole('textbox',{name:'Tercih edilen ad',exact:true}).waitFor();
  await page.getByRole('button',{name:'Ayarları kapat',exact:true}).click();
  await page.setViewportSize({width:820,height:620});await capture('aevra-home-compact');
  await page.getByRole('button',{name:'Hızlı başlangıç: Metin çevir'}).click();await page.getByRole('combobox',{name:'Hedef dil'}).selectOption('Almanca');await capture('aevra-translate-compact');
  await page.getByRole('button',{name:'Araştırma',exact:true}).click();await page.getByRole('complementary',{name:'Araştırma tarayıcısı'}).waitFor();await capture('aevra-research-compact');
  await page.getByRole('button',{name:'Tarayıcıyı küçült',exact:true}).click();assert.equal(await page.getByRole('complementary',{name:'Araştırma tarayıcısı'}).count(),0);
  await page.getByRole('button',{name:'Ana sayfa',exact:true}).click();await page.setViewportSize({width:1240,height:820});
  await page.getByRole('button',{name:'Ayarlar ve gizlilik',exact:true}).click();await page.getByRole('button',{name:'Görünüm',exact:true}).click();await page.getByRole('button',{name:'Koyu',exact:true}).click();
  await page.getByRole('button',{name:'Ayarları kapat',exact:true}).click();
  assert.equal(await page.locator('.app-shell').getAttribute('data-theme'),'dark');await capture('aevra-home-dark');
  assert.deepEqual(errors,[],'browser runtime errors');
  const report={ok:true,captures,widths:[1240,820],heights:[820,620],horizontalOverflow:false,composerVisible:true,runtimeErrors:errors,previewBandHidden:true,modelStarted:false};
  await writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
} finally {await browser?.close();await new Promise(resolve=>server.httpServer.close(resolve));}
