const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const { component, find, text } = require('./component_fixture.cjs');

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
    id: 'saved-model', provider: 'runpod', profileVersion: 1, experimentalConfiguration: null, sourceUrl: 'https://huggingface.co/org/repo/tree/main', profile: { model: {} },
    metadata: { repository: 'org/repo', revision: 'a'.repeat(40), access: 'public', weightSizeGb: 15, weightFormat: 'safetensors', parameterCount: 7615616512, license: 'apache-2.0', configJson: JSON.stringify({ architectures: ['Qwen2ForCausalLM'], model_type: 'qwen2', max_position_embeddings: 32768, torch_dtype: 'bfloat16', num_hidden_layers: 28 }), notes: [] },
    analysis: { summary: 'Estimate', runtimeCompatible: true, compatibilityReason: 'Candidate', minimumTotalVramGb: 24, gpuCount: 1, contextMinTokens: 4096, contextMaxTokens: 16384, recommendedContextTokens: 8192, minCpuRamGb: 32, diskGb: 50, minCacheGb: 20, toolParser: 'hermes', reasoningParser: null, quantization: null, assumptions: [], warnings: [] },
    quotes: [{ gpuType: 'Example GPU', gpuCount: 1, hourlyUsd: 12.5, availableRegions: [] }],
    analyzedAt: 100, pricesCheckedAt: 100, analyzerVersion: 'fixture', runtime: { version: '0.31.0' },
  };
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

