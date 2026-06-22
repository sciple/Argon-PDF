<script lang="ts">
    import { document as docStore, highlights } from '../stores.js';
    import { renderUrl } from '../api.js';
    import HighlightLayer from './HighlightLayer.svelte';
    import SelectionLayer from './SelectionLayer.svelte';
    import type { ViewerState } from '../types.js';
    import type { Writable } from 'svelte/store';

    interface Props {
        viewerStore: Writable<ViewerState>;
        role: 'center' | 'right';
    }
    let { viewerStore, role }: Props = $props();

    const BASE_DPI = 96;
    const FIT_MARGIN = 24; // px breathing room around the page in fit mode
    const MIN_ZOOM = 0.1;
    const MAX_ZOOM = 6.0;

    let container = $state<HTMLElement>();
    let visiblePages = $state(new Set<number>());
    let pageElements: HTMLElement[] = [];

    // Mirrors of the viewer store
    let mode = $state<'fit' | 'manual'>('fit');
    let manualZoom = $state(1.0);
    let targetPage = $state(0);

    // Available content width of the scroll area (tracked via ResizeObserver).
    let containerWidth = $state(0);

    $effect(() => {
        // Use effect so the subscription re-binds if viewerStore prop changes
        return viewerStore.subscribe(v => {
            mode = v.mode;
            manualZoom = v.manualZoom;
            if (v.targetPage !== targetPage) {
                targetPage = v.targetPage;
                scrollToPage(targetPage);
            }
        });
    });

    // Track the pane's content width so fit-width re-fits on divider/window resize.
    $effect(() => {
        const el = container;
        if (!el) return;
        const ro = new ResizeObserver((entries) => {
            for (const e of entries) containerWidth = e.contentRect.width;
        });
        ro.observe(el);
        return () => ro.disconnect();
    });

    // Widest page drives the fit scale so no page overflows horizontally.
    const maxPageWidthPts = $derived(
        $docStore && $docStore.page_sizes.length
            ? Math.max(...$docStore.page_sizes.map((s) => s.width_pts))
            : 612
    );

    // Zoom that makes the widest page fill the available width.
    const fitZoom = $derived.by(() => {
        if (!$docStore || containerWidth <= 0) return 1.0;
        const pageWpx = (maxPageWidthPts / 72) * BASE_DPI;
        const z = (containerWidth - FIT_MARGIN) / pageWpx;
        return Math.max(MIN_ZOOM, Math.min(z, MAX_ZOOM));
    });

    // The zoom actually used for layout/overlays (display size).
    const effectiveZoom = $derived(mode === 'fit' ? fitZoom : manualZoom);

    // The zoom used for the rendered bitmap resolution. Debounced in fit mode so
    // dragging the divider rescales via CSS smoothly and re-renders crisp on settle.
    let renderZoom = $state(1.0);
    $effect(() => {
        const z = effectiveZoom;
        if (mode === 'manual') {
            renderZoom = z; // discrete button clicks: update immediately
            return;
        }
        const t = setTimeout(() => { renderZoom = z; }, 100);
        return () => clearTimeout(t);
    });

    // Render at devicePixelRatio so text is crisp on HiDPI screens (Windows
    // display scaling). The bitmap is rendered larger but displayed at the same
    // CSS size, so the browser never has to upscale it. Capped to avoid huge
    // renders on extreme scaling setups.
    function dpr() { return Math.min(window.devicePixelRatio || 1, 3); }
    function scalePct(z: number) { return Math.round(z * dpr() * 100); }
    function ptsToPx(pts: number, z: number) { return (pts / 72) * BASE_DPI * z; }

    function scrollToPage(page: number) {
        if (!container || !$docStore) return;
        const el = pageElements[page];
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }

    function setupObserver() {
        if (!container) return;
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
            // Generous vertical margin so pages render ~1.5 screens ahead of
            // the viewport and are ready by the time they scroll in.
            { root: container, rootMargin: '1200px 0px', threshold: 0 }
        );
        pageElements.forEach(el => { if (el) observer.observe(el); });
        return () => observer.disconnect();
    }

    $effect(() => {
        // Re-run observer whenever doc or zoom changes (new pageElements)
        if ($docStore) {
            const cleanup = setupObserver();
            return cleanup;
        }
    });

    // Explicit zoom actions switch the viewer into manual mode, continuing from
    // whatever zoom is currently displayed (the fit zoom, if we were fitting).
    function zoomIn() {
        const z = Math.min(effectiveZoom + 0.25, 4.0);
        viewerStore.update(v => ({ ...v, mode: 'manual', manualZoom: z }));
    }

    function zoomOut() {
        const z = Math.max(effectiveZoom - 0.25, 0.25);
        viewerStore.update(v => ({ ...v, mode: 'manual', manualZoom: z }));
    }

    function fitWidth() {
        viewerStore.update(v => ({ ...v, mode: 'fit' }));
    }

    // Filter highlights for a specific page
    function pageHighlights(pageIdx: number) {
        return $highlights.filter(h => h.page_index === pageIdx);
    }
