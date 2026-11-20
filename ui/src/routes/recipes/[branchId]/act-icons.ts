/**
 * The icons of a recipe's acts (#649): each one path, drawn at 22px in the
 * current colour, as the navigation's are (`places.ts`), with no icon
 * library. Every act also carries its word (U8).
 */

import { places } from '#lib/shell/places.js';

function placePath(href: string): string {
	const path = places.find((place) => place.href === href)?.path;
	if (!path) throw new Error(`No place at ${href} to take an icon from`);
	return path;
}

export const ACT_ICONS = {
	plan: placePath('/plan'),
	shopping: placePath('/shopping'),
	onList: 'M20 6 9 17l-5-5',
	share: 'M12 15V3M7 8l5-5 5 5M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7',
	more: 'M5 11a1 1 0 1 1 0 2 1 1 0 0 1 0-2ZM12 11a1 1 0 1 1 0 2 1 1 0 0 1 0-2ZM19 11a1 1 0 1 1 0 2 1 1 0 0 1 0-2Z',
	edit: 'M12 20h9M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z',
	history: 'M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8M3 3v5h5M12 7v5l4 2',
	translate: 'm5 8 6 6M4 14l6-6 2-3M2 5h12M7 2h1M22 22l-5-10-5 10M14 18h6',
	print:
		'M6 9V2h12v7M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2M6 14h12v8H6Z',
	name: 'M4 7V4h16v3M9 20h6M12 4v16',
	delete: 'M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2',
	putBack: 'M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11',
	copy: 'M8 8h12v12H8ZM4 16V4h12',
	variation:
		'M6 3a3 3 0 1 1 0 6 3 3 0 0 1 0-6ZM18 3a3 3 0 1 1 0 6 3 3 0 0 1 0-6ZM12 15a3 3 0 1 1 0 6 3 3 0 0 1 0-6ZM6 9v1a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V9M12 12v3',
} as const;

export type ActIconName = keyof typeof ACT_ICONS;
