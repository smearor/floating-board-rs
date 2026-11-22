/* Smearor — Table Top Smart Desk */
(() => {
    'use strict';

    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    /* ---------- nav state ---------- */
    const nav = document.getElementById('nav');
    const onScroll = () => nav.classList.toggle('scrolled', window.scrollY > 24);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });

    /* ---------- reveal on scroll ---------- */
    const revealEls = document.querySelectorAll('.reveal');
    if (!reduced && 'IntersectionObserver' in window) {
        const io = new IntersectionObserver((entries) => {
            for (const e of entries) {
                if (e.isIntersecting) {
                    e.target.classList.add('in');
                    io.unobserve(e.target);
                }
            }
        }, { threshold: 0.12, rootMargin: '0px 0px -6% 0px' });
        revealEls.forEach(el => io.observe(el));
    } else {
        revealEls.forEach(el => el.classList.add('in'));
    }

    /* ---------- stat counters ---------- */
    const stats = document.querySelectorAll('.stat-num');
    if (!reduced && 'IntersectionObserver' in window) {
        const io2 = new IntersectionObserver((entries) => {
            for (const e of entries) {
                if (!e.isIntersecting) continue;
                io2.unobserve(e.target);
                const el = e.target;
                const target = Number(el.dataset.target || 0);
                const prefix = el.dataset.prefix || '';
                const suffix = el.dataset.suffix || '';
                const t0 = performance.now();
                const dur = 1100;
                const tick = (t) => {
                    const p = Math.min(1, (t - t0) / dur);
                    const eased = 1 - Math.pow(1 - p, 3);
                    el.textContent = prefix + Math.round(target * eased) + suffix;
                    if (p < 1) requestAnimationFrame(tick);
                };
                requestAnimationFrame(tick);
            }
        }, { threshold: 0.6 });
        stats.forEach(el => io2.observe(el));
    } else {
        stats.forEach(el => {
            el.textContent = (el.dataset.prefix || '') + el.dataset.target + (el.dataset.suffix || '');
        });
    }

    /* ---------- hero tile field ---------- */
    const canvas = document.getElementById('tilefield');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const COLORS = ['#04e762', '#f5b700', '#00a1e4', '#dc0073', '#89fc00'];

    let w = 0, h = 0, dpr = 1, tiles = [];

    function resize() {
        dpr = Math.min(window.devicePixelRatio || 1, 2);
        w = canvas.clientWidth;
        h = canvas.clientHeight;
        canvas.width = w * dpr;
        canvas.height = h * dpr;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        spawn();
    }

    function spawn() {
        const count = Math.round(Math.min(26, Math.max(10, (w * h) / 60000)));
        tiles = Array.from({ length: count }, () => ({
            x: Math.random() * w,
            y: Math.random() * h,
            s: 14 + Math.random() * 46,
            r: Math.random() * Math.PI * 2,
            vr: (Math.random() - 0.5) * 0.0022,
            vx: (Math.random() - 0.5) * 0.16,
            vy: (Math.random() - 0.5) * 0.12,
            c: COLORS[(Math.random() * COLORS.length) | 0],
            a: 0.04 + Math.random() * 0.1,
            rr: 4 + Math.random() * 6
        }));
    }

    function roundRect(x, y, s, rr) {
        const h2 = s / 2;
        ctx.beginPath();
        ctx.moveTo(x - h2 + rr, y - h2);
        ctx.arcTo(x + h2, y - h2, x + h2, y + h2, rr);
        ctx.arcTo(x + h2, y + h2, x - h2, y + h2, rr);
        ctx.arcTo(x - h2, y + h2, x - h2, y - h2, rr);
        ctx.arcTo(x - h2, y - h2, x + h2, y - h2, rr);
        ctx.closePath();
    }

    function frame() {
        ctx.clearRect(0, 0, w, h);

        /* faint grid — the tabletop surface */
        ctx.strokeStyle = 'rgba(140, 170, 220, 0.05)';
        ctx.lineWidth = 1;
        const gs = 72;
        ctx.beginPath();
        for (let x = 0; x <= w; x += gs) { ctx.moveTo(x, 0); ctx.lineTo(x, h); }
        for (let y = 0; y <= h; y += gs) { ctx.moveTo(0, y); ctx.lineTo(w, y); }
        ctx.stroke();

        /* drifting tangible tiles */
        for (const t of tiles) {
            t.x += t.vx; t.y += t.vy; t.r += t.vr;
            if (t.x < -t.s) t.x = w + t.s;
            if (t.x > w + t.s) t.x = -t.s;
            if (t.y < -t.s) t.y = h + t.s;
            if (t.y > h + t.s) t.y = -t.s;

            ctx.save();
            ctx.translate(t.x, t.y);
            ctx.rotate(t.r);
            ctx.translate(-t.x, -t.y);
            ctx.globalAlpha = t.a;
            ctx.fillStyle = t.c;
            ctx.shadowColor = t.c;
            ctx.shadowBlur = 18;
            roundRect(t.x, t.y, t.s, t.rr);
            ctx.fill();
            ctx.restore();
        }
        requestAnimationFrame(frame);
    }

    resize();
    window.addEventListener('resize', resize);

    if (reduced) {
        /* static frame, no motion */
        ctx.clearRect(0, 0, w, h);
        for (const t of tiles) {
            ctx.save();
            ctx.translate(t.x, t.y);
            ctx.rotate(t.r);
            ctx.translate(-t.x, -t.y);
            ctx.globalAlpha = t.a;
            ctx.fillStyle = t.c;
            roundRect(t.x, t.y, t.s, t.rr);
            ctx.fill();
            ctx.restore();
        }
    } else {
        requestAnimationFrame(frame);
    }

    /* ---------- footer year ---------- */
    const year = document.getElementById('year');
    if (year) year.textContent = new Date().getFullYear();
})();
