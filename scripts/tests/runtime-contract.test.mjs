import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { mkdtempSync, cpSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkRuntimeDeclarations } from '../runtime-contract.mjs';
const source = fileURLToPath(new URL('../../', import.meta.url));
test('运行时声明拒绝版本漂移及遗漏分发资源', () => {
  const root = mkdtempSync(join(tmpdir(), 'genmod-runtime-contract-'));
  for (const file of ['package.json', 'runtime/package.json', 'runtime/runtime-lock.json', 'src-tauri/tauri.conf.json']) cpSync(join(source,file), join(root,file), {recursive:true});
  assert.doesNotThrow(() => checkRuntimeDeclarations(root));
  const path = join(root,'runtime/package.json');
  const pkg = JSON.parse(readFileSync(path,'utf8')); pkg.dependencies['@earendil-works/pi-coding-agent']='0.0.0';
  writeFileSync(path, JSON.stringify(pkg));
  assert.throws(() => checkRuntimeDeclarations(root), /runtime\/package.json/);
});
import { copyRuntimeAssets } from '../copy-runtime-assets.mjs';
import { mkdirSync, existsSync, statSync } from 'node:fs';
test('干净准备与重复准备覆盖更新，精确退役且保留其他工作资料', () => {
 const root=mkdtempSync(join(tmpdir(),'genmod-assets-')); const portable=join(root,'portable');
 mkdirSync(join(root,'runtime/extensions'),{recursive:true}); mkdirSync(join(root,'runtime/helpers'),{recursive:true});
 writeFileSync(join(root,'runtime/extensions/mod-tools.ts'),'first'); writeFileSync(join(root,'runtime/helpers/read-session.mjs'),'history');
 copyRuntimeAssets(root,portable);
 const entry=join(portable,'extensions/mod-tools.ts'); const before=statSync(entry).mtimeMs;
 copyRuntimeAssets(root,portable); assert.equal(statSync(entry).mtimeMs,before);
 writeFileSync(join(portable,'extensions/assets.ts'),'retired'); writeFileSync(join(portable,'helpers/process-image.ps1'),'retired'); writeFileSync(join(portable,'keep.txt'),'user material');
 writeFileSync(join(root,'runtime/extensions/mod-tools.ts'),'updated'); copyRuntimeAssets(root,portable);
 assert.equal(readFileSync(entry,'utf8'),'updated'); assert.equal(existsSync(join(portable,'extensions/assets.ts')),false); assert.equal(existsSync(join(portable,'helpers/process-image.ps1')),false); assert.equal(readFileSync(join(portable,'keep.txt'),'utf8'),'user material');
});
test('资源白名单缺入口或仍有旧版本时拒绝构建', () => {
 const root=mkdtempSync(join(tmpdir(),'genmod-bundle-'));
 for(const file of ['package.json','runtime/package.json','runtime/runtime-lock.json','src-tauri/tauri.conf.json']) cpSync(join(source,file),join(root,file),{recursive:true});
 const file=join(root,'src-tauri/tauri.conf.json'); const config=JSON.parse(readFileSync(file,'utf8'));
 delete config.bundle.resources['../runtime/resources/pi-portable-v1.5.0/']; writeFileSync(file,JSON.stringify(config));
 assert.throws(()=>checkRuntimeDeclarations(root),/白名单/);
});
import { checkPreparedRuntime } from '../runtime-contract.mjs';
test('准备结果缺入口或部署 pi 版本错误时拒绝构建', () => {
 const root=mkdtempSync(join(tmpdir(),'genmod-prepared-')); const lock=checkRuntimeDeclarations(source);
 assert.throws(()=>checkPreparedRuntime(root,lock),/入口缺失/);
 const portable=join(root,`pi-portable-v${lock.pi.version}`);
 const paths=[join(root,lock.node.directory,'node.exe'),join(portable,lock.pi.entry),join(portable,'extensions/mod-tools.ts'),join(portable,'helpers/read-session.mjs')];
 for(const path of paths){mkdirSync(dirname(path),{recursive:true});writeFileSync(path,'fixture');}
 const packageFile=join(portable,'node_modules',lock.pi.package,'package.json'); writeFileSync(packageFile,JSON.stringify({version:'0.0.0'}));
 assert.throws(()=>checkPreparedRuntime(root,lock),/分发 pi 版本/);
});
