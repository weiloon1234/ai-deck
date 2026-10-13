const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const defaults = require('../../src/shared/defaults.json');
const providers = require('../../src/shared/providers.json');
const { component, find, text } = require('./component_fixture.cjs');

function fixture() {
  return {
    state: vue.ref({ settings: structuredClone(defaults), importedModels: [], deployments: [], sessions: [] }),
    snapshot: vue.ref({ providers: providers.map(info => ({ info, keyConfigured: true })), catalog: [], runpodKeyConfigured: true }),
    profile: vue.ref({ model: { id: 'fixture', gpuTypes: ['fixture-gpu'], gpuCount: 1, minCacheGb: 20 } }),
    selectedProfileId: vue.ref('fixture'), activeDeployment: vue.ref(null),
    hardware: vue.ref({ provider: 'runpod', gpus: [{ id: 'fixture-gpu', securePrice: 3 }], dataCenters: [], networkVolumes: [] }),
    busy: vue.ref(null), error: vue.ref(null), connected: vue.ref(false), clock: vue.ref(100), view: vue.ref('deployments'),
    async perform(_label, action) { return action(); }, async call() {},
  };
}

test('changing provider clears quotes, selections and the open creation confirmation', () => {
  const deck = fixture();
  const setup = component('src/features/deployments/DeploymentsView.vue', deck, false).setup({}, { expose() {}, emit() {} });
  setup.dataCenter.value = 'fixture-dc'; setup.volume.value = 'fixture-volume'; setup.provisionConfirm.value = true;
  assert.equal(setup.rate.value, 3);
  deck.state.value.settings.provider = 'gputw';
  assert.equal(deck.hardware.value, null);
  assert.equal(setup.rate.value, 0);
  assert.equal(setup.dataCenter.value, ''); assert.equal(setup.volume.value, '');
  assert.equal(setup.provisionConfirm.value, false);
});

test('GPUtw selection shows the provisioning gate instead of Runpod creation controls', () => {
  const deck = fixture(); deck.state.value.settings.provider = 'gputw';
  const tree = component('src/features/deployments/DeploymentsView.vue', deck).setup({}, { emit() {} })({}, []);
  assert.match(text(tree), /GPUtw.ai provisioning is pending/);
  assert.equal(find(tree, n => n.type === 'button' && text(n).includes('Provision Runpod')), undefined);
  assert.equal(find(tree, n => n.type === 'select'), undefined);
});

test('late hardware from a previous provider is discarded', async () => {
  const deck = fixture(); let respond;
  deck.call = async (command, args) => {
    assert.equal(command, 'hardware'); assert.equal(args.provider, 'runpod');
    return new Promise(resolve => { respond = resolve; });
  };
  const setup = component('src/features/deployments/DeploymentsView.vue', deck, false).setup({}, { expose() {}, emit() {} });
  const pending = setup.refreshHardware();
  deck.state.value.settings.provider = 'gputw';
  respond({ provider: 'runpod', gpus: [], dataCenters: [], networkVolumes: [] });
  await pending;
  assert.equal(deck.hardware.value, null);
});
