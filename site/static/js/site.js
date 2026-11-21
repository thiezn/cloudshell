const themeButton = document.querySelector('#theme-toggle');
themeButton?.addEventListener('click', () => {
    const theme = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    try { localStorage.setItem('theme', theme); } catch (_) { }
});

const menuButton = document.querySelector('#menu-toggle');
const primaryNav = document.querySelector('#primary-nav');
menuButton?.addEventListener('click', () => {
    const isExpanded = menuButton.getAttribute('aria-expanded') === 'true';
    menuButton.setAttribute('aria-expanded', String(!isExpanded));
    primaryNav?.classList.toggle('open', !isExpanded);
});
