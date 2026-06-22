import { writable } from 'svelte/store';
import type { PDFDocumentProxy } from 'pdfjs-dist';
import type { ViewerState } from './types.js';

export interface PdfState {
    proxy: PDFDocumentProxy;
    numPages: number;
    /** Page-1 size at scale 1 (PDF points). Used as the layout size for all
     *  pages — maths textbooks are uniform; mixed-size docs still render each
     *  page correctly, only the slot box may differ slightly. */
    defaultWidth: number;
    defaultHeight: number;
    fileName: string;
}

export const pdfDoc = writable<PdfState | null>(null);

export const mainViewer = writable<ViewerState>({ targetPage: 0, mode: 'fit', manualZoom: 1.0 });
export const sideViewer = writable<ViewerState>({ targetPage: 0, mode: 'fit', manualZoom: 1.0 });

export const sideOpen = writable<boolean>(true);
