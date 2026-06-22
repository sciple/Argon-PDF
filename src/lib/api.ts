import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export async function openDocumentDialog(): Promise<string | null> {
    const selected = await open({
        multiple: false,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
    });
    return typeof selected === 'string' ? selected : null;
}

/** Read a PDF file's raw bytes via the Rust `read_pdf` command (ArrayBuffer IPC). */
export async function readPdf(path: string): Promise<Uint8Array> {
    const buf = await invoke<ArrayBuffer>('read_pdf', { path });
    return new Uint8Array(buf);
}

/** Best-effort file name from a full path (for display). */
export function baseName(path: string): string {
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] || path;
}
