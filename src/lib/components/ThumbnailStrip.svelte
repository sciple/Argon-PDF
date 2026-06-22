<script lang="ts">
    import { pdfDoc } from '../stores.js';
    import Thumbnail from './Thumbnail.svelte';

    let container = $state<HTMLElement>();
    let visibleThumbs = $state(new Set<number>());
    let thumbEls: HTMLElement[] = [];

    $effect(() => {
        if (!$pdfDoc || !container) return;
        const observer = new IntersectionObserver(
            (entries) => {
                const next = new Set(visibleThumbs);
                for (const e of entries) {
                    const idx = Number((e.target as HTMLElement).dataset.thumbIdx);
                    if (e.isIntersecting) next.add(idx);
                    else next.delete(idx);
                }
                visibleThumbs = next;
            },
            { root: container, rootMargin: '400px 0px', threshold: 0 }
        );
        thumbEls.forEach((el) => el && observer.observe(el));
        return () => observer.disconnect();
    });
</script>

<aside class="strip" bind:this={container}>
    {#if $pdfDoc}
        {#each Array($pdfDoc.numPages) as _, i (i)}
            <div class="thumb-row" data-thumb-idx={i} bind:this={thumbEls[i]}>
                <Thumbnail pageIndex={i} visible={visibleThumbs.has(i)} />
            </div>
        {/each}
    {:else}
        <div class="strip-empty"><span>No PDF open</span></div>
    {/if}
</aside>

<style>
.strip {
    width: 180px;
    min-width: 180px;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 28px;
    padding: 12px 8px 24px;
}

.thumb-row {
    width: 100%;
    display: flex;
    justify-content: center;
}

.strip-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 12px;
}
</style>
