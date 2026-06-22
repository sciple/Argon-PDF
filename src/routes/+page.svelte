<script lang="ts">
    import TopBar from '$lib/components/TopBar.svelte';
    import ThumbnailStrip from '$lib/components/ThumbnailStrip.svelte';
    import PageViewer from '$lib/components/PageViewer.svelte';
    import { mainViewer, sideViewer, sideOpen } from '$lib/stores.js';

    // Fraction of the viewer area given to the main panel (rest goes to side).
    let centerFraction = $state(0.5);
    let viewersEl = $state<HTMLDivElement>();
    let dragging = $state(false);

    const MIN = 0.15;
    const MAX = 0.85;
    const STEP = 0.02;

    function fractionFromX(clientX: number): number {
        if (!viewersEl) return centerFraction;
        const rect = viewersEl.getBoundingClientRect();
        const f = (clientX - rect.left) / rect.width;
        return Math.max(MIN, Math.min(MAX, f));
    }

    function onDividerPointerDown(e: PointerEvent) {
        dragging = true;
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
        e.preventDefault();
    }

    function onDividerPointerMove(e: PointerEvent) {
        if (!dragging) return;
        centerFraction = fractionFromX(e.clientX);
    }

    function onDividerPointerUp(e: PointerEvent) {
        dragging = false;
        try {
            (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
        } catch {
            // pointer already released
        }
    }

    function onDividerKey(e: KeyboardEvent) {
        if (e.key === 'ArrowLeft') {
            centerFraction = Math.max(MIN, centerFraction - STEP);
            e.preventDefault();
        } else if (e.key === 'ArrowRight') {
            centerFraction = Math.min(MAX, centerFraction + STEP);
            e.preventDefault();
        } else if (e.key === 'Home') {
            centerFraction = 0.5;
            e.preventDefault();
        }
    }
</script>

<div class="app-shell">
    <TopBar />

    <div class="main-area">
        <ThumbnailStrip />

        <div class="viewers" class:dragging bind:this={viewersEl}>
            <div class="viewer-pane" style="flex-grow: {$sideOpen ? centerFraction : 1}">
                <PageViewer viewerStore={mainViewer} label="Main" />
            </div>

            {#if $sideOpen}
                <!-- A focusable, keyboard-operable separator is the ARIA window-splitter pattern -->
                <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <div
                    class="viewer-divider"
                    role="separator"
                    aria-orientation="vertical"
                    aria-valuenow={Math.round(centerFraction * 100)}
                    aria-valuemin={Math.round(MIN * 100)}
                    aria-valuemax={Math.round(MAX * 100)}
                    tabindex="0"
                    title="Drag to resize the viewers (double-click to reset)"
                    onpointerdown={onDividerPointerDown}
                    onpointermove={onDividerPointerMove}
                    onpointerup={onDividerPointerUp}
                    ondblclick={() => (centerFraction = 0.5)}
                    onkeydown={onDividerKey}
                >
                    <div class="divider-grip"></div>
                </div>

                <div class="viewer-pane" style="flex-grow: {1 - centerFraction}">
                    <PageViewer viewerStore={sideViewer} label="Side" />
                </div>
            {/if}
        </div>
    </div>
</div>

<style>
:global(:root) {
    /* Light theme palette */
    --bg-canvas: #e7e7ec;   /* viewer area behind the white pages */
    --bg-panel: #ffffff;    /* top bar, thumbnail strip, notes panel */
    --bg-elevated: #f1f1f4; /* buttons, chips */
    --bg-hover: #e4e4ea;    /* hover state */
    --border: #d9d9e0;      /* panel borders / dividers */
    --text: #1f1f24;        /* primary text */
    --text-muted: #71717a;  /* secondary text */
    --accent: #7c3aed;      /* brand purple */
    --accent-hover: #6d28d9;
    --on-accent: #ffffff;   /* text on accent */
    --danger: #dc2626;      /* delete / errors */
}

:global(*, *::before, *::after) {
    box-sizing: border-box;
}

:global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    overflow: hidden;
    font-family: system-ui, -apple-system, sans-serif;
    background: var(--bg-canvas);
    color: var(--text);
}

:global(#svelte) {
    height: 100%;
}

.app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
}

.main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
}

.viewers {
    flex: 1;
    display: flex;
    overflow: hidden;
}

.viewer-pane {
    flex-basis: 0;
    min-width: 0;     /* allow shrinking below page width */
    height: 100%;
    overflow: hidden;
}

.viewer-divider {
    width: 6px;
    flex-shrink: 0;
    background: var(--border);
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.12s;
    touch-action: none;
}

.viewer-divider:hover,
.viewers.dragging .viewer-divider {
    background: var(--accent);
}

.viewer-divider:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
}

.divider-grip {
    width: 2px;
    height: 28px;
    border-radius: 2px;
    background: var(--text-muted);
    opacity: 0.6;
    pointer-events: none;
}

.viewer-divider:hover .divider-grip,
.viewers.dragging .divider-grip {
    background: #fff;
    opacity: 0.9;
}

/* While dragging, suppress text selection and pointer interactions in the panes */
.viewers.dragging {
    user-select: none;
    cursor: col-resize;
}

.viewers.dragging .viewer-pane {
    pointer-events: none;
}
</style>
