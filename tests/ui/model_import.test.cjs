const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const { component, find } = require('./component_fixture.cjs');

function fixture() {
  const calls = [];
  return {
    calls, state: vue.ref({ importedModels: [], settings: { policy: { maxHourlyUsd: 10 } } }),
    snapshot: vue.ref({ catalog: [], runpodKeyConfigured: true }), selectedProfileId: vue.ref(''), view: vue.ref('models'),
    busy: vue.ref(null), modelImportBusy: vue.ref(null), modelImportNotice: vue.ref(''),
    async call(command, args) { calls.push({ command, args }); return { id: 'saved-model', reused: true }; },
    async modelOperation(label, action) { return action(); },
    perform() { throw Error('Unexpected general operation'); }, openExternal() {},
  };
}
test('pasted URL automatically requests analysis and explains reuse without provisioning', async () => {
  const deck = fixture();
  const setup = component('src/features/models/ModelsView.vue', deck, false).setup({}, { expose() {} });
  setup.url.value = ' https://huggingface.co/org/repo ';
  await setup.importModel();
  assert.deepEqual(JSON.parse(JSON.stringify(deck.calls)), [{ command: 'import_hugging_face_model', args: { url: 'https://huggingface.co/org/repo', reanalyze: false } }]);
  assert.match(deck.modelImportNotice.value, /No new Codex analysis/);
  assert.equal(setup.url.value, '');
  deck.modelOperation = async () => undefined;
  setup.url.value = 'https://huggingface.co/org/retry'; await setup.importModel();
  assert.equal(setup.url.value, 'https://huggingface.co/org/retry');
});
function savedModel() {
  return {
    id: 'saved-model', sourceUrl: 'https://huggingface.co/org/repo/tree/main', profile: { model: {} },
    metadata: { repository: 'org/repo', revision: 'a'.repeat(40), access: 'public', weightSizeGb: 15, weightFormat: 'safetensors', parameterCount: 7615616512, license: 'apache-2.0', configJson: JSON.stringify({ architectures: ['Qwen2ForCausalLM'], model_type: 'qwen2', max_position_embeddings: 32768, torch_dtype: 'bfloat16', num_hidden_layers: 28 }), notes: [] },
    analysis: { summary: 'Estimate', runtimeCompatible: true, compatibilityReason: 'Candidate', minimumTotalVramGb: 24, gpuCount: 1, contextMinTokens: 4096, contextMaxTokens: 16384, recommendedContextTokens: 8192, minCpuRamGb: 32, diskGb: 50, minCacheGb: 20, assumptions: [], warnings: [] },
    quotes: [{ gpuType: 'Example GPU', gpuCount: 1, hourlyUsd: 12.5, availableRegions: [] }],
    analyzedAt: 100, pricesCheckedAt: 100, analyzerVersion: 'fixture', runtime: { version: '0.31.0' },
  };
}
function text(node) {
  if (typeof node === 'string') return node;
  if (!node || typeof node !== 'object') return '';
  return Array.isArray(node.children) ? node.children.map(text).join(' ') : text(node.children);
}
test('suggestion shows context range, dated price and over-budget status; use selects the saved profile', () => {
  const deck = fixture(); const model = savedModel();
  const render = component('src/features/models/ModelHardwareSuggestion.vue', deck).setup({ model }, {});
  const tree = render({}, []); const content = text(tree);
  assert.match(content, /4,096–16,384 tokens/); assert.match(content, /default 8,192/);
  assert.match(content, /\$12\.50\/h/); assert.match(content, /Over \$10\.00\/h limit/);
  assert.match(content, /Checked/); assert.match(content, /UNTESTED/);
  const card = component('src/features/models/ModelSuggestion.vue', deck).setup({ model }, {})({}, []);
  assert.equal(find(card, n => n.type.fixtureComponent === 'ModelDetails.vue').props.metadata, model.metadata);
  assert.equal(find(card, n => n.type.fixtureComponent === 'ModelHardwareSuggestion.vue').props.model, model);
  find(card, n => n.type === 'button' && text(n).includes('Use saved suggestion')).props.onClick();
  assert.equal(deck.selectedProfileId.value, model.id); assert.equal(deck.view.value, 'deployments');
});
test('model details show saved facts and published context independently of the AI estimate', () => {
  const metadata = savedModel().metadata;
  const render = component('src/features/models/ModelDetails.vue', fixture()).setup({ metadata }, {});
  const tree = render({}, []); const content = text(tree);
  for (const expected of ['7.62B', '7,615,616,512', '32,768 tokens', 'Published context limit', 'Qwen2ForCausalLM', 'bfloat16', '28', 'apache-2.0', 'safetensors', '15.0 GB', 'Not specified']) assert.ok(content.includes(expected), expected);
  const link = find(tree, n => n.type === 'a');
  assert.equal(link.props.href, `https://huggingface.co/org/repo/blob/${'a'.repeat(40)}/config.json`);
});
test('text config wins for multimodal context; scaling never silently multiplies the published limit', () => {
  const metadata = { ...savedModel().metadata, configJson: JSON.stringify({
    max_position_embeddings: 1024, vision_config: { max_position_embeddings: 256 }, torch_dtype: 'float32',
    text_config: { max_position_embeddings: 131072, dtype: 'bfloat16', rope_scaling: { factor: 4, original_max_position_embeddings: 32768 }, quantization_config: { quant_method: 'awq', bits: 4 }, num_local_experts: 8, num_experts_per_tok: 2 },
  }) };
  const setup = component('src/features/models/ModelDetails.vue', fixture(), false).setup({ metadata }, { expose() {} });
  assert.equal(setup.context.value.value, 131072); assert.equal(setup.context.value.key, 'text_config.max_position_embeddings');
  assert.equal(setup.dtype.value, 'bfloat16'); assert.equal(setup.quantization.value, 'awq · 4-bit');
  assert.equal(setup.structure.value.find(f => f.name === 'Experts').value, '8');
  assert.match(setup.rope.value, /32768/);
});
test('missing, malformed or unsupported metadata stays unknown instead of inventing model facts', () => {
  for (const configJson of ['', 'invalid JSON', '[]', '{"max_position_embeddings":-1}', '{"max_position_embeddings":"128k"}', '{"vision_config":{"max_position_embeddings":512}}', '{"max_position_embeddings":1024,"text_config":{"model_type":"language"}}']) {
    const metadata = { ...savedModel().metadata, repository: 'org/70B-model', parameterCount: null, license: null, configJson };
    const setup = component('src/features/models/ModelDetails.vue', fixture(), false).setup({ metadata }, { expose() {} });
    assert.equal(setup.context.value, undefined); assert.equal(setup.parameters.value, 'Not provided');
  }
});
test('selecting an imported deployment profile shows its saved facts and GPU suggestion', () => {
  const deck = fixture(); const first = savedModel(); const second = { ...first, id: 'other-model' };
  Object.assign(deck, { profile: vue.ref({ model: { id: first.id, gpuTypes: [] } }), hardware: vue.ref(null), activeDeployment: vue.ref(null), error: vue.ref(null), connected: vue.ref(false), clock: vue.ref(100) });
  deck.state.value.importedModels = [first, second];
  const setup = component('src/features/deployments/DeploymentsView.vue', deck, false).setup({}, { expose() {}, emit() {} });
  assert.equal(setup.selectedImported.value.id, first.id);
  deck.profile.value = { model: { id: second.id, gpuTypes: [] } };
  assert.equal(setup.selectedImported.value.id, second.id);
  deck.profile.value = { model: { id: 'bundled', gpuTypes: [] } };
  assert.equal(setup.selectedImported.value, undefined);
});
test('price refresh reuses analysis, while explicit reanalysis requests an update', async () => {
  const deck = fixture(); const model = savedModel();
  const setup = component('src/features/models/ModelSuggestion.vue', deck, false).setup({ model }, { expose() {} });
  await setup.refreshPrices(); await setup.reanalyze();
  assert.deepEqual(JSON.parse(JSON.stringify(deck.calls)), [
    { command: 'refresh_model_prices', args: { id: model.id } },
    { command: 'import_hugging_face_model', args: { url: model.sourceUrl, reanalyze: true } },
  ]);
});
