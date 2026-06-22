export interface PageSize {
    width_pts: number;
    height_pts: number;
}

export interface DocumentState {
    doc_id: string;
    page_count: number;
    has_text_layer: boolean;
    page_sizes: PageSize[];
}

export interface ViewerState {
    /** 0-based page index currently targeted by thumbnail/note clicks */
    targetPage: number;
    zoom: number;
}

export interface HighlightRect {
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface Highlight {
    id: number;
    doc_id: string;
    page_index: number;
    start_char: number;
    end_char: number;
    rects: HighlightRect[];
    text_excerpt: string;
    color: string;
}

export interface CharRect {
    char_index: number;
    x: number;
    y: number;
    width: number;
    height: number;
    char_val: string;
}

export interface TextLayout {
    chars: CharRect[];
}

export type AppErrorType =
    | { type: 'Pdf'; message: string }
    | { type: 'DocNotFound'; message: string }
    | { type: 'Db'; message: string }
    | { type: 'Io'; message: string }
    | { type: 'NeedsPassword' }
    | { type: 'InvalidPdf' };
