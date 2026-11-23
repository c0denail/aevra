import { readdir, readFile, writeFile, realpath } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { root, resources } from './assets.mjs';

const entries = new Map();
async function add(directory, name, version, license) {
  const key = `${name}@${version}`;
  if (entries.has(key)) return;
  const names = (await readdir(directory)).filter(file => /^(licen[cs]e|copying|copyright|notice|ofl)([.-].*)?$/i.test(file));
  const texts = [];
  for (const file of names.sort()) {
    try { texts.push(`${file}\n${await readFile(path.join(directory, file), 'utf8')}`); } catch { /* directories are not license files */ }
  }
  entries.set(key, `${key}\nDeclared license: ${typeof license === 'string' ? license : JSON.stringify(license) || 'Not specified'}\n${texts.join('\n\n') || 'License text not present at package root; consult upstream manifest.'}`);
}

const store = path.join(root, 'node_modules/.pnpm');
for (const item of await readdir(store, { withFileTypes: true })) {
  if (!item.isDirectory() || item.name === 'node_modules') continue;
  const modules = path.join(store, item.name, 'node_modules');
  let names;
  try { names = await readdir(modules); } catch { continue; }
  for (const name of names) {
    const packages = name.startsWith('@') ? (await readdir(path.join(modules, name))).map(n => `${name}/${n}`) : [name];
    for (const packageName of packages) {
      try {
        const directory = await realpath(path.join(modules, packageName));
        const pkg = JSON.parse(await readFile(path.join(directory, 'package.json'), 'utf8'));
        await add(directory, pkg.name, pkg.version, pkg.license);
      } catch { /* unrelated store entries */ }
    }
  }
}
const metadata = spawnSync('cargo', ['metadata', '--format-version', '1', '--locked'], { cwd: root, encoding: 'utf8', maxBuffer: 20_000_000 });
if (metadata.status !== 0) throw new Error(metadata.stderr || 'Cargo metadata alınamadı.');
for (const pkg of JSON.parse(metadata.stdout).packages) {
  if (pkg.source) await add(path.dirname(pkg.manifest_path), pkg.name, pkg.version, pkg.license);
}
const contents = [...entries].sort(([a], [b]) => a.localeCompare(b, 'en')).map(([, value]) => value).join('\n\n' + '='.repeat(80) + '\n\n');
await writeFile(path.join(resources, 'licenses/DEPENDENCY-LICENSES.txt'), `Aevra dependency license inventory\nIncludes build and platform-specific dependencies.\n\n${contents}\n`);
console.log(`${entries.size} bağımlılığın lisans envanteri hazırlandı.`);
