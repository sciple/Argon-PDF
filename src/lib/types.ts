export type ZoomMode = 'fit' | 'manual';

export interface ViewerState {
    /** 0-based page index targeted by page-box / thumbnail clicks */
    targetPage: number;
    /** 'fit' = auto-scale pages to pane width; 'manual' = use manualZoom */
    mode: ZoomMode;
    /** Zoom factor applied only in 'manual' mode (1.0 = 100%) */
    manualZoom: number;
}
