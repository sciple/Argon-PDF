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
:global(*, *::before, *::after) {
    box-sizing: border-box;
}

:global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    overflow: hidden;
    font-family: system-ui, -apple-system, sans-serif;
    background: #181825;
    color: #cdd6f4;
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
    background: #313244;
    flex-shrink: 0;
}
</style>
