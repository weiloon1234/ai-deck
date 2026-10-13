const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const defaults = require('../../src/shared/defaults.json');
const { component, find, text } = require('./component_fixture.cjs');

function fixture() {
  const calls = [];
  const deck = {
    calls, state: vue.ref({ settings: structuredClone(defaults) }),
    snapshot: vue.ref({ runpodKeyConfigured: false }), busy: vue.ref(null), installations: vue.ref([]),
    async call(command, args) {
      calls.push({ command, args });
      if (command === 'save_settings') deck.state.value.settings = structuredClone(args.settings);
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
