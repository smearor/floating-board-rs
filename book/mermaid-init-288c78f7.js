// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/* Smearor-branded mermaid theme — the book is dark-only, so the theme
   switcher logic of the stock mermaid-init.js is not needed. */
(() => {
    mermaid.initialize({
        startOnLoad: true,
        theme: 'base',
        themeVariables: {
            background: '#0a0e16',
            primaryColor: '#10151f',
            primaryTextColor: '#eef2f8',
            primaryBorderColor: '#00a1e4',
            secondaryColor: '#161c28',
            tertiaryColor: '#0a0e16',
            lineColor: '#9aa7bd',
            textColor: '#eef2f8',
            edgeLabelBackground: '#10151f',
            fontFamily: '"Source Code Pro", monospace',
            fontSize: '14px'
        }
    });
})();
