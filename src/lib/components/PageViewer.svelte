<script lang="ts">
    import { untrack } from 'svelte';
    import { pdfDoc } from '../stores.js';
    import { getPage, renderPageToCanvas } from '../pdf.js';
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
    // page index -> scale it was last rendered at (avoid redundant re-renders)
    let renderedAt = new Map<number, number>();

    // Mirrored store state
    let mode = $state<'fit' | 'manual'>('fit');
    let manualZoom = $state(1.0);
    let targetPage = $state(0);

    let containerWidth = $state(0);
    let scrollTop = $state(0);

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

    // Track pane width so fit-width re-fits on divider/window resize.
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

    // Live display scale (drives slot sizes); render scale is debounced so
    // dragging the divider rescales via CSS smoothly and re-renders crisp on settle.
    const displayScale = $derived(mode === 'fit' ? fitZoom : manualZoom);

    let renderScale = $state(1.0);
    $effect(() => {
        const z = displayScale;
        if (mode === 'manual') {
            renderScale = z;
            return;
        }
        const t = setTimeout(() => { renderScale = z; }, 90);
        return () => clearTimeout(t);
    });

    // Keep the reading position anchored when the scale changes (divider drag,
    // window resize, manual zoom). Without this the slots resize but scrollTop
    // stays a fixed pixel value, so the content under the viewport drifts and
    // you lose your place. We pin the page (+ fractional offset into it) that
    // sits at the top of the viewport so it stays put across the rescale.
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
            const rel = scrollTop - PAD; // offset of viewport top below page 0's top
            if (rel > 0) {
                const idx = Math.min(Math.floor(rel / oldStride), $pdfDoc!.numPages - 1);
                const frac = (rel - idx * oldStride) / oldH; // position within the anchored page
                const newTop = PAD + idx * newStride + frac * newH;
                container!.scrollTop = newTop;
                scrollTop = newTop;
            }
        });
        prevScale = scale;
    });

    function pageW() { return ($pdfDoc?.defaultWidth ?? 612) * displayScale; }
    function pageH() { return ($pdfDoc?.defaultHeight ?? 792) * displayScale; }
    function stride() { return pageH() + GAP; }

    const currentPage = $derived.by(() => {
        if (!$pdfDoc) return 0;
        const idx = Math.round((scrollTop) / stride());
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

    // Virtualization: observe which page slots are near the viewport.
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
            { root: container, rootMargin: '800px 0px', threshold: 0 }
        );
        pageEls.forEach((el) => el && observer.observe(el));
        return () => observer.disconnect();
    });

    // Render visible pages to their canvases at the current render scale.
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

    function zoomIn() {
        const z = Math.min(displayScale + 0.25, 4.0);
        viewerStore.update((v) => ({ ...v, mode: 'manual', manualZoom: z }));
    }
    function zoomOut() {
        const z = Math.max(displayScale - 0.25, 0.25);
        viewerStore.update((v) => ({ ...v, mode: 'manual', manualZoom: z }));
    }
    function fitWidth() {
        viewerStore.update((v) => ({ ...v, mode: 'fit' }));
    }

    function jumpTo(n: number) {
        if (!$pdfDoc || Number.isNaN(n)) return;
        const page = Math.max(1, Math.min(Math.floor(n), $pdfDoc.numPages)) - 1;
        // force a scroll even if targetPage is unchanged
        targetPage = page;
        viewerStore.update((v) => ({ ...v, targetPage: page }));
        scrollToPage(page);
    }
</script>

<div class="viewer-root">
    <div class="toolbar">
        <span class="role-label">{label}</span>

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
                <button onclick={zoomOut} title="Zoom out">−</button>
                <button class="zoom-pct" onclick={fitWidth} title="Fit width">{Math.round(displayScale * 100)}%</button>
                <button onclick={zoomIn} title="Zoom in">+</button>
                <button class="fit-btn" class:active={mode === 'fit'} onclick={fitWidth} title="Fit page width">Fit</button>
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
}

.role-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    text-transform: uppercase;
}

.spacer { flex: 1; }

.page-jump {
    display: flex;
    align-items: center;
    gap: 4px;
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
</style>
