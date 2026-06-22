<script lang="ts">
    import { highlights } from '../stores.js';
    import { deleteHighlight } from '../api.js';
    import type { Highlight, PageSize } from '../types.js';

    interface Props {
        pageHighlights: Highlight[];
        size: PageSize;
        zoom: number;
        pageIndex: number;
    }
    let { pageHighlights, size, zoom, pageIndex }: Props = $props();

    const BASE_DPI = 96;

    // Convert PDF-point coordinates to CSS pixels.
    // PDF coordinates: origin bottom-left, y grows up.
    // CSS coordinates: origin top-left, y grows down.
    function toPixel(pdfX: number, pdfY: number, pdfW: number, pdfH: number) {
        const scale = (BASE_DPI / 72) * zoom;
        const pageHeightPx = (size.height_pts / 72) * BASE_DPI * zoom;
        return {
            left: pdfX * scale,
            top: pageHeightPx - (pdfY + pdfH) * scale,
            width: pdfW * scale,
            height: pdfH * scale,
        };
    }

    async function handleDelete(id: number) {
        await deleteHighlight(id);
        highlights.update(hs => hs.filter(h => h.id !== id));
    }
</script>

{#each pageHighlights as hl (hl.id)}
    {#each hl.rects as rect}
        {@const px = toPixel(rect.x, rect.y, rect.width, rect.height)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
            class="highlight-rect"
            style="
                left: {px.left}px;
                top: {px.top}px;
                width: {px.width}px;
                height: {px.height}px;
                background: {hl.color}55;
                border-bottom: 2px solid {hl.color};
            "
            title={hl.text_excerpt}
            onclick={() => handleDelete(hl.id)}
        ></div>
    {/each}
{/each}

<style>
.highlight-rect {
    position: absolute;
    cursor: pointer;
    transition: opacity 0.15s;
    border-radius: 1px;
}

.highlight-rect:hover {
    opacity: 0.5;
}

.highlight-rect:hover::after {
    content: '✕';
    position: absolute;
    top: -14px;
    right: 0;
    background: var(--danger);
    color: white;
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    pointer-events: none;
}
</style>
