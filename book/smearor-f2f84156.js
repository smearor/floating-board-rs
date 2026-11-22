/* floating-board book — injects the Smearor tile brand into the menu bar
 * and a link back to the documentation landing page. */
(() => {
    'use strict';

    const COLORS = ['t-green', 't-yellow', 't-blue', 't-pink', 't-lime', 't-green', 't-yellow'];

    /* ---------- brand: mini tiles + / floating-board ---------- */
    const title = document.querySelector('#mdbook-menu-bar .menu-title');
    if (title) {
        const tiles = 'SMEAROR'
            .split('')
            .map((ch, i) => `<span class="mini-tile ${COLORS[i]}">${ch}</span>`)
            .join('');
        title.innerHTML =
            `<a class="nav-brand" href="../index.html" title="floating-board">` +
            `<span class="mini-tiles" aria-hidden="true">${tiles}</span>` +
            `<span class="nav-sub">/ floating-board</span>` +
            `</a>` +
            `<span class="menu-title-text">${title.textContent.replace(/^floating-board\s*/i, '')}</span>`;
    }

    /* ---------- home button: back to the landing page ---------- */
    const rightButtons = document.querySelector('#mdbook-menu-bar .right-buttons');
    if (rightButtons) {
        const home = document.createElement('a');
        home.className = 'icon-button';
        home.href = '../index.html';
        home.title = 'floating-board site';
        home.setAttribute('aria-label', 'floating-board site');
        home.innerHTML =
            '<span class="fa-svg"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512" fill="currentColor">' +
            '<path d="M575.8 255.5c0 18-15 32.1-32 32.1h-32l.7 160.2c0 2.7-.2 5.4-.5 8.1V472c0 22.1-17.9 40-40 40H456' +
            'c-1.1 0-2.2 0-3.3-.1c-1.4.1-2.8.1-4.2.1H416 392c-22.1 0-40-17.9-40-40V448 384c0-17.7-14.3-32-32-32H256' +
            'c-17.7 0-32 14.3-32 32v64 24c0 22.1-17.9 40-40 40H160 128.1c-1.5 0-3-.1-4.5-.2c-1.2.1-2.4.2-3.6.2H104' +
            'c-22.1 0-40-17.9-40-40V360c0-.9 0-1.9 .1-2.8V287.6H32c-18 0-32-14-32-32.1c0-9 3-17 10-24L266.4 8' +
            'c7-7 15-8 22-8s15 2 21 7L564.8 231.5c8 7 12 15 11 24z"/></svg></span>';
        rightButtons.prepend(home);
    }
})();
