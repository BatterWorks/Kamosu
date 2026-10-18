/**
 * The agent's house notes as the Runner section's row and the sheet it opens
 * say them (#344).
 */

import type { GetHouseNotesOutput } from '$lib/api/catalogue';
import { howLongAgo } from '$lib/dates';
import { m } from '$lib/paraglide/messages';

/** Who saved the notes last and when, or the empty string if nobody has. */
export function savedLine(notes: GetHouseNotesOutput): string {
	if (!notes.saved_at) return '';
	const when = howLongAgo(notes.saved_at);
	return notes.saved_by
		? m.operator_notes_saved({ who: notes.saved_by, when })
		: m.operator_notes_saved_when({ when });
}

/** The notes' first line with words on it, which the row shows. */
export const firstLine = (notes: string) =>
	notes.split('\n').find((line) => line.trim() !== '') ?? notes;
