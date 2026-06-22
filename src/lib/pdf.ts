import * as pdfjsLib from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
import type { PDFDocumentProxy, PDFPageProxy, RenderTask } from 'pdfjs-dist';

// Point PDF.js at its worker (bundled by Vite, served same-origin).
pdfjsLib.GlobalWorkerOptions.workerSrc = workerUrl;

export async function loadPdf(data: Uint8Array): Promise<PDFDocumentProxy> {
    return pdfjsLib.getDocument({ data }).promise;
}

// Per-document page cache so main viewer, side viewer, and thumbnails share
// already-loaded PdfPageProxy objects instead of re-fetching them.
const pageCaches = new WeakMap<PDFDocumentProxy, Map<number, Promise<PDFPageProxy>>>();

export function getPage(doc: PDFDocumentProxy, pageNumber: number): Promise<PDFPageProxy> {
    let cache = pageCaches.get(doc);
    if (!cache) {
        cache = new Map();
        pageCaches.set(doc, cache);
    }
    let p = cache.get(pageNumber);
    if (!p) {
        p = doc.getPage(pageNumber);
        cache.set(pageNumber, p);
    }
    return p;
}

export function outputScale(): number {
    return Math.min(window.devicePixelRatio || 1, 3);
}

// Track the in-flight render per canvas so a re-render (zoom/resize) cancels the
// previous one — PDF.js throws if two renders target the same canvas at once.
const renderTasks = new WeakMap<HTMLCanvasElement, RenderTask>();

/**
 * Render `page` into `canvas` at the given CSS scale, at device-pixel resolution
 * (crisp on HiDPI). Cancels any previous render of the same canvas.
 */
// ── Text search ───────────────────────────────────────────────────────────────

export interface PageTextIndex {
    fullText: string;
    items: string[];
    offsets: number[];
}

export interface TextMatch {
    pageIndex: number;
    start: number;
    end: number;
}

const textIdxCache = new WeakMap<PDFDocumentProxy, Map<number, Promise<PageTextIndex>>>();

export async function getPageTextIndex(doc: PDFDocumentProxy, pageIndex: number): Promise<PageTextIndex> {
    let byPage = textIdxCache.get(doc);
    if (!byPage) { byPage = new Map(); textIdxCache.set(doc, byPage); }
    let p = byPage.get(pageIndex);
    if (!p) {
        p = getPage(doc, pageIndex + 1).then(async (page) => {
            const content = await page.getTextContent();
            let fullText = '';
            const items: string[] = [];
            const offsets: number[] = [];
            for (const raw of content.items) {
                if ('str' in raw) {
                    offsets.push(fullText.length);
                    items.push(raw.str);
                    fullText += raw.str;
                }
            }
            return { fullText, items, offsets };
        });
        byPage.set(pageIndex, p);
    }
    return p;
}

export async function searchAllPages(
    doc: PDFDocumentProxy,
    numPages: number,
    query: string,
): Promise<TextMatch[]> {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    const results: TextMatch[] = [];
    for (let i = 0; i < numPages; i++) {
        const idx = await getPageTextIndex(doc, i);
        const lower = idx.fullText.toLowerCase();
        let pos = 0;
        for (;;) {
            const at = lower.indexOf(q, pos);
            if (at === -1) break;
            results.push({ pageIndex: i, start: at, end: at + q.length });
            pos = at + 1;
        }
    }
    return results;
}

// ─────────────────────────────────────────────────────────────────────────────

export async function renderPageToCanvas(
    page: PDFPageProxy,
    canvas: HTMLCanvasElement,
    cssScale: number,
): Promise<void> {
    const prev = renderTasks.get(canvas);
    if (prev) {
        try { prev.cancel(); } catch { /* ignore */ }
    }

    const scale = outputScale();
    const viewport = page.getViewport({ scale: cssScale });

    canvas.width = Math.floor(viewport.width * scale);
    canvas.height = Math.floor(viewport.height * scale);
    canvas.style.width = Math.floor(viewport.width) + 'px';
    canvas.style.height = Math.floor(viewport.height) + 'px';

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const transform = scale !== 1 ? [scale, 0, 0, scale, 0, 0] : undefined;
    const task = page.render({ canvasContext: ctx, viewport, transform });
    renderTasks.set(canvas, task);

    try {
        await task.promise;
    } catch (e: unknown) {
        // RenderingCancelledException is expected when superseded — swallow it.
        const name = (e as { name?: string })?.name;
        if (name !== 'RenderingCancelledException') throw e;
    } finally {
        if (renderTasks.get(canvas) === task) renderTasks.delete(canvas);
    }
}
