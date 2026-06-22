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

    let container: HTMLElement;
    let visiblePages = $state(new Set<number>());
    let pageElements: HTMLElement[] = [];

    let zoom = $state(1.0);
    let targetPage = $state(0);

    $effect(() => {
        // Use effect so the subscription re-binds if viewerStore prop changes
        return viewerStore.subscribe(v => {
            zoom = v.zoom;
            if (v.targetPage !== targetPage) {
                targetPage = v.targetPage;
                scrollToPage(targetPage);
            }
        });
    });

    function scalePct(z: number) { return Math.round(z * 100); }
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
            { root: container, rootMargin: '200px 0px', threshold: 0 }
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

    function zoomIn() {
        viewerStore.update(v => ({ ...v, zoom: Math.min(v.zoom + 0.25, 4.0) }));
    }

    function zoomOut() {
        viewerStore.update(v => ({ ...v, zoom: Math.max(v.zoom - 0.25, 0.25) }));
    }

    function zoomReset() {
        viewerStore.update(v => ({ ...v, zoom: 1.0 }));
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
        <button onclick={zoomReset} title="Reset zoom">{Math.round(zoom * 100)}%</button>
        <button onclick={zoomIn} title="Zoom in">+</button>
    </div>

    <!-- Scrollable page area -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="page-scroll" bind:this={container}>
        {#if $docStore}
            {#each $docStore.page_sizes as size, i}
                {@const pw = ptsToPx(size.width_pts, zoom)}
                {@const ph = ptsToPx(size.height_pts, zoom)}
                <div
                    class="page-slot"
                    data-page-idx={i}
                    style="width: {pw}px; height: {ph}px;"
                    bind:this={pageElements[i]}
                >
                    {#if visiblePages.has(i)}
                        <!-- Raster bitmap from PDFium -->
                        <img
                            src={renderUrl($docStore.doc_id, i, scalePct(zoom))}
                            alt="Page {i + 1}"
                            width={pw}
                            height={ph}
                            draggable="false"
                        />
                        <!-- Existing highlights -->
                        <HighlightLayer
                            pageHighlights={pageHighlights(i)}
                            {size}
                            {zoom}
                            pageIndex={i}
                        />
                        <!-- Text selection overlay (only if doc has text layer) -->
                        {#if $docStore.has_text_layer}
                            <SelectionLayer
                                docId={$docStore.doc_id}
                                pageIndex={i}
                                {size}
                                {zoom}
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
    height: 100%;
    overflow: hidden;
    background: #181825;
    position: relative;
}

.zoom-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: #1e1e2e;
    border-bottom: 1px solid #313244;
    flex-shrink: 0;
}

.zoom-bar button {
    min-width: 32px;
    padding: 2px 8px;
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 4px;
    color: #cdd6f4;
    cursor: pointer;
    font-size: 13px;
}

.zoom-bar button:hover { background: #45475a; }

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
    box-shadow: 0 4px 16px rgba(0,0,0,0.5);
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
    color: #6c7086;
}

.empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #6c7086;
    font-size: 14px;
}

.no-text-banner {
    position: absolute;
    bottom: 8px;
    left: 8px;
    right: 8px;
    background: #45475a;
    color: #fab387;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
    text-align: center;
    pointer-events: none;
}
</style>
