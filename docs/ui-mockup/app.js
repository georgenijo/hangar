// Multi-view switcher + command palette

const views = document.querySelectorAll('.view');
const links = document.querySelectorAll('.nav-link');
const crumb = document.getElementById('crumb-view');

function showView(name) {
  views.forEach(v => v.hidden = v.id !== `view-${name}`);
  links.forEach(l => l.classList.toggle('active', l.dataset.view === name));
  if (crumb) crumb.textContent = name.replace(/-/g, ' ');
  window.scrollTo({ top: 0, behavior: 'instant' });
}

function routeFromHash() {
  const hash = location.hash.replace('#', '') || 'command';
  // map session-detail etc directly; fallback to command
  const names = ['command','sessions','session-detail','pipeline','conductor','fleet','search','costs','voice','settings'];
  showView(names.includes(hash) ? hash : 'command');
}

links.forEach(l => {
  l.addEventListener('click', e => {
    // hashchange handles the rest
  });
});

window.addEventListener('hashchange', routeFromHash);
window.addEventListener('DOMContentLoaded', routeFromHash);

