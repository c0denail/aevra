import { mkdir, readFile, writeFile, stat, copyFile, access } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { unzipSync } from 'fflate';
import path from 'node:path';
import { root, resources, specification, download, digest, runtimeFiles, verifyAssets } from './assets.mjs';

if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Bu varlık profili Windows x64 içindir.');
const cache = path.join(root, '.cache');
await mkdir(cache, { recursive: true });
const archive = path.join(cache, `llama-${specification.runtime.version}-cpu.zip`);
await download(specification.runtime, archive);
const runtime = path.join(resources, 'runtime');
await mkdir(runtime, { recursive: true });
for (const [entry, bytes] of Object.entries(unzipSync(await readFile(archive)))) {
  const name = path.posix.basename(entry);
  if (name === 'llama-server.exe' || /\.dll$/i.test(name)) await writeFile(path.join(runtime, name), bytes);
}
const gpuArchive = path.join(cache, `llama-${specification.gpuRuntime.version}-vulkan.zip`);
await download(specification.gpuRuntime, gpuArchive);
const gpuRuntime = path.join(runtime, 'vulkan');
await mkdir(gpuRuntime, { recursive: true });
for (const [entry, bytes] of Object.entries(unzipSync(await readFile(gpuArchive)))) {
  const name = path.posix.basename(entry);
  if (name === 'llama-server.exe' || /\.dll$/i.test(name)) await writeFile(path.join(gpuRuntime, name), bytes);
}
// Ship the Microsoft C++ runtime app-locally; end users do not install it.
const vswhere = path.join(process.env['ProgramFiles(x86)'] || 'C:/Program Files (x86)', 'Microsoft Visual Studio/Installer/vswhere.exe');
let redist = process.env.AEVRA_VC_REDIST_DIR;
if (!redist) {
  const result = spawnSync(vswhere, ['-latest', '-products', '*', '-find', 'VC/Redist/MSVC/**/x64/Microsoft.VC143.CRT/msvcp140.dll'], { encoding: 'utf8' });
  const file = result.stdout?.trim().split(/\r?\n/)[0];
  if (file) redist = path.dirname(file);
}
if (!redist) throw new Error('Visual Studio C++ Redistributable kaynak klasörü bulunamadı. AEVRA_VC_REDIST_DIR ayarlayın.');
await access(path.join(redist, 'vcruntime140.dll'));
const { readdir } = await import('node:fs/promises');
for (const name of await readdir(redist)) {
  if (/\.dll$/i.test(name)) {
    await copyFile(path.join(redist, name), path.join(runtime, name));
    await copyFile(path.join(redist, name), path.join(gpuRuntime, name));
  }
}
await download(specification.model, path.join(resources, 'models', specification.model.file));
await download(specification.projector, path.join(resources, 'models', specification.projector.file));
await download(specification.pdfFont,path.join(resources,'fonts',specification.pdfFont.file));
const pdfArchive = path.join(cache, `pdfium-${specification.pdfium.version}.tgz`);
await download(specification.pdfium, pdfArchive);
const pdfDirectory = path.join(cache, `pdfium-${specification.pdfium.version}`);
await mkdir(pdfDirectory, { recursive: true });
const extracted = spawnSync('tar.exe', ['-xf', pdfArchive, '-C', pdfDirectory], { stdio: 'inherit' });
if (extracted.status !== 0) throw new Error('PDF motoru açılamadı.');
await copyFile(path.join(pdfDirectory, 'bin/pdfium.dll'), path.join(runtime, 'pdfium.dll'));
for (const name of await readdir(path.join(pdfDirectory, 'licenses'))) {
  await copyFile(path.join(pdfDirectory, 'licenses', name), path.join(resources, 'licenses', `PDFium-${name}.txt`));
}
await download(specification.webview, path.join(cache, specification.webview.file));
const files = [];
const font=path.join(resources,'fonts',specification.pdfFont.file);
files.push({path:`fonts/${specification.pdfFont.file}`,size:(await stat(font)).size,sha256:await digest(font)});
for (const name of await runtimeFiles()) {
  const file = path.join(runtime, name);
  files.push({ path: `runtime/${name}`, size: (await stat(file)).size, sha256: await digest(file) });
}
for (const name of (await readdir(path.join(resources, 'licenses'))).filter(name => name.endsWith('.txt')).sort()) {
  const file = path.join(resources, 'licenses', name);
  await readFile(file);
  files.push({ path: `licenses/${name}`, size: (await stat(file)).size, sha256: await digest(file) });
}
await writeFile(path.join(resources, 'asset-lock.json'), JSON.stringify({
  schemaVersion: 1, runtimeVersion: specification.runtime.version, gpuRuntimeVersion: specification.gpuRuntime.version,
  model: { path: `models/${specification.model.file}`, size: specification.model.size, sha256: specification.model.sha256 }, files,
  projector: { path: `models/${specification.projector.file}`, size: specification.projector.size, sha256: specification.projector.sha256 },
}, null, 2) + '\n');
await verifyAssets();
