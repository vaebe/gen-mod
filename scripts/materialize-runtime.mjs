import { cpSync, existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, relative, parse } from "node:path";

/** 只复制运行时依赖闭包，并实体化模块目录，安装包不依赖开发机 pnpm 链接。 */
export function materialize(deployment, destination) {
  const packageInfo = (path) => JSON.parse(readFileSync(join(path, "package.json"), "utf8"));
  function dependency(parent, name) {
    let current = parent;
    while (true) {
      const candidate = join(current, "node_modules", name);
      if (existsSync(join(candidate, "package.json"))) { return realpathSync(candidate); }
      const next = dirname(current);
      if (next === current || current === parse(current).root) { throw new Error(`运行时缺少依赖 ${name}`); }
      current = next;
    }
  }
  const packages = new Map();
  const graph = new Map();
  function discover(path) {
    if (graph.has(path)) { return; }
    const info = packageInfo(path);
    if (!packages.has(info.name)) { packages.set(info.name, path); }
    const dependencies = new Map();
    graph.set(path, dependencies);
    for (const name of Object.keys({ ...info.dependencies, ...info.optionalDependencies })) {
      let child;
      try { child = dependency(path, name); }
      catch (error) { if (info.optionalDependencies?.[name]) { continue; } throw error; }
      dependencies.set(name, child); discover(child);
    }
  }
  discover(realpathSync(join(deployment, "node_modules/@earendil-works/pi-coding-agent")));
  discover(realpathSync(join(deployment, "node_modules/typebox")));
  let copied = 0;
  function copy(path, target, visible, chain) {
    if (chain.has(path)) { throw new Error("运行时存在不能实体化的版本循环依赖"); }
    const nested = new Set(chain); nested.add(path);
    cpSync(path, target, { recursive: true, dereference: true, filter: (source) => {
      const name = relative(path, source).replaceAll("\\", "/");
      // 分发保留可执行代码和许可证；source map 仅用于调试，不随安装包复制。
      return !(name === "node_modules" || name.startsWith("node_modules/") || name === ".git" || name.startsWith(".git/") || name === ".env" || /\.(?:js|mjs|cjs|ts|mts|cts)\.map$/.test(name));
    } });
    copied += 1;
    const scope = new Map(visible);
    scope.set(packageInfo(path).name, path);
    for (const [name, child] of graph.get(path)) {
      if (scope.get(name) !== child) { copy(child, join(target, "node_modules", name), scope, nested); }
    }
  }
  mkdirSync(join(destination, "node_modules"), { recursive: true });
  for (const [name, path] of packages) { copy(path, join(destination, "node_modules", name), packages, new Set()); }
  writeFileSync(join(destination, "package.json"), JSON.stringify({ name: "gen-mod-pi-runtime", private: true, type: "module" }));
  return copied;
}
