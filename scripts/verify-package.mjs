// Read-only verification of the current portable app and offline installer.
import {readFile,readdir,stat} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawn,spawnSync} from 'node:child_process';
import path from 'node:path';
import {root,resources,specification,digest,verifyAssets} from './assets.mjs';

const lock=await verifyAssets();
const payload=path.resolve(process.env.AEVRA_PAYLOAD_DIR||path.join(root,'artifacts/Aevra-memory'));
const packaged=path.join(payload,'resources');
if(await digest(path.join(packaged,'asset-lock.json'))!==await digest(path.join(resources,'asset-lock.json')))throw new Error('Uygulama varlık kilidi kaynakla eşleşmiyor.');
for(const file of [...lock.files,lock.model,lock.projector]){
  const absolute=path.resolve(packaged,file.path);
  if(!absolute.startsWith(packaged+path.sep))throw new Error('Geçersiz paket yolu.');
  if((await stat(absolute)).size!==file.size||await digest(absolute)!==file.sha256)throw new Error(`Paket dosyası doğrulanamadı: ${file.path}`);
}
const models=(await readdir(path.join(packaged,'models'))).filter(name=>name.endsWith('.gguf')).sort();
const expectedModels=[specification.model.file,specification.projector.file].sort();
if(JSON.stringify(models)!==JSON.stringify(expectedModels))throw new Error('Paket eski veya fazla model içeriyor.');
const version=JSON.parse(await readFile(path.join(root,'package.json'),'utf8')).version;
const prefix=`Aevra-${version}-windows-x64-setup`;
const archive=path.join(root,`artifacts/Aevra-${version}-windows-x64-offline.zip`);
const archiveHash=await digest(archive);
const checksum=await readFile(`${archive}.sha256`,'utf8');
if(checksum.split(/\s+/)[0]!==archiveHash)throw new Error('ZIP özeti eşleşmiyor.');
function tar(args){const result=spawnSync('tar.exe',args,{encoding:'utf8',maxBuffer:64*1024});if(result.status!==0)throw new Error(result.stderr||'ZIP okunamadı.');return result.stdout;}
const entries=tar(['-tf',archive]).trim().split(/\r?\n/);
const checks=tar(['-xOf',archive,'SHA256SUMS.txt']).trim().split(/\r?\n/).map(line=>{
  const match=/^([a-f0-9]{64})  ([\w.-]+)$/.exec(line);
  if(!match||!(match[2]===`${prefix}.exe`||new RegExp(`^${prefix.replaceAll('.','\\.')}-[1-9][0-9]*\\.bin$`).test(match[2])))throw new Error('Kurulum özeti geçersiz.');
  return {sha256:match[1],name:match[2]};
});
if(new Set(entries).size!==entries.length||entries.length!==checks.length+1||!checks.some(c=>c.name.endsWith('.exe'))||!checks.some(c=>c.name.endsWith('.bin'))||checks.some(c=>!entries.includes(c.name)))throw new Error('ZIP içeriği beklenen kurulumla eşleşmiyor.');
for(const file of checks){
  const hash=createHash('sha256');const child=spawn('tar.exe',['-xOf',archive,file.name],{stdio:['ignore','pipe','pipe'],windowsHide:true});
  let stderr='';child.stderr.on('data',chunk=>{stderr+=chunk.toString().slice(0,1000);});
  const done=new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',code=>code===0?resolve():reject(new Error(stderr||'Kurulum dilimi okunamadı.')));});
  await Promise.all([(async()=>{for await(const chunk of child.stdout)hash.update(chunk);})(),done]);
  if(hash.digest('hex')!==file.sha256)throw new Error(`ZIP içeriği bozuk: ${file.name}`);
}
console.log(JSON.stringify({payload,model:specification.model.name,verifiedAssets:lock.files.length+2,exeSha256:await digest(path.join(payload,'Aevra.exe')),archiveBytes:(await stat(archive)).size,archiveSha256:archiveHash,installerFiles:checks.length},null,2));
