<script lang="ts">
    import { untrack } from 'svelte';
    import { TextLayer } from 'pdfjs-dist';
    import { pdfDoc } from '../stores.js';
    import { getPage, renderPageToCanvas, searchAllPages } from '../pdf.js';
    import type { TextMatch } from '../pdf.js';
    import Icon from './Icon.svelte';
    import type { ViewerState } from '../types.js';
    import type { Writable } from 'svelte/store';

    interface Props {
        viewerStore: Writable<ViewerState>;
        label: string;
    }
    let { viewerStore, label }: Props = $props();

    const GAP = 12;
    const PAD = 16;
    const FIT_MARGIN = 16;
    const MIN_ZOOM = 0.1;
    const MAX_ZOOM = 6.0;

    let container = $state<HTMLElement>();
    let visiblePages = $state(new Set<number>());
    let pageEls: HTMLElement[] = [];
    let canvasEls: HTMLCanvasElement[] = [];
    let textLayerEls: (HTMLElement | undefined)[] = [];
    let renderedAt = new Map<number, number>();

    // Per-rendered-page text data used for search highlighting.
    interface PageLayer {
        divs: HTMLElement[];
        itemsStr: string[];
        offsets: number[];
        fullText: string;
    }
    let textLayersByPage = new Map<number, PageLayer>();

    // Search
    let searchQuery = $state('');
    let searchMatches = $state<TextMatch[]>([]);
    let searchMatchIdx = $state(-1);
    let searching = $state(false);
    let searchDebounce: ReturnType<typeof setTimeout> | null = null;

    // Mirrored store state
    let mode = $state<'fit' | 'manual'>('fit');
    let manualZoom = $state(1.0);
    let targetPage = $state(0);

    let containerWidth = $state(0);
    let scrollTop = $state(0);

    // Reset text layers and search whenever the open document changes.
    $effect(() => {
        $pdfDoc;
        textLayersByPage = new Map();
        searchMatches = [];
        searchMatchIdx = -1;
        searching = false;
    });

    $effect(() => {
        return viewerStore.subscribe((v) => {
            mode = v.mode;
            manualZoom = v.manualZoom;
            if (v.targetPage !== targetPage) {
                targetPage = v.targetPage;
                scrollToPage(targetPage);
            }
        });
    });

    $effect(() => {
        const el = container;
        if (!el) return;
        const ro = new ResizeObserver((entries) => {
            for (const e of entries) containerWidth = e.contentRect.width;
        });
        ro.observe(el);
        return () => ro.disconnect();
    });

    const fitZoom = $derived.by(() => {
        if (!$pdfDoc || containerWidth <= 0) return 1.0;
        const z = (containerWidth - FIT_MARGIN) / $pdfDoc.defaultWidth;
        return Math.max(MIN_ZOOM, Math.min(z, MAX_ZOOM));
    });

    const displayScale = $derived(mode === 'fit' ? fitZoom : manualZoom);

    let renderScale = $state(1.0);
    $effect(() => {
        const z = displayScale;
        if (mode === 'manual') { renderScale = z; return; }
        const t = setTimeout(() => { renderScale = z; }, 90);
        return () => clearTimeout(t);
    });

    // Anchor scroll position across scale changes so the top of the viewport
    // stays on the same page content (divider drag, window resize, manual zoom).
    let prevScale = 0;
    $effect(() => {
        const scale = displayScale;
        if (!container || !$pdfDoc) { prevScale = scale; return; }
        if (prevScale === 0 || prevScale === scale) { prevScale = scale; return; }
        untrack(() => {
            const baseH = $pdfDoc!.defaultHeight;
            const oldH = baseH * prevScale;
            const newH = baseH * scale;
            const oldStride = oldH + GAP;
            const newStride = newH + GAP;
            const rel = scrollTop - PAD;
            if (rel > 0) {
                const idx = Math.min(Math.floor(rel / oldStride), $pdfDoc!.numPages - 1);
                const frac = (rel - idx * oldStride) / oldH;
                const newTop = PAD + idx * newStride + frac * newH;
                container!.scrollTop = newTop;
                scrollTop = newTop;
            }
        });
        prevScale = scale;
    });

    function pageW() { return ($pdfDoc?.defaultWidth  ?? 612) * displayScale; }
    function pageH() { return ($pdfDoc?.defaultHeight ?? 792) * displayScale; }
    function stride() { return pageH() + GAP; }

    const currentPage = $derived.by(() => {
        if (!$pdfDoc) return 0;
        const idx = Math.round(scrollTop / stride());
        return Math.max(0, Math.min(idx, $pdfDoc.numPages - 1));
    });

    function scrollToPage(page: number) {
        if (!container || !$pdfDoc) return;
        const el = pageEls[page];
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    function onScroll() {
        if (container) scrollTop = container.scrollTop;
    }

    // Virtualization: IntersectionObserver drives which page slots get canvases.
    $effect(() => {
        if (!$pdfDoc || !container) return;
        renderedAt = new Map();
        const observer = new IntersectionObserver(
            (entries) => {
                const next = new Set(visiblePages);
                for (const e of entries) {
                    const idx = Number((e.target as HTMLElement).dataset.pageIdx);
                    if (e.isIntersecting) next.add(idx);
                    else next.delete(idx);
                }
                visiblePages = next;
            },
            { root: container, rootMargin: '800px 0px', threshold: 0 },
        );
        pageEls.forEach((el) => el && observer.observe(el));
        return () => observer.disconnect();
    });

    // Render canvases for visible pages.
    $effect(() => {
        const doc = $pdfDoc;
        const scale = renderScale;
        const vis = visiblePages;
        if (!doc) return;
        for (const i of vis) {
            if (renderedAt.get(i) === scale) continue;
            const canvas = canvasEls[i];
            if (!canvas) continue;
            renderedAt.set(i, scale);
            getPage(doc.proxy, i + 1)
                .then((pg) => renderPageToCanvas(pg, canvas, scale))
                .catch(() => renderedAt.delete(i));
        }
    });

    // Render text layers for visible pages (enables search highlighting).
    $effect(() => {
        const doc = $pdfDoc;
        const scale = renderScale;
        const vis = visiblePages;
        if (!doc) return;
        for (const i of vis) {
            const el = textLayerEls[i];
            if (!el) continue;
            // data-scale on the element detects freshly-mounted elements even when
            // scale hasn't changed (page went off-screen and came back).
            if (el.dataset.scale === String(scale)) continue;
            el.dataset.scale = String(scale);
            buildTextLayer(doc.proxy, i, el, scale);
        }
    });

    async function buildTextLayer(
        proxy: import('pdfjs-dist').PDFDocumentProxy,
        pageIndex: number,
        el: HTMLElement,
        scale: number,
    ) {
        const page = await getPage(proxy, pageIndex + 1);
        const viewport = page.getViewport({ scale });
        el.innerHTML = '';
        const layer = new TextLayer({ textContentSource: page.streamTextContent(), container: el, viewport });
        try { await layer.render(); } catch { return; }
        const itemsStr = layer.textContentItemsStr;
        let fullText = '';
        const offsets: number[] = [];
        for (const s of itemsStr) { offsets.push(fullText.length); fullText += s; }
        textLayersByPage.set(pageIndex, { divs: layer.textDivs, itemsStr, offsets, fullText });
        applyHighlightsToPage(pageIndex);
    }

    // ── Search ────────────────────────────────────────────────────────────────

    function onSearchInput() {
        if (searchDebounce) clearTimeout(searchDebounce);
        searchDebounce = setTimeout(runSearch, 300);
    }

    async function runSearch() {
        const query = searchQuery.trim();
        const doc = $pdfDoc;
        if (!doc || !query) {
            searchMatches = [];
            searchMatchIdx = -1;
            clearHighlights();
            return;
        }
        searching = true;
        try {
            const m = await searchAllPages(doc.proxy, doc.numPages, query);
            searchMatches = m;
            searchMatchIdx = m.length > 0 ? 0 : -1;
            applyHighlights();
            if (m.length > 0) scrollToPage(m[0].pageIndex);
        } finally {
            searching = false;
        }
    }

    function nextMatch() {
        if (!searchMatches.length) return;
        searchMatchIdx = (searchMatchIdx + 1) % searchMatches.length;
        applyHighlights();
        scrollToPage(searchMatches[searchMatchIdx].pageIndex);
    }

    function clearSearch() {
        searchQuery = '';
        searchMatches = [];
        searchMatchIdx = -1;
        clearHighlights();
    }

    function clearHighlights() {
        for (const { divs } of textLayersByPage.values())
            for (const d of divs) d.classList.remove('search-hi', 'search-cur');
    }

    function applyHighlights() {
        clearHighlights();
        for (const pi of textLayersByPage.keys()) applyHighlightsToPage(pi);
    }

    function applyHighlightsToPage(pi: number) {
        const layer = textLayersByPage.get(pi);
        if (!layer) return;
        for (const d of layer.divs) d.classList.remove('search-hi', 'search-cur');
        if (!searchQuery.trim() || !searchMatches.length) return;
        for (let mi = 0; mi < searchMatches.length; mi++) {
            const m = searchMatches[mi];
            if (m.pageIndex !== pi) continue;
            const cur = mi === searchMatchIdx;
            for (let j = 0; j < layer.divs.length; j++) {
                const s = layer.offsets[j];
                const e = s + (layer.itemsStr[j]?.length ?? 0);
                if (s < m.end && e > m.start) {
                    layer.divs[j].classList.add('search-hi');
                    if (cur) layer.divs[j].classList.add('search-cur');
                }
            }
        }
    }

    // ── Zoom / page jump ──────────────────────────────────────────────────────

    function zoomIn() {
        viewerStore.update((v) => ({ ...v, mode: 'manual', manualZoom: Math.min(displayScale + 0.25, 4.0) }));
    }
    function zoomOut() {
        viewerStore.update((v) => ({ ...v, mode: 'manual', manualZoom: Math.max(displayScale - 0.25, 0.25) }));
    }
    function fitWidth() {
        viewerStore.update((v) => ({ ...v, mode: 'fit' }));
    }

    function jumpTo(n: number) {
        if (!$pdfDoc || Number.isNaN(n)) return;
        const page = Math.max(1, Math.min(Math.floor(n), $pdfDoc.numPages)) - 1;
        targetPage = page;
        viewerStore.update((v) => ({ ...v, targetPage: page }));
        scrollToPage(page);
    }
</script>

<div class="viewer-root">
    <div class="toolbar">
        <span class="role-label">{label}</span>

        {#if $pdfDoc}
            <div class="search-bar">
                <Icon name="search" size={13} class="search-icon" />
                <input
                    class="search-input"
                    type="search"
                    placeholder="Search…"
                    bind:value={searchQuery}
                    oninput={onSearchInput}
                    onkeydown={(e) => {
                        if (e.key === 'Enter') nextMatch();
                        if (e.key === 'Escape') clearSearch();
                    }}
                    aria-label="Search in document"
                />
                {#if searching}
                    <span class="search-count">…</span>
                {:else if searchQuery}
                    <span class="search-count" class:no-results={!searchMatches.length}>
                        {searchMatches.length ? `${searchMatchIdx + 1} / ${searchMatches.length}` : '0'}
                    </span>
                    <button
                        class="icon-btn search-btn"
                        onclick={nextMatch}
                        disabled={!searchMatches.length}
                        title="Next match (Enter)"
                        aria-label="Next match"
                    ><Icon name="chevron-down" size={13} /></button>
                    <button
                        class="icon-btn search-btn"
                        onclick={clearSearch}
                        title="Clear search (Escape)"
                        aria-label="Clear search"
                    ><Icon name="x" size={13} /></button>
                {/if}
            </div>
        {/if}

        <div class="spacer"></div>

        {#if $pdfDoc}
            <label class="page-jump">
                <input
                    class="page-box"
                    type="number"
                    min="1"
                    max={$pdfDoc.numPages}
                    value={currentPage + 1}
                    onkeydown={(e) => { if (e.key === 'Enter') jumpTo(Number((e.currentTarget as HTMLInputElement).value)); }}
                    onchange={(e) => jumpTo(Number((e.currentTarget as HTMLInputElement).value))}
                />
                <span class="page-total">/ {$pdfDoc.numPages}</span>
            </label>

            <div class="zoom-group">
                <button class="icon-btn" onclick={zoomOut} title="Zoom out" aria-label="Zoom out">
                    <Icon name="zoom-out" />
                </button>
                <button class="zoom-pct" onclick={fitWidth} title="Fit width">{Math.round(displayScale * 100)}%</button>
                <button class="icon-btn" onclick={zoomIn} title="Zoom in" aria-label="Zoom in">
                    <Icon name="zoom-in" />
                </button>
                <button class="icon-btn fit-btn" class:active={mode === 'fit'} onclick={fitWidth} title="Fit page width" aria-label="Fit page width" aria-pressed={mode === 'fit'}>
                    <Icon name="fit-width" />
                </button>
            </div>
        {/if}
    </div>

    <div class="page-scroll" bind:this={container} onscroll={onScroll}>
        {#if $pdfDoc}
            {@const pw = pageW()}
            {@const ph = pageH()}
            {#each Array($pdfDoc.numPages) as _, i (i)}
                <div
                    class="page-slot"
                    data-page-idx={i}
                    style="width: {pw}px; height: {ph}px;"
                    bind:this={pageEls[i]}
                >
                    {#if visiblePages.has(i)}
                        <canvas class="page-canvas" bind:this={canvasEls[i]}></canvas>
                        <div class="text-layer" bind:this={textLayerEls[i]}></div>
                    {/if}
                    <div class="page-label">{i + 1}</div>
                </div>
            {/each}
        {:else}
            <div class="empty-state">
                <p>Open a PDF to start reading</p>
            </div>
        {/if}
    </div>
</div>

<style>
.viewer-root {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--bg-canvas);
    position: relative;
}

.toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow: hidden;
    min-width: 0;
}

.role-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    text-transform: uppercase;
}

.spacer { flex: 1; }

/* ── Search bar ── */
.search-bar {
    display: flex;
    align-items: center;
    gap: 1px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1px 4px;
    min-width: 0;
    flex-shrink: 1;
}

:global(.search-bar .search-icon) { color: var(--text-muted); flex-shrink: 0; }

.search-input {
    width: 110px;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    outline: none;
    padding: 2px 4px;
}

.search-input::-webkit-search-cancel-button { display: none; }

.search-count {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
    padding: 0 3px;
    font-variant-numeric: tabular-nums;
}

.search-count.no-results { color: var(--danger); }

.search-btn {
    border: none !important;
    background: transparent !important;
    padding: 3px !important;
    min-width: unset !important;
    border-radius: 3px !important;
}

.search-btn:hover { background: var(--bg-hover) !important; }
.search-btn:disabled { opacity: 0.4; cursor: default; }

/* ── Page jump & zoom ── */
.page-jump {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
}

.page-box {
    width: 48px;
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elevated);
    color: var(--text);
    font-size: 12px;
    text-align: right;
}

.page-total {
    font-size: 11px;
    color: var(--text-muted);
}

.zoom-group {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
}

.toolbar button {
    min-width: 30px;
    padding: 2px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
}

.toolbar button:hover { background: var(--bg-hover); }

.toolbar .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    padding: 4px;
}

.zoom-pct {
    min-width: 50px;
    text-align: center;
    font-variant-numeric: tabular-nums;
}

.fit-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
}
.fit-btn.active:hover { background: var(--accent-hover); }

/* ── Scroll area & pages ── */
.page-scroll {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
}

.page-slot {
    position: relative;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border);
    background: white;
    flex-shrink: 0;
}

.page-canvas {
    display: block;
    width: 100%;
    height: 100%;
}

.page-label {
    position: absolute;
    bottom: -20px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 10px;
    color: var(--text-muted);
}

.empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 14px;
}

/* ── Text layer (PDF.js transparent text overlay for search highlights) ── */
:global(.text-layer) {
    position: absolute;
    inset: 0;
    overflow: hidden;
    line-height: 1;
    pointer-events: none;
}

:global(.text-layer span),
:global(.text-layer br) {
    color: transparent;
    position: absolute;
    white-space: pre;
    cursor: text;
    transform-origin: 0% 0%;
    pointer-events: auto;
}

:global(.text-layer .markedContent) {
    top: 0;
    height: 0;
}

:global(.text-layer span.search-hi) {
    background: rgba(255, 205, 0, 0.45);
    border-radius: 2px;
}

:global(.text-layer span.search-cur) {
    background: rgba(255, 110, 0, 0.6);
    border-radius: 2px;
}
</style>
