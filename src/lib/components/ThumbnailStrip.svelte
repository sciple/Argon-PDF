<script lang="ts">
    import { document as docStore } from '../stores.js';
    import Thumbnail from './Thumbnail.svelte';

    let container: HTMLElement;
    let visibleThumbs = $state(new Set<number>());
    let thumbElements: HTMLElement[] = [];

    $effect(() => {
        if (!$docStore || !container) return;

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
            { root: container, rootMargin: '300px 0px', threshold: 0 }
        );

        thumbElements.forEach(el => { if (el) observer.observe(el); });
        return () => observer.disconnect();
    });
</script>

<aside class="strip" bind:this={container}>
    {#if $docStore}
        {#each $docStore.page_sizes as size, i}
            <div
                class="thumb-row"
                data-thumb-idx={i}
                bind:this={thumbElements[i]}
            >
                <Thumbnail
                    pageIndex={i}
                    {size}
                    visible={visibleThumbs.has(i)}
                />
            </div>
        {/each}
    {:else}
        <div class="strip-empty">
            <span>No PDF open</span>
        </div>
    {/if}
</aside>

<style>
.strip {
    width: 180px;
    min-width: 180px;
    overflow-y: auto;
    overflow-x: hidden;
    background: #1e1e2e;
    border-right: 1px solid #313244;
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
    color: #6c7086;
    font-size: 12px;
}
</style>
