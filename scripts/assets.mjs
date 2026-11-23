import { createReadStream, createWriteStream } from 'node:fs';
import { mkdir, readFile, stat, rename, rm, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { pipeline } from 'node:stream/promises';
import { Readable } from 'node:stream';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

export const root = fileURLToPath(new URL('../', import.meta.url));
export const resources = path.join(root, 'src-tauri/resources');
export const specification = JSON.parse(await readFile(path.join(root, 'distribution/assets.json'), 'utf8'));

export async function digest(file) {
  const hash = createHash('sha256');
  for await (const part of createReadStream(file)) hash.update(part);
  return hash.digest('hex');
}

export async function matches(file, spec) {
  try { return (await stat(file)).size === spec.size && await digest(file) === spec.sha256; }
  catch { return false; }
}

export async function download(spec, destination) {
  if (await matches(destination, spec)) { console.log(`Doğrulandı: ${path.basename(destination)}`); return; }
  await mkdir(path.dirname(destination), { recursive: true });
  const temporary = `${destination}.partial`;
  console.log(`İndiriliyor: ${path.basename(destination)} (${Math.round(spec.size / 1e6)} MB)`);
  const response = await fetch(spec.url, { signal: AbortSignal.timeout(3_600_000) });
  if (!response.ok || !response.body) throw new Error(`İndirme başarısız: HTTP ${response.status}`);
  await pipeline(Readable.fromWeb(response.body), createWriteStream(temporary));
  if (!await matches(temporary, spec)) { await rm(temporary, { force: true }); throw new Error('SHA-256 veya boyut eşleşmedi; dosya reddedildi.'); }
  await rename(temporary, destination);
}

export async function runtimeFiles(directory = path.join(resources, 'runtime')) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (entry.isDirectory()) files.push(...(await runtimeFiles(path.join(directory, entry.name))).map(name => `${entry.name}/${name}`));
    else if (/\.(exe|dll)$/i.test(entry.name)) files.push(entry.name);
  }
  return files.sort();
}

export async function verifyAssets() {
  const modelPath = path.join(resources, 'models', specification.model.file);
  if (!await matches(modelPath, specification.model)) throw new Error('Model eksik veya bütünlük kontrolü başarısız. Önce pnpm assets:fetch çalıştırın.');
  if (!await matches(path.join(resources, 'models', specification.projector.file), specification.projector)) throw new Error('Görsel model bileşeni eksik veya bozuk.');
  if(!await matches(path.join(resources,'fonts',specification.pdfFont.file),specification.pdfFont))throw new Error('PDF yazı tipi eksik veya bozuk.');
  const lock = JSON.parse(await readFile(path.join(resources, 'asset-lock.json'), 'utf8'));
  if (lock.runtimeVersion !== specification.runtime.version || lock.model.sha256 !== specification.model.sha256) throw new Error('Varlık kilidi güncel değil.');
  if (!lock.files.some(file => file.path === 'runtime/llama-server.exe')) throw new Error('llama-server.exe pakette yok.');
  if (!lock.files.some(file => file.path === 'runtime/pdfium.dll') || lock.projector?.sha256 !== specification.projector.sha256) throw new Error('Belge/görsel bileşeni kilidi güncel değil.');
  if (lock.gpuRuntimeVersion !== specification.gpuRuntime.version) throw new Error('GPU motoru kilidi güncel değil.');
  for (const name of ['llama-server.exe', 'ggml-vulkan.dll', 'msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll']) {
    if (!lock.files.some(file => file.path === `runtime/vulkan/${name}`)) throw new Error(`GPU çalışma dosyası eksik: ${name}`);
  }
  for (const name of ['msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll']) {
    if (!lock.files.some(file => file.path.toLowerCase() === `runtime/${name}`)) throw new Error(`C++ çalışma dosyası eksik: ${name}`);
  }
  for (const file of lock.files) {
    if (!/^(runtime\/(vulkan\/)?|licenses\/|fonts\/)[\w.-]+$/.test(file.path)) throw new Error('Geçersiz paket yolu.');
    if (!await matches(path.join(resources, file.path), file)) throw new Error(`Paket dosyası doğrulanamadı: ${file.path}`);
  }
  console.log('Model, motor ve lisans dosyaları doğrulandı.');
  return lock;
}
