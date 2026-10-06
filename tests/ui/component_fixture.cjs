const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { parse, compileScript } = require('vue/compiler-sfc');
const ts = require('typescript');
const vue = require('vue');


function component(file, deck, inlineTemplate = true) {
  const filename = path.resolve(__dirname, '../../', file);
  const descriptor = parse(fs.readFileSync(filename, 'utf8'), { filename }).descriptor;
  const script = compileScript(descriptor, { id: file, inlineTemplate });
  const code = ts.transpileModule(script.content, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const scope = { exports: {}, require(id) {
    if (id === 'vue') return { ...vue, onMounted() {}, onUnmounted() {} };
    if (id.endsWith('/useDeck')) return { useDeck: () => deck, money: value => `$${value.toFixed(2)}` };
    if (id.endsWith('.vue')) return { default: { fixtureComponent: path.basename(id) } };
    throw Error(`Unexpected dependency: ${id}`);
  } };
  vm.runInNewContext(code, scope, { filename });
  return scope.exports.default;
}
function find(vnode, predicate) {
  if (!vnode || typeof vnode !== 'object') return;
  if (predicate(vnode)) return vnode;
  if (Array.isArray(vnode.children)) {
    for (const child of vnode.children) { const result = find(child, predicate); if (result) return result; }
  }
}

module.exports = { component, find };
