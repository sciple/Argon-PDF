<script lang="ts">
    import { pdfDoc, mainViewer, sideViewer, sideOpen } from '../stores.js';
    import { getPage, renderPageToCanvas } from '../pdf.js';

    interface Props {
        pageIndex: number;
        visible: boolean;
    }
    let { pageIndex, visible }: Props = $props();

    const THUMB_WIDTH = 150;

    let canvas = $state<HTMLCanvasElement>();
    let hovered = $state(false);
    let rendered = false;

    const thumbHeight = $derived(
        $pdfDoc ? ($pdfDoc.defaultHeight / $pdfDoc.defaultWidth) * THUMB_WIDTH : THUMB_WIDTH * 1.3
    );

    $effect(() => {
        const doc = $pdfDoc;
        const cv = canvas;
        if (!doc || !visible || !cv || rendered) return;
        rendered = true;
        const scale = THUMB_WIDTH / doc.defaultWidth;
        getPage(doc.proxy, pageIndex + 1)
            .then((pg) => renderPageToCanvas(pg, cv, scale))
            .catch(() => { rendered = false; });
    });

    function toMain() {
        mainViewer.update((v) => ({ ...v, targetPage: pageIndex }));
    }
    function toSide() {
        sideOpen.set(true);
        sideViewer.update((v) => ({ ...v, targetPage: pageIndex }));
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="thumb-wrap"
    style="width: {THUMB_WIDTH}px; height: {thumbHeight}px;"
    onmouseenter={() => (hovered = true)}
    onmouseleave={() => (hovered = false)}
>
    {#if visible && $pdfDoc}
        <canvas class="thumb-canvas" bind:this={canvas}></canvas>
    {:else}
        <div class="thumb-placeholder"></div>
    {/if}

    <span class="page-num">{pageIndex + 1}</span>

    {#if hovered}
        <div class="hover-buttons">
            <button onclick={toMain} title="Show in main viewer">Main</button>
            <button onclick={toSide} title="Show in side viewer">Side</button>
        </div>
    {/if}
</div>

<style>
.thumb-wrap {
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: visible;
}

.thumb-canvas,
.thumb-placeholder {
    display: block;
    width: 100%;
    height: 100%;
    border-radius: 3px;
}

.thumb-placeholder { background: var(--bg-hover); }

.page-num {
    position: absolute;
    bottom: -18px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 10px;
    color: var(--text-muted);
    pointer-events: none;
}

.hover-buttons {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: rgba(255, 255, 255, 0.78);
    border-radius: 3px;
}

.hover-buttons button {
    width: 70%;
    padding: 4px 0;
    background: var(--accent);
    border: none;
    border-radius: 4px;
    color: var(--on-accent);
    font-size: 12px;
    cursor: pointer;
    font-weight: 600;
}

.hover-buttons button:hover { background: var(--accent-hover); }
</style>
