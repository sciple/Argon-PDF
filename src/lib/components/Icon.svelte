<script lang="ts">
    // Tiny inline-SVG icon set (Lucide-style, MIT). Stroke uses currentColor so
    // icons inherit button text color and stay crisp at any display scale.
    interface Props {
        name: keyof typeof PATHS;
        size?: number;
        class?: string;
    }
    let { name, size = 16, class: cls = '' }: Props = $props();

    const PATHS = {
        'folder-open':
            '<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>',
        loader:
            '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
        'columns-2':
            '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/>',
        'panel-left':
            '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/>',
        'panel-right':
            '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/>',
        'zoom-in':
            '<circle cx="11" cy="11" r="8"/><line x1="21" x2="16.65" y1="21" y2="16.65"/><line x1="11" x2="11" y1="8" y2="14"/><line x1="8" x2="14" y1="11" y2="11"/>',
        'zoom-out':
            '<circle cx="11" cy="11" r="8"/><line x1="21" x2="16.65" y1="21" y2="16.65"/><line x1="8" x2="14" y1="11" y2="11"/>',
        'fit-width':
            '<path d="m18 8 4 4-4 4"/><path d="M2 12h20"/><path d="m6 8-4 4 4 4"/>',
    } as const;
</script>

<svg
    class={cls}
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
>
    {@html PATHS[name]}
</svg>

<style>
svg {
    display: block;
    flex-shrink: 0;
}
</style>
