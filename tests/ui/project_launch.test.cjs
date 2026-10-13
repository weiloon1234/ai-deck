// Exercise actual Vue component setup/template handlers with an in-memory deck.
// No DOM, desktop commands, credentials or cloud access are available here.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const defaults = require('../../src/shared/defaults.json');
const { component, find, text } = require('./component_fixture.cjs');

function fixture() {
  return {
    state: vue.ref({ settings: structuredClone(defaults), projects: [{ id: 'a', name: 'A', path: '/fixture/a' }, { id: 'b', name: 'B', path: '/fixture/b' }], sessions: [] }),
    view: vue.ref('projects'), busy: vue.ref(null), error: vue.ref(null), connected: vue.ref(false),
    enabledDeployment: vue.ref(null), runningSessions: vue.ref([]), quitRequested: vue.ref(false),
    activeSessionId: vue.ref(null), snapshot: vue.ref(null), native: false,
    call() { throw Error('Native calls are forbidden'); }, perform() { throw Error('Actions are forbidden'); },
  };
}
test('project row carries its selected folder through App into the launch dialog', () => {
  const deck = fixture();
  const renderApp = component('src/App.vue', deck).setup({}, {});
  let tree = renderApp({}, []);
  const projectsView = find(tree, n => n.type?.fixtureComponent === 'ProjectsView.vue');
  assert.ok(projectsView);
  const renderProjects = component('src/features/projects/ProjectsView.vue', deck).setup({}, {});
  const projects = renderProjects({ $emit: (name, id) => { assert.equal(name, 'launch'); projectsView.props.onLaunch(id); } }, []);
  const rows = [];
  (function collect(v) { if (!v || typeof v !== 'object') return; if (v.type === 'article') rows.push(v); if (Array.isArray(v.children)) v.children.forEach(collect); })(projects);
  find(rows[1], n => n.type === 'button' && n.children === 'Open CLI').props.onClick();
  tree = renderApp({}, []);
  const dialog = find(tree, n => n.type?.fixtureComponent === 'LaunchSessionDialog.vue');
  // Vue normalizes a template's kebab-case prop when mounting the component.
  assert.equal(dialog.props['project-id'], 'b');
  const setup = component('src/features/sessions/LaunchSessionDialog.vue', deck, false).setup({ projectId: dialog.props['project-id'] }, { expose() {}, emit() {} });
  assert.equal(setup.projectId.value, 'b');
});
test('generic new session uses its default instead of the previous row selection', () => {
  const deck = fixture();
  const app = component('src/App.vue', deck, false).setup({}, { expose() {} });
  app.openSession('b'); assert.equal(app.launchProjectId.value, 'b');
  app.openSession(); assert.equal(app.launchProjectId.value, null);
  const dialog = component('src/features/sessions/LaunchSessionDialog.vue', deck, false).setup({ projectId: null }, { expose() {}, emit() {} });
  assert.equal(dialog.projectId.value, 'a');
});
test('sidebar keeps open sessions visible and exposes ended history on demand', () => {
  const deck = fixture();
  deck.state.value.sessions = [
    { id: 'ended', name: 'Earlier session', status: 'ended', cli: 'codex', model: 'fixture' },
    { id: 'open', name: 'Current session', status: 'running', cli: 'codex', model: 'fixture' },
  ];
  deck.selectSession = id => { deck.activeSessionId.value = id; deck.view.value = 'sessions'; };
  const render = component('src/App.vue', deck).setup({}, {});
  let tree = render({}, []);
  const session = name => n => n.type === 'button' && text(n).includes(name);
  assert.ok(find(tree, session('Current session')));
  assert.equal(find(tree, session('Earlier session')), undefined);
  const history = find(tree, n => n.props?.class === 'session-history-toggle');
  assert.equal(history.props['aria-expanded'], false);
  history.props.onClick();
  tree = render({}, []);
  assert.equal(find(tree, n => n.props?.class === 'session-history-toggle').props['aria-expanded'], true);
  find(tree, session('Earlier session')).props.onClick();
  assert.equal(deck.activeSessionId.value, 'ended');
  assert.equal(deck.view.value, 'sessions');
});
test('switching screens returns to the top of the workspace', async () => {
  const deck = fixture(); const scrolls = [];
  const setup = component('src/App.vue', deck, false).setup({}, { expose() {} });
  setup.workspaceElement.value = { scrollIntoView: options => scrolls.push(options.block) };
  deck.view.value = 'setup';
  await vue.nextTick();
  assert.deepEqual(scrolls, ['start']);
});

test('disconnected sessions offer a route to enable an endpoint before opening a CLI', () => {
  const deck = fixture();
  deck.activeSession = vue.ref(null);
  const render = component('src/features/sessions/SessionsView.vue', deck).setup({}, {});
  let tree = render({}, []);
  find(tree, n => n.type === 'button' && text(n).includes('Go to Deployment')).props.onClick();
  assert.equal(deck.view.value, 'deployments');
  deck.connected.value = true;
  let launches = 0;
  tree = render({ $emit: name => { assert.equal(name, 'launch'); launches++; } }, []);
  assert.equal(find(tree, n => n.type === 'button' && text(n).includes('Go to Deployment')), undefined);
  find(tree, n => n.type === 'button' && n.children === 'Open a coding session').props.onClick({});
  assert.equal(launches, 1);
});

test('launch dialog closes when routing a disconnected user to Deployment', () => {
  const deck = fixture(); let closed = false;
  const render = component('src/features/sessions/LaunchSessionDialog.vue', deck).setup({}, { emit: name => { closed = name === 'close'; } });
  const tree = render({}, []);
  // Modal content is a slot in the compiler-based fixture.
  const content = tree.children.default();
  find({ children: content }, n => n.type === 'button' && n.children === 'Go to Deployment').props.onClick();
  assert.equal(closed, true);
  assert.equal(deck.view.value, 'deployments');
});

test('launch and resume use their bindings without candidate acknowledgements', async () => {
  const deck = fixture(); const calls = []; let selected;
  deck.connected.value = true;
  deck.perform = async (_label, action) => action();
  deck.call = async (command, args) => { calls.push({ command, args }); return 'fixture-session'; };
  deck.selectSession = id => { selected = id; };
  const launch = component('src/features/sessions/LaunchSessionDialog.vue', deck, false).setup({ projectId: 'b' }, { expose() {}, emit() {} });
  await launch.launch();
  assert.equal(selected, 'fixture-session');
  assert.deepEqual(JSON.parse(JSON.stringify(calls[0])), { command: 'launch_session', args: { request: { projectId: 'b', cli: 'codex', name: 'Coding session' } } });
  const dialog = component('src/features/sessions/LaunchSessionDialog.vue', deck).setup({ projectId: 'b' }, { emit() {} })({}, []);
  assert.equal(find({ children: dialog.children.default() }, n => n.type === 'input' && n.props?.type === 'checkbox'), undefined);
  deck.activeSession = vue.ref({ id: 'original-session', status: 'ended', deploymentId: 'original-deployment', model: 'fixture', unverifiedCombination: true });
  const session = component('src/features/sessions/SessionsView.vue', deck, false).setup({}, { expose() {}, emit() {} });
  session.resumeOpen.value = true; await session.resume();
  assert.deepEqual(JSON.parse(JSON.stringify(calls[1])), { command: 'resume_session', args: { id: 'original-session' } });
  assert.equal(session.resumeOpen.value, false);
  const content = component('src/features/sessions/SessionsView.vue', deck).setup({}, { emit() {} })({}, []);
  assert.match(text(content), /original-deployment/); assert.doesNotMatch(text(content), /Candidate test|Validated combination/);
});
