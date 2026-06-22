<script lang="ts">
    import { document as docStore, notesOpen, highlights } from '../stores.js';
    import { openDocument, openDocumentDialog, listHighlights } from '../api.js';

    let loading = $state(false);
    let error = $state<string | null>(null);

    async function handleOpen() {
        error = null;
        const path = await openDocumentDialog();
        if (!path) return;
        loading = true;
        try {
            const doc = await openDocument(path);
            docStore.set(doc);
            const hl = await listHighlights(doc.doc_id);
            highlights.set(hl);
        } catch (e: unknown) {
            const err = e as { type?: string; message?: string };
            if (err?.type === 'NeedsPassword') {
                error = 'This PDF is password-protected.';
            } else {
                error = err?.message ?? String(e);
            }
        } finally {
            loading = false;
        }
    }

    async function handleDrop(e: DragEvent) {
        e.preventDefault();
        const file = e.dataTransfer?.files?.[0];
        if (!file) return;
        const path = (file as File & { path?: string }).path;
        if (!path) return;
        error = null;
        loading = true;
        try {
            const doc = await openDocument(path);
            docStore.set(doc);
            const hl = await listHighlights(doc.doc_id);
            highlights.set(hl);
        } catch (e: unknown) {
            const err = e as { type?: string; message?: string };
            error = err?.message ?? String(e);
        } finally {
            loading = false;
        }
    }

    function handleDragover(e: DragEvent) {
        e.preventDefault();
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="topbar" ondrop={handleDrop} ondragover={handleDragover}>
    <span class="app-name">Argon-PDF</span>

    <button onclick={handleOpen} disabled={loading} class="btn-open">
        {loading ? 'Opening…' : 'Open PDF'}
    </button>

    {#if $docStore}
        <button
            onclick={() => notesOpen.update(v => !v)}
            class="btn-notes"
            class:active={$notesOpen}
        >
            Notes {$notesOpen ? '▶' : '◀'}
        </button>
    {/if}

    {#if error}
        <span class="error-msg">{error}</span>
    {/if}

    <span class="drag-hint">or drop a PDF here</span>
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

.app-name {
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--accent);
}

.btn-open, .btn-notes {
    padding: 4px 14px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    transition: background 0.15s;
}

.btn-open:hover, .btn-notes:hover {
    background: var(--bg-hover);
}

.btn-notes.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
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