</script>

<div class="viewer-root">
    <!-- Zoom controls -->
    <div class="zoom-bar">
        <button onclick={zoomOut} title="Zoom out">−</button>
        <button class="zoom-pct" onclick={fitWidth} title="Fit width">{Math.round(effectiveZoom * 100)}%</button>
        <button onclick={zoomIn} title="Zoom in">+</button>
        <button class="fit-btn" class:active={mode === 'fit'} onclick={fitWidth} title="Fit page width">Fit</button>
    </div>

    <!-- Scrollable page area -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="page-scroll" bind:this={container}>
        {#if $docStore}
            {#each $docStore.page_sizes as size, i}
                {@const pw = ptsToPx(size.width_pts, effectiveZoom)}
                {@const ph = ptsToPx(size.height_pts, effectiveZoom)}
                <div
                    class="page-slot"
                    data-page-idx={i}
                    style="width: {pw}px; height: {ph}px;"
                    bind:this={pageElements[i]}
                >
                    {#if visiblePages.has(i)}
                        <!-- Raster bitmap from PDFium (resolution = renderZoom × DPR) -->
                        <img
                            src={renderUrl($docStore.doc_id, i, scalePct(renderZoom))}
                            alt="Page {i + 1}"
                            width={pw}
                            height={ph}
                            draggable="false"
                        />
                        <!-- Existing highlights -->
                        <HighlightLayer
                            pageHighlights={pageHighlights(i)}
                            {size}
                            zoom={effectiveZoom}
                            pageIndex={i}
                        />
                        <!-- Text selection overlay (only if doc has text layer) -->
                        {#if $docStore.has_text_layer}
                            <SelectionLayer
                                docId={$docStore.doc_id}
                                pageIndex={i}
                                {size}
                                zoom={effectiveZoom}
                            />
                        {/if}
                    {/if}
                    <div class="page-label">Page {i + 1}</div>
                </div>
            {/each}
        {:else}
            <div class="empty-state">
                <p>Open a PDF to start reading</p>
            </div>
        {/if}
    </div>

    {#if $docStore && !$docStore.has_text_layer}
        <div class="no-text-banner">
            This document has no selectable text — highlighting is unavailable.
        </div>
    {/if}
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

.zoom-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
}

.zoom-bar button {
    min-width: 32px;
    padding: 2px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
}

.zoom-bar button:hover { background: var(--bg-hover); }

.zoom-pct {
    min-width: 52px;
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
    overflow-y: auto;
    overflow-x: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
}

.page-slot {
    position: relative;
    box-shadow: 0 2px 10px rgba(0,0,0,0.15);
    border: 1px solid var(--border);
    background: white;
    flex-shrink: 0;
}

.page-slot img {
    display: block;
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
    user-select: none;
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

.no-text-banner {
    position: absolute;
    bottom: 8px;
    left: 8px;
    right: 8px;
    background: #fef3c7;
    color: #92400e;
    border: 1px solid #fcd34d;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    text-align: center;
    pointer-events: none;
}
</style>
