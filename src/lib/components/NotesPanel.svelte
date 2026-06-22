<script lang="ts">
    import { highlights, centerViewer, rightViewer } from '../stores.js';
    import { deleteHighlight } from '../api.js';
    import type { Highlight } from '../types.js';

    function sendToCenter(hl: Highlight) {
        centerViewer.update(v => ({ ...v, targetPage: hl.page_index }));
    }

    function sendToRight(hl: Highlight) {
        rightViewer.update(v => ({ ...v, targetPage: hl.page_index }));
    }

    async function handleDelete(hl: Highlight) {
        await deleteHighlight(hl.id);
        highlights.update(hs => hs.filter(h => h.id !== hl.id));
    }
</script>

<aside class="notes-panel">
    <h2 class="panel-title">Notes</h2>

    {#if $highlights.length === 0}
        <p class="notes-empty">No highlights yet. Select text in a viewer to create one.</p>
    {:else}
        <ul class="notes-list">
            {#each $highlights as hl (hl.id)}
                <li class="note-item">
                    <div class="note-color" style="background: {hl.color};"></div>
                    <div class="note-body">
                        <span class="note-page">p. {hl.page_index + 1}</span>
                        <span class="note-excerpt">{hl.text_excerpt || '(no text)'}</span>
                    </div>
                    <div class="note-actions">
                        <button onclick={() => sendToCenter(hl)} title="Show in center">C</button>
                        <button onclick={() => sendToRight(hl)} title="Show in right">R</button>
                        <button class="delete-btn" onclick={() => handleDelete(hl)} title="Delete highlight">✕</button>
                    </div>
                </li>
            {/each}
        </ul>
    {/if}
</aside>

<style>
.notes-panel {
    width: 260px;
    min-width: 260px;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
}

.panel-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--accent);
    padding: 12px 16px 8px;
    border-bottom: 1px solid var(--border);
    margin: 0;
    flex-shrink: 0;
}

.notes-empty {
    color: var(--text-muted);
    font-size: 12px;
    padding: 16px;
    margin: 0;
}

.notes-list {
    list-style: none;
    margin: 0;
    padding: 8px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.note-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
}

.note-item:hover { background: var(--bg-elevated); }

.note-color {
    width: 6px;
    min-width: 6px;
    height: 36px;
    border-radius: 3px;
    margin-top: 2px;
}

.note-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.note-page {
    font-size: 10px;
    color: var(--text-muted);
}

.note-excerpt {
    font-size: 12px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
}

.note-actions {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.note-actions button {
    width: 24px;
    height: 20px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text);
    cursor: pointer;
    font-size: 11px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
}

.note-actions button:hover { background: var(--bg-hover); }
.delete-btn:hover { background: #fee2e2 !important; color: var(--danger); border-color: var(--danger); }
</style>