const experimentalHardware = { provider: 'runpod', gpus: [{ id: 'Example GPU', displayName: 'Fixture GPU', memoryInGb: 24, secureCloud: true, securePrice: 3 }], dataCenters: [], networkVolumes: [] };
test('an uncertain saved model exposes an inline editor instead of a review queue', () => {
  const deck = fixture(); const model = savedModel(); model.profile = null; model.analysis.runtimeCompatible = false;
  const render = component('src/features/models/ModelSuggestion.vue', deck).setup({ model }, {});
  const tree = render({}, []);
  assert.match(text(tree), /SAVED · NOT CONFIGURED/); assert.doesNotMatch(text(tree), /NEEDS REVIEW/);
  find(tree, n => n.type === 'button' && text(n) === 'Edit configuration').props.onClick();
  assert.equal(find(render({}, []), n => n.type.fixtureComponent === 'ModelConfigurationEditor.vue').props.model, model);
  assert.equal(deck.calls.length, 0);
});
test('experimental editing saves normalized configuration and opens deployment without creating compute', async () => {
  const deck = fixture(); const model = savedModel(); model.profile = null; model.analysis.runtimeCompatible = false;
  deck.call = async (command, args) => { deck.calls.push({ command, args }); if (command === 'imported_model_hardware') return experimentalHardware; };
  const events = [];
  const setup = component('src/features/models/ModelConfigurationEditor.vue', deck, false).setup({ model }, { emit: name => events.push(name), expose() {} });
  await setup.loadHardware();
  assert.ok(setup.hardwareCheckedAt.value > model.pricesCheckedAt); // This read is distinct from the saved snapshot.
  setup.configuration.value.contextTokens = 4096; setup.configuration.value.toolParser = ' hermes '; setup.configuration.value.reasoningParser = '';
  await setup.tryExperimentally();
  assert.equal(deck.calls[0].command, 'imported_model_hardware'); assert.equal(deck.calls[0].args.id, model.id);
  const saved = JSON.parse(JSON.stringify(deck.calls[1]));
  assert.equal(saved.command, 'configure_imported_model'); assert.equal(saved.args.request.expectedProfileVersion, 1);
  assert.equal(saved.args.request.configuration.toolParser, 'hermes'); assert.equal(saved.args.request.configuration.reasoningParser, null);
  assert.equal(saved.args.request.configuration.contextTokens, 4096);
  assert.equal(model.analysis.runtimeCompatible, false); assert.equal(model.analysis.recommendedContextTokens, 8192);
  assert.equal(deck.selectedProfileId.value, model.id); assert.equal(deck.view.value, 'deployments'); assert.deepEqual(events, ['close']);
  assert.match(deck.modelImportNotice.value, /No compute was created/);
  assert.ok(deck.calls.every(c => !['provision', 'enable', 'launch_session'].includes(c.command)));
});
test('experimental editing retains failed drafts and prevents stale or over-budget saves', async () => {
  const deck = fixture(); const model = vue.reactive(savedModel());
  model.experimentalConfiguration = { gpuType: 'Example GPU', gpuCount: 1, contextTokens: 2048, minCpuRamGb: 32, diskGb: 50, minCacheGb: 20, toolParser: null, reasoningParser: null, quantization: null };
  deck.call = async command => command === 'imported_model_hardware' ? experimentalHardware : undefined;
  const setup = component('src/features/models/ModelConfigurationEditor.vue', deck, false).setup({ model }, { emit() { throw Error('Must not close failed editor'); }, expose() {} });
  await setup.loadHardware();
  setup.configuration.value.contextTokens = 4096;
  assert.equal(model.experimentalConfiguration.contextTokens, 2048); // Draft is independent even for reactive props.
  setup.configuration.value.gpuCount = 4;
  assert.equal(setup.rate.value, 12); await setup.tryExperimentally(); assert.equal(deck.view.value, 'models');
  setup.configuration.value.gpuCount = 1;
  deck.modelOperation = async () => undefined;
  await setup.tryExperimentally(); assert.match(setup.saveError.value, /not saved/); assert.equal(setup.configuration.value.contextTokens, 4096);
  model.profileVersion = 2;
  assert.equal(setup.changed.value, true); await setup.tryExperimentally(); assert.equal(deck.view.value, 'models');
});
test('missing protected config explains the secret setup without blocking an explicit trial', async () => {
  const deck = fixture(); const model = savedModel(); model.profile = null;
  model.metadata.configJson = ''; model.metadata.access = 'gated'; model.analysis.runtimeCompatible = false;
  const render = component('src/features/models/ModelConfigurationEditor.vue', deck).setup({ model }, { emit() {} });
  const tree = render({}, []);
  assert.match(text(tree), /architecture and published context remain unknown/);
  assert.match(text(tree), /Runpod Secrets/); assert.match(text(tree), /only its secret name in Setup/);
  deck.call = async (command, args) => { deck.calls.push({ command, args }); return command === 'imported_model_hardware' ? experimentalHardware : undefined; };
  const setup = component('src/features/models/ModelConfigurationEditor.vue', deck, false).setup({ model }, { emit() {}, expose() {} });
  await setup.loadHardware(); await setup.tryExperimentally();
  assert.deepEqual(deck.calls.map(c => c.command), ['imported_model_hardware', 'configure_imported_model']);
  assert.equal(deck.view.value, 'deployments'); assert.equal(model.metadata.configJson, '');
});
test('experimental editing uses saved provider options and directs a different selected provider to setup', async () => {
  const deck = fixture(); deck.state.value.settings.provider = 'gputw'; const model = savedModel();
  deck.call = async command => command === 'imported_model_hardware' ? experimentalHardware : undefined;
  const setup = component('src/features/models/ModelConfigurationEditor.vue', deck, false).setup({ model }, { emit() {}, expose() {} });
  await setup.loadHardware(); await setup.tryExperimentally();
  assert.equal(deck.state.value.settings.provider, 'gputw'); assert.equal(deck.view.value, 'setup'); assert.equal(deck.selectedProfileId.value, model.id);
});
test('a configuration save completing after navigation does not redirect the owner', async () => {
  const deck = fixture(); const model = savedModel(); let complete;
  deck.call = async command => command === 'imported_model_hardware' ? experimentalHardware : new Promise(resolve => { complete = resolve; });
  const setup = component('src/features/models/ModelConfigurationEditor.vue', deck, false).setup({ model }, { emit() {}, expose() {} });
  await setup.loadHardware(); const save = setup.tryExperimentally();
  deck.view.value = 'projects'; complete(); await save;
  assert.equal(deck.view.value, 'projects'); assert.equal(deck.selectedProfileId.value, '');
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
test('library total stays stable while searching and excludes imported catalog duplicates', () => {
  const deck = fixture(); const model = savedModel();
  deck.state.value.importedModels = [model];
  deck.snapshot.value.catalog = [
    { model: { id: model.id, displayName: 'Imported', repository: 'org/repo' } },
    { model: { id: 'bundled', displayName: 'Bundled model', repository: 'publisher/bundled' } },
  ];
  const setup = component('src/features/models/ModelsView.vue', deck, false).setup({}, { expose() {} });
  assert.equal(setup.libraryCount.value, 2);
  setup.search.value = 'publisher';
  assert.equal(setup.imported.value.length, 0);
  assert.equal(setup.profiles.value.length, 1);
  assert.equal(setup.libraryCount.value, 2);
});

test('empty model search can be cleared to restore matching models', () => {
  const deck = fixture();
  deck.state.value.importedModels = [savedModel()];
  const render = component('src/features/models/ModelsView.vue', deck).setup({}, {});
  let tree = render({}, []);
  find(tree, n => n.type === 'input' && n.props?.type === 'search').props['onUpdate:modelValue']('no-matching-model');
  tree = render({}, []);
  assert.equal(find(tree, n => n.type?.fixtureComponent === 'ModelSuggestion.vue'), undefined);
  find(tree, n => n.type === 'button' && n.children === 'Clear search').props.onClick();
  tree = render({}, []);
  assert.ok(find(tree, n => n.type?.fixtureComponent === 'ModelSuggestion.vue'));
});
test('deployment telemetry appears only for an active Pod while creation controls retain their gates', () => {
  const deck = fixture();
  Object.assign(deck, {
    profile: vue.ref(null), hardware: vue.ref(null), activeDeployment: vue.ref(null),
    error: vue.ref(null), connected: vue.ref(false), clock: vue.ref(100),
  });
  Object.assign(deck.state.value, { sessions: [], deployments: [] });
  const render = component('src/features/deployments/DeploymentsView.vue', deck).setup({}, {});
  let tree = render({}, []);
  assert.equal(find(tree, n => n.props?.class === 'stats-grid'), undefined);
  const provision = find(tree, n => n.type === 'button' && text(n).includes('Provision Runpod'));
  assert.equal(provision.props.disabled, true);
  deck.activeDeployment.value = {
    id: 'fixture-deployment', stage: 'ready', createdAt: 40, deadlineAt: 500, hourlyUsd: 2,
    profile: { model: { displayName: 'Fixture model', servedModel: 'fixture', maxSessions: 1 } },
    podIds: [], message: 'Fixture only',
  };
  tree = render({}, []);
  assert.ok(find(tree, n => n.props?.class === 'stats-grid'));
  assert.equal(find(tree, n => n.type === 'button' && n.children === 'Open CLI').props.disabled, true);
  let workflow = find(tree, n => n.props?.class === 'workflow');
  assert.ok(find(workflow, n => n.type === 'div' && n.props?.class === 'current' && text(n).includes('Enable endpoint')));
  deck.connected.value = true;
  tree = render({}, []);
  workflow = find(tree, n => n.props?.class === 'workflow');
  assert.ok(find(workflow, n => n.type === 'div' && n.props?.class === 'current' && text(n).includes('Open your CLI')));
});
