import { spawnSync } from 'node:child_process';
import { copyFile, mkdir, writeFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { verifyAssets, root, resources, specification, matches, digest } from './assets.mjs';

if (process.platform !== 'win32') throw new Error('Windows kurulumu Windows üzerinde üretilir.');
if (!process.argv.includes('--archive-only')) {
const assetLock=await verifyAssets();
const webview = path.join(root, '.cache', specification.webview.file);
if (!await matches(webview, specification.webview)) throw new Error('Çevrimdışı WebView2 paketi eksik. pnpm assets:fetch çalıştırın.');
const result = spawnSync(process.execPath, [path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build', '--no-bundle'], { cwd: root, stdio: 'inherit' });
if (result.status !== 0) process.exit(result.status ?? 1);
const payload = path.resolve(process.env.AEVRA_PAYLOAD_DIR || path.join(root, 'artifacts', 'Aevra-memory'));
const expectedModels=new Set([path.basename(assetLock.model.path),path.basename(assetLock.projector.path)]);
let existingModels=[];
try {existingModels=await readdir(path.join(payload,'resources/models'));}catch(error){if(error.code!=='ENOENT')throw error;}
if(existingModels.some(name=>name.endsWith('.gguf')&&!expectedModels.has(name)))throw new Error('Paket klasöründe önceki model dosyaları var. AEVRA_PAYLOAD_DIR ile yeni bir klasör seçin.');
await mkdir(payload, { recursive: true });
const executable=path.join(root,'target/release/aevra-desktop.exe');
if(!await matches(path.join(payload,'Aevra.exe'),{size:(await stat(executable)).size,sha256:await digest(executable)}))await copyFile(executable,path.join(payload,'Aevra.exe'));
// Package the verified manifest only; cached older models must not enter the new installer.
for(const file of [...assetLock.files,assetLock.model,assetLock.projector]){
  const target=path.join(payload,'resources',file.path);
  await mkdir(path.dirname(target),{recursive:true});
  // An open Aevra may be using the same verified model and engine files.
  if(!await matches(target,file))await copyFile(path.join(resources,file.path),target);
}
await copyFile(path.join(resources,'asset-lock.json'),path.join(payload,'resources/asset-lock.json'));
const compiler = process.env.AEVRA_ISCC || path.join(root, 'node_modules/innosetup/bin/ISCC.exe');
// Keep large model payloads in installer slices, then deliver a single ZIP64 bundle.
const setup = spawnSync(compiler, ['/Qp', `/DPayloadDir=${payload}`, path.join(root, 'distribution/Aevra.iss')], { cwd: root, stdio: 'inherit' });
if (setup.status !== 0) process.exit(setup.status ?? 1);
console.log(`Uygulama klasörü hazır: ${payload}`);
}
const prefix = 'Aevra-0.1.0-windows-x64-setup';
const setupDirectory = path.join(root, 'artifacts/installer');
const files = (await readdir(setupDirectory)).filter(name => name.startsWith(prefix) && /\.(exe|bin)$/.test(name)).sort();
if (!files.some(name => name.endsWith('.exe')) || !files.some(name => name.endsWith('.bin'))) throw new Error('Kurulum EXE ve veri dilimleri bulunamadı.');
const checksums = [];
for (const file of files) checksums.push(`${await digest(path.join(setupDirectory, file))}  ${file}`);
await writeFile(path.join(setupDirectory, 'SHA256SUMS.txt'), checksums.join('\n') + '\n');
const archive = path.join(root, 'artifacts/Aevra-0.1.0-windows-x64-offline.zip');
const zipped = spawnSync('tar.exe', ['-a', '-cf', archive, '--options', 'zip:compression=store', '-C', setupDirectory, ...files, 'SHA256SUMS.txt'], { stdio: 'inherit' });
if (zipped.status !== 0) process.exit(zipped.status ?? 1);
await writeFile(`${archive}.sha256`, `${await digest(archive)}  ${path.basename(archive)}\n`);
console.log(`Çevrimdışı kurulum paketi hazır: ${archive}`);
