const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const defaults = require('../../src/shared/defaults.json');
const providers = require('../../src/shared/providers.json');
const { component, find, text } = require('./component_fixture.cjs');

function fixture() {
  const calls = [];
  const deck = {
    calls, state: vue.ref({ settings: structuredClone(defaults) }),
    snapshot: vue.ref({ runpodKeyConfigured: false, providers: providers.map(info => ({ info, keyConfigured: false, credentialStoreError: null })) }), busy: vue.ref(null), installations: vue.ref([]),
    async call(command, args) {
      calls.push({ command, args });
      if (command === 'save_settings') deck.state.value.settings = structuredClone(args.settings);
      if (command === 'select_provider') deck.state.value.settings.provider = args.provider;
    },
    async perform(_label, action) { return action(); },
  };
  return deck;
}

test('setup tracks unsaved edits, saves normalized settings and clears stale success feedback', async () => {
  const deck = fixture();
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  assert.equal(setup.dirty.value, false);
  setup.form.value.codexPath = ' /fixture/codex ';
  assert.equal(setup.dirty.value, true);
  await setup.save();
  assert.equal(deck.calls.length, 1);
  assert.equal(deck.calls[0].command, 'save_settings');
  assert.equal(deck.calls[0].args.settings.codexPath, '/fixture/codex');
  assert.equal(setup.form.value.codexPath, '/fixture/codex');
  assert.equal(setup.dirty.value, false);
  assert.equal(setup.saved.value, true);
  setup.form.value.claudePath = '/fixture/claude';
  assert.equal(setup.saved.value, false);
  assert.equal(setup.dirty.value, true);
});

test('spending setup retains hourly and budget controls without a lifetime field', async () => {
  const deck = fixture();
  deck.state.value.settings.policy.maxLifetimeMinutes = 1; // Retained legacy data is hidden.
  const render = component('src/features/setup/SetupView.vue', deck).setup({}, {});
  const tree = render({}, []);
  assert.match(text(tree), /Maximum GPU rate|Total experiment budget/);
  assert.doesNotMatch(text(tree), /Maximum deployment lifetime/);
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  setup.totalBudget.value = 30; await setup.save();
  assert.equal(deck.calls[0].args.settings.policy.totalBudgetUsd, 30);
  assert.equal(deck.calls[0].args.settings.policy.maxHourlyUsd, defaults.policy.maxHourlyUsd);
  assert.equal(deck.calls[0].args.settings.policy.maxLifetimeMinutes, 1);
});

test('provider selection is saved immediately, clears key draft and preserves other unsaved settings', async () => {
  const deck = fixture();
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  setup.key.value = 'fixture-runpod-key';
  setup.form.value.codexPath = '/fixture/unsaved-codex';
  const select = { value: 'gputw' };
  await setup.selectProvider({ target: select });
  await vue.nextTick();
  assert.equal(deck.calls[0].command, 'select_provider');
  assert.equal(deck.calls[0].args.provider, 'gputw');
  assert.equal(setup.key.value, '');
  assert.equal(setup.form.value.provider, 'gputw');
  assert.equal(setup.form.value.codexPath, '/fixture/unsaved-codex');
  assert.equal(setup.dirty.value, true);
  setup.key.value = 'fixture-gputw-key';
  await setup.saveKey();
  assert.equal(deck.calls[1].command, 'save_provider_key');
  assert.equal(deck.calls[1].args.provider, 'gputw');
  assert.equal(setup.key.value, '');
});

test('GPUtw setup shows its key and read-only readiness limits, and read checks report only read access', async () => {
  const deck = fixture();
  deck.state.value.settings.provider = 'gputw';
  deck.snapshot.value.providers[1].keyConfigured = true;
  const render = component('src/features/setup/SetupView.vue', deck).setup({}, {});
  const tree = render({}, []);
  assert.match(text(tree), /GPUtw.ai API key/);
  assert.match(text(tree), /Provisioning is pending/);
  assert.match(text(tree), /lost creation response/);
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  await setup.checkConnection();
  assert.equal(deck.calls[0].command, 'check_provider_connection');
  assert.equal(deck.calls[0].args.provider, 'gputw');
  assert.match(setup.connectionNotice.value, /instances:read/);
  assert.match(setup.connectionNotice.value, /Provisioning is still unavailable/);
  await setup.removeKey();
  assert.equal(deck.calls[1].command, 'remove_provider_key');
  assert.equal(deck.calls[1].args.provider, 'gputw');
  assert.equal(setup.connectionNotice.value, '');
});

test('failed provider selection restores the control and failed key/check operations never claim success', async () => {
  const deck = fixture();
  deck.call = async () => { throw Error('Fixture failure'); };
  deck.perform = async (_label, action) => { try { return await action(); } catch {} };
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  const select = { value: 'gputw' };
  await setup.selectProvider({ target: select });
  assert.equal(select.value, 'runpod');
  setup.key.value = 'fixture-key';
  await setup.saveKey(); await setup.checkConnection();
  assert.equal(setup.key.value, '');
  assert.equal(setup.keyNotice.value, '');
  assert.equal(setup.connectionNotice.value, '');
});

test('a failed settings save retains the draft and does not claim success', async () => {
  const deck = fixture();
  deck.call = async () => { throw Error('Fixture write failure'); };
  deck.perform = async (_label, action) => { try { return await action(); } catch {} };
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  setup.form.value.codexPath = '/fixture/retry';
  await setup.save();
  assert.equal(setup.form.value.codexPath, '/fixture/retry');
  assert.equal(setup.dirty.value, true);
  assert.equal(setup.saved.value, false);
  assert.equal(deck.state.value.settings.codexPath, defaults.codexPath);
});

test('edits made during a pending save survive its completion', async () => {
  const deck = fixture(); let completeSave;
  deck.call = async (_command, { settings }) => {
    await new Promise(resolve => { completeSave = resolve; });
    deck.state.value.settings = structuredClone(settings);
  };
  const setup = component('src/features/setup/SetupView.vue', deck, false).setup({}, { expose() {} });
  setup.form.value.codexPath = '/fixture/submitted';
  const saving = setup.save();
  setup.form.value.codexPath = '/fixture/newer-edit';
  completeSave(); await saving;
  assert.equal(deck.state.value.settings.codexPath, '/fixture/submitted');
  assert.equal(setup.form.value.codexPath, '/fixture/newer-edit');
  assert.equal(setup.dirty.value, true);
  assert.equal(setup.saved.value, false);
});

test('settings can only be saved after a change and announce current draft status', () => {
  const deck = fixture();
  const render = component('src/features/setup/SetupView.vue', deck).setup({}, {});
  let tree = render({}, []);
  const button = node => node.type === 'button' && node.children === 'Save settings';
  assert.equal(find(tree, button).props.disabled, true);
  assert.match(text(tree), /No unsaved changes/);
  find(tree, node => node.type === 'input' && node.props?.placeholder === 'Auto-detect').props['onUpdate:modelValue']('/fixture/codex');
  tree = render({}, []);
  assert.equal(find(tree, button).props.disabled, false);
  assert.match(text(tree), /Unsaved changes/);
});
