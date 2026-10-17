/**
 * What each way of adding a recipe is drawn with and says while the server is
 * out of reach, written once for the + (#174) and for a recipe dropped onto
 * Recipes (#204), which is the same act by another road. A photo of a page
 * and a video's link, and a PDF or pasted text on an instance with a Runner,
 * go to Kamosu's AI agent (#278, #349).
 */

import { m } from '$lib/paraglide/messages';
import type { Act } from './adding.svelte';

/** An act offered as a row of its own. Sending is the last step of three of them. */
export type Source = Exclude<Act, 'send'>;

/** Each source's icon, as the path of a 24 by 24 line drawing. */
export const ICON: Record<Source, string> = {
	link: 'M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1',
	file: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5',
	pdf: 'M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4',
	paste:
		'M9 4h6v3H9zM8 5H6a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1h-2M8 12h8M8 16h5',
	write: 'M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4',
	// A camera: its body, the bump on top, the lens (#278).
	picture:
		'M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1zM12 17a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z',
	// A screen with a play button on it (#349).
	video: 'M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zM10 9v6l5-3z',
};

/** What an act says offline instead of its label (#76). */
export const WAITS_FOR_SERVER: Record<Source, () => string> = {
	link: m.offline_waits_import_link,
	file: m.offline_waits_bring_in,
	pdf: m.offline_waits_pdf,
	write: m.offline_waits_write,
	paste: m.offline_waits_paste,
	picture: m.offline_waits_picture,
	video: m.offline_waits_video,
};

/**
 * What a source says offline. On an instance with a Runner, a PDF and pasted
 * text are sent to Kamosu's AI agent rather than read here, and say so (#278).
 */
export function waitsForServer(source: Source, runner: boolean): string {
	if (runner && source === 'pdf') return m.offline_waits_agent_pdf();
	if (runner && source === 'paste') return m.offline_waits_agent_paste();
	return WAITS_FOR_SERVER[source]();
}
