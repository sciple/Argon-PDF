<script lang="ts">
    import { document as docStore, centerViewer, rightViewer } from '../stores.js';
    import { renderUrl } from '../api.js';
    import type { PageSize } from '../types.js';

    interface Props {
        pageIndex: number;
        size: PageSize;
        visible: boolean;
    }
    let { pageIndex, size, visible }: Props = $props();

    // Thumbnail scale: ~150px wide
    const THUMB_WIDTH_PX = 150;
    const scale = $derived(
        $docStore ? Math.round((THUMB_WIDTH_PX / ((size.width_pts / 72) * 96)) * 100) : 20
    );
    const thumbH = $derived(
        Math.round((size.height_pts / size.width_pts) * THUMB_WIDTH_PX)
    );

    function sendToCenter() {
        centerViewer.update(v => ({ ...v, targetPage: pageIndex }));
    }

    function sendToRight() {
        rightViewer.update(v => ({ ...v, targetPage: pageIndex }));
    }

    let hovered = $state(false);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="thumb-wrap"
    onmouseenter={() => (hovered = true)}
    onmouseleave={() => (hovered = false)}
    style="width: {THUMB_WIDTH_PX}px; height: {thumbH}px;"
>
    {#if visible && $docStore}
        <img
            src={renderUrl($docStore.doc_id, pageIndex, scale)}
            alt="Page {pageIndex + 1}"
            width={THUMB_WIDTH_PX}
            height={thumbH}
            draggable="false"
        />
    {:else}
        <div class="thumb-placeholder" style="height: {thumbH}px;"></div>
    {/if}

    <span class="page-num">{pageIndex + 1}</span>

    {#if hovered}
        <div class="hover-buttons">
            <button onclick={sendToCenter} title="Show in center viewer">Center</button>
            <button onclick={sendToRight} title="Show in right viewer">Right</button>
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

.thumb-wrap img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 3px;
}

.thumb-placeholder {
    width: 100%;
    background: var(--bg-hover);
    border-radius: 3px;
}

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
    width: 80%;
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
