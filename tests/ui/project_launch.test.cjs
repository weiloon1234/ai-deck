// Exercise actual Vue component setup/template handlers with an in-memory deck.
// No DOM, desktop commands, credentials or cloud access are available here.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const vue = require('vue');
const { component, find } = require('./component_fixture.cjs');

function fixture() {
  return {
    state: vue.ref({ projects: [{ id: 'a', name: 'A', path: '/fixture/a' }, { id: 'b', name: 'B', path: '/fixture/b' }], sessions: [] }),
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
