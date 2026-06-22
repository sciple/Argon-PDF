import { writable } from 'svelte/store';
import type { DocumentState, Highlight, ViewerState } from './types.js';

export const document = writable<DocumentState | null>(null);

export const centerViewer = writable<ViewerState>({ targetPage: 0, mode: 'fit', manualZoom: 1.0 });
export const rightViewer = writable<ViewerState>({ targetPage: 0, mode: 'fit', manualZoom: 1.0 });

export const highlights = writable<Highlight[]>([]);

export const notesOpen = writable<boolean>(false);
