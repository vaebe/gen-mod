import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

/** lock 是版本维护来源，必须保留的依赖声明及 Tauri 白名单在下载/构建前校验。 */
export function checkRuntimeDeclarations(root) {
  const read = path => JSON.parse(readFileSync(join(root, path), 'utf8'));
  const lock = read('runtime/runtime-lock.json');
  for (const [file, field] of [['package.json', 'devDependencies'], ['runtime/package.json', 'dependencies']]) {
    if (read(file)[field]?.[lock.pi.package] !== lock.pi.version) throw new Error(`${file} 的 pi 版本与 runtime-lock.json 不一致`);
  }
  const resources = read('src-tauri/tauri.conf.json').bundle.resources;
  const expected = {
    [`../runtime/resources/${lock.node.directory}/node.exe`]: `runtime/${lock.node.directory}/node.exe`,
    [`../runtime/resources/${lock.node.directory}/LICENSE`]: `runtime/${lock.node.directory}/LICENSE`,
    [`../runtime/resources/pi-portable-v${lock.pi.version}/`]: `runtime/pi-portable-v${lock.pi.version}/`,
    '../runtime/resources/resource-manifest.json': 'runtime/resource-manifest.json',
  };
  const actual = Object.fromEntries(Object.entries(resources).filter(([key]) => key.startsWith('../runtime/')));
  if (JSON.stringify(Object.entries(actual).sort()) !== JSON.stringify(Object.entries(expected).sort())) throw new Error('Tauri 运行时资源白名单与 runtime-lock.json 不一致');
  return lock;
}

/** 检查真正被三个消费者使用的资源；宿主 Node 不参与内置版本判断。 */
export function checkPreparedRuntime(resources, lock) {
  const node = join(resources, lock.node.directory, 'node.exe');
  const portable = join(resources, `pi-portable-v${lock.pi.version}`);
  for (const path of [node, join(portable, lock.pi.entry), join(portable, 'extensions/mod-tools.ts'), join(portable, 'helpers/read-session.mjs')]) {
    if (!existsSync(path)) throw new Error(`运行时入口缺失：${path}`);
  }
  const pkg = JSON.parse(readFileSync(join(portable, 'node_modules', lock.pi.package, 'package.json'), 'utf8'));
  if (pkg.version !== lock.pi.version) throw new Error('分发 pi 版本与 runtime-lock.json 不一致');
  const result = spawnSync(node, ['--version'], { encoding: 'utf8', windowsHide: true });
  if (result.status !== 0 || result.stdout.trim() !== `v${lock.node.version}`) throw new Error('分发 Node 版本与 runtime-lock.json 不一致');
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const root = fileURLToPath(new URL('../', import.meta.url));
  checkRuntimeDeclarations(root);
  console.log('运行时版本声明一致');
}
