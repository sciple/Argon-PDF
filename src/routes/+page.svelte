<script lang="ts">
    import TopBar from '$lib/components/TopBar.svelte';
    import ThumbnailStrip from '$lib/components/ThumbnailStrip.svelte';
    import PageViewer from '$lib/components/PageViewer.svelte';
    import NotesPanel from '$lib/components/NotesPanel.svelte';
    import { centerViewer, rightViewer, notesOpen } from '$lib/stores.js';
</script>

<div class="app-shell">
    <TopBar />

    <div class="main-area">
        <ThumbnailStrip />

        <div class="viewers">
            <PageViewer viewerStore={centerViewer} role="center" />
            <div class="viewer-divider"></div>
            <PageViewer viewerStore={rightViewer} role="right" />
        </div>

        {#if $notesOpen}
            <NotesPanel />
        {/if}
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

.viewer-divider {
    width: 3px;
    background: var(--border);
    flex-shrink: 0;
}
</style>
