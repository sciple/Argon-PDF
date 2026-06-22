<script lang="ts">
    import { highlights } from '../stores.js';
    import { getPageTextLayout, addHighlight } from '../api.js';
    import type { PageSize, TextLayout } from '../types.js';

    interface Props {
        docId: string;
        pageIndex: number;
        size: PageSize;
        zoom: number;
    }
    let { docId, pageIndex, size, zoom }: Props = $props();

    const BASE_DPI = 96;

    let layout = $state<TextLayout | null>(null);

    $effect(() => {
        // Load text layout when this page becomes visible
        getPageTextLayout(docId, pageIndex).then(l => { layout = l; }).catch(() => {});
    });

    function toStyle(x: number, y: number, w: number, h: number) {
        const scale = (BASE_DPI / 72) * zoom;
        const pageHeightPx = (size.height_pts / 72) * BASE_DPI * zoom;
        return [
            `left:${x * scale}px`,
            `top:${pageHeightPx - (y + h) * scale}px`,
            `width:${Math.max(w * scale, 2)}px`,
            `height:${h * scale}px`,
        ].join(';');
    }

    async function onMouseUp() {
        const sel = window.getSelection();
        if (!sel || sel.isCollapsed) return;

        const anchorEl = sel.anchorNode?.parentElement;
        const focusEl = sel.focusNode?.parentElement;
        if (!anchorEl || !focusEl) return;

        const startIdx = Number(anchorEl.dataset.charIdx);
        const endIdx = Number(focusEl.dataset.charIdx);
        if (isNaN(startIdx) || isNaN(endIdx)) return;

        const start = Math.min(startIdx, endIdx);
        const end = Math.max(startIdx, endIdx) + 1;
        if (end <= start) return;

        sel.removeAllRanges();

        try {
            const hl = await addHighlight(docId, pageIndex, start, end);
            highlights.update(hs => [...hs, hl]);
        } catch {
            // ignore empty selection errors
        }
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="selection-layer" onmouseup={onMouseUp}>
    {#if layout}
        {#each layout.chars as ch (ch.char_index)}
            <span
                class="char-span"
                data-char-idx={ch.char_index}
                style={toStyle(ch.x, ch.y, ch.width, ch.height)}
            >{ch.char_val}</span>
        {/each}
    {/if}
</div>

<style>
.selection-layer {
    position: absolute;
    inset: 0;
    pointer-events: auto;
    cursor: text;
    user-select: text;
    -webkit-user-select: text;
}

.char-span {
    position: absolute;
    color: transparent;
    white-space: pre;
    line-height: 1;
    /* font-size scales with width so selection sizing is approximately correct */
    font-size: var(--char-font-size, 12px);
}
</style>
