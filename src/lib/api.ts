import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { DocumentState, TextLayout, Highlight } from './types.js';

export function renderUrl(doc_id: string, page: number, scale_pct: number): string {
    // Tauri serves custom URI schemes differently per platform:
    //   Windows/Android: http://<scheme>.localhost/<path>
    //   macOS/Linux:     <scheme>://localhost/<path>
    // Our path segments are URL-safe (hex hash + integers) so no encoding needed.
    const path = `render/${doc_id}/${page}/${scale_pct}`;
    const isWindows = navigator.userAgent.includes('Windows');
    return isWindows
        ? `http://argon.localhost/${path}`
        : `argon://localhost/${path}`;
}

export async function openDocumentDialog(): Promise<string | null> {
    const selected = await open({
        multiple: false,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    if (typeof selected === 'string') return selected;
    return null;
}

export async function openDocument(path: string): Promise<DocumentState> {
    return invoke<DocumentState>('open_document', { path });
}

export async function getPageTextLayout(doc_id: string, page_index: number): Promise<TextLayout> {
    return invoke<TextLayout>('get_page_text_layout', { docId: doc_id, pageIndex: page_index });
}

export async function addHighlight(
    doc_id: string,
    page_index: number,
    start_char: number,
    end_char: number,
    color?: string,
): Promise<Highlight> {
    return invoke<Highlight>('add_highlight', { docId: doc_id, pageIndex: page_index, startChar: start_char, endChar: end_char, color });
}

export async function listHighlights(doc_id: string): Promise<Highlight[]> {
    return invoke<Highlight[]>('list_highlights', { docId: doc_id });
}

export async function deleteHighlight(id: number): Promise<void> {
    return invoke('delete_highlight', { id });
}
