<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { getCurrentWebview } from '@tauri-apps/api/webview';
    import { pdfDoc, mainViewer, sideViewer, sideOpen } from '../stores.js';
    import { openDocumentDialog, readPdf, baseName } from '../api.js';
    import { loadPdf, getPage } from '../pdf.js';
    import Icon from './Icon.svelte';

    let loading = $state(false);
    let error = $state<string | null>(null);
    let dragHover = $state(false);

    async function loadFromPath(path: string) {
        loading = true;
        error = null;
        try {
            const bytes = await readPdf(path);
            const proxy = await loadPdf(bytes);
            const page1 = await getPage(proxy, 1);
            const vp = page1.getViewport({ scale: 1 });
            pdfDoc.set({
                proxy,
                numPages: proxy.numPages,
                defaultWidth: vp.width,
                defaultHeight: vp.height,
                fileName: baseName(path),
            });
            mainViewer.set({ targetPage: 0, mode: 'fit', manualZoom: 1 });
            sideViewer.set({ targetPage: 0, mode: 'fit', manualZoom: 1 });
        } catch (e: unknown) {
            error = (e as { message?: string })?.message ?? String(e);
        } finally {
            loading = false;
        }
    }

    async function handleOpen() {
        const path = await openDocumentDialog();
        if (path) loadFromPath(path);
    }

    // Tauri intercepts OS file drops at the window level (the webview never sees
    // an HTML drop), so we listen via the webview drag-drop event for the path.
    let unlisten: (() => void) | undefined;
    onMount(async () => {
        unlisten = await getCurrentWebview().onDragDropEvent((event) => {
            const p = event.payload;
            if (p.type === 'enter' || p.type === 'over') {
                dragHover = true;
            } else if (p.type === 'leave') {
                dragHover = false;
            } else if (p.type === 'drop') {
                dragHover = false;
                const pdf = p.paths.find((x) => x.toLowerCase().endsWith('.pdf')) ?? p.paths[0];
                if (pdf) loadFromPath(pdf);
            }
        });
    });
    onDestroy(() => unlisten?.());
</script>

<header class="topbar" class:drag-hover={dragHover}>
    <span class="app-name">Argon-PDF</span>

    <button
        onclick={handleOpen}
        disabled={loading}
        class="btn icon-btn"
        title={loading ? 'Opening…' : 'Open PDF'}
        aria-label={loading ? 'Opening…' : 'Open PDF'}
    >
        <Icon name={loading ? 'loader' : 'folder-open'} class={loading ? 'spin' : ''} />
    </button>

    {#if $pdfDoc}
        <button
            onclick={() => sideOpen.update((v) => !v)}
            class="btn icon-btn"
            class:active={$sideOpen}
            title={$sideOpen ? 'Hide side page' : 'Show side page'}
            aria-label={$sideOpen ? 'Hide side page' : 'Show side page'}
            aria-pressed={$sideOpen}
        >
            <Icon name="columns-2" />
        </button>
        <span class="file-name" title={$pdfDoc.fileName}>{$pdfDoc.fileName}</span>
        <span class="page-count">{$pdfDoc.numPages} pages</span>
    {/if}

    {#if error}
        <span class="error-msg">{error}</span>
    {/if}

    <span class="drag-hint">{dragHover ? 'Drop to open' : 'or drop a PDF here'}</span>
</header>

<style>
.topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    height: 44px;
    background: var(--bg-panel);
    color: var(--text);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
}

.topbar.drag-hover {
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
}

.app-name {
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--accent);
}

.btn {
    padding: 4px 14px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    transition: background 0.15s;
}

.btn:hover { background: var(--bg-hover); }

.btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
}

.icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 6px;
}

.icon-btn:disabled {
    opacity: 0.6;
    cursor: default;
}

:global(.icon-btn .spin) {
    animation: spin 0.8s linear infinite;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}

.file-name {
    font-size: 12px;
    color: var(--text);
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.page-count {
    font-size: 11px;
    color: var(--text-muted);
}

.error-msg {
    color: var(--danger);
    font-size: 12px;
}

.drag-hint {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-muted);
}
</style>
