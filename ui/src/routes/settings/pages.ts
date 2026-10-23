/**
 * Settings' pages (#221, option 2 of the audit of 9 October 2026).
 *
 * Settings was one page 8,000px tall on a phone, with fourteen sections. It
 * is a menu of rows now, each opening a page of its own that holds the
 * sections it names. The sections' own words did not change; this is where
 * they live, not what they say. The fifth, the Security Record, came with
 * #371 (choice A of 20 October 2026), and Import from another app, first on
 * the menu, with #412 (option B of 23 October 2026).
 */

import { m } from '$lib/paraglide/messages';
import { KEPT_FOR_DAYS } from '$lib/security-record';
import { appsInBrief } from '$lib/other-apps';

export type SettingsPage = 'import' | 'account' | 'device' | 'kitchens' | 'instance' | 'security';

export interface PageEntry {
	page: SettingsPage;
	href: string;
	title: () => string;
	/** What the page holds, in one line under its name on the menu. */
	inside: () => string;
}

export const PAGES: PageEntry[] = [
	// Its own page, first on the menu (#412, option B of 23 October 2026):
	// it was the Crouton button on This instance.
	{
		page: 'import',
		href: '/settings/import',
		title: m.settings_import_page,
		inside: appsInBrief,
	},
	{
		page: 'account',
		href: '/settings/account',
		title: m.settings_account,
		inside: m.settings_account_inside,
	},
	{
		page: 'device',
		href: '/settings/device',
		title: m.settings_device,
		inside: m.settings_device_inside,
	},
	{
		page: 'kitchens',
		href: '/settings/kitchens',
		title: m.settings_kitchens_page,
		inside: m.settings_kitchens_inside,
	},
	{
		page: 'instance',
		href: '/settings/instance',
		title: m.settings_instance,
		inside: m.settings_instance_inside,
	},
	{
		page: 'security',
		href: '/settings/security',
		title: m.settings_security_record,
		inside: () => m.settings_security_record_inside({ days: KEPT_FOR_DAYS }),
	},
];

export const titleOf = (page: SettingsPage): string =>
	PAGES.find((entry) => entry.page === page)!.title();
