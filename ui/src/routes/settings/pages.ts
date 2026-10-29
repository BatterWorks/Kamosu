/**
 * Settings' menu: four groups by whose a thing is, and a page for each job
 * (#469, option C of 26 October 2026).
 *
 * It was seven pages that mixed what is yours, what is the household's and
 * what is the instance's, and the Operator screen beside them behind a link.
 * People could not tell which part was theirs. Now *You*, *Your household*
 * and *Add recipes* are everybody's, and *This instance* is the Operator's
 * and nobody else's (ADR 0040). Each row is a name and a short line saying
 * what is inside. The prototype is `.dev/prototypes/469-settings/`, its
 * `c-menu-phone.html` and `c-menu-macbook.html`.
 *
 * Foods and What has been brought in keep their own screens at `/foods` and
 * `/imports`, since each is a list with what is open beside it (#199).
 */

import { m } from '$lib/paraglide/messages';
import { KEPT_FOR_DAYS } from '$lib/security-record';
import { appsInBrief } from '$lib/other-apps';
import { demo } from '$lib/demo.svelte';

export interface PageEntry {
	href: string;
	title: () => string;
	/** What the page holds, in one line under its name on the menu. */
	inside: () => string;
	/** Whether the menu offers it here, where it is not offered everywhere. */
	offered?: () => boolean;
}

export interface Group {
	heading: () => string;
	/** Shown to an Operator alone, as every Operator surface is (ADR 0040). */
	operator?: boolean;
	entries: PageEntry[];
}

/** Your name, which the card at the top of the menu opens. */
export const NAME_PAGE = '/settings/name';

/** The page the wide layout opens beside the menu when none is named. */
export const FIRST_PAGE = '/settings/signing-in';

export const GROUPS: Group[] = [
	{
		heading: m.settings_group_you,
		entries: [
			{ href: FIRST_PAGE, title: m.settings_signing_in, inside: m.settings_signing_in_inside },
			{ href: '/settings/keys', title: m.settings_keys, inside: m.settings_keys_inside },
			{
				href: '/settings/language',
				title: m.settings_language_page,
				inside: m.settings_language_inside,
			},
			{ href: '/settings/device', title: m.settings_device, inside: m.settings_device_inside },
			{
				href: '/settings/security',
				title: m.settings_security_record,
				inside: () => m.settings_security_record_inside({ days: KEPT_FOR_DAYS }),
			},
		],
	},
	{
		heading: m.settings_group_household,
		entries: [
			{
				href: '/settings/kitchens',
				title: m.settings_kitchens_page,
				inside: m.settings_kitchens_inside,
			},
			{
				href: '/settings/cookbook',
				title: m.settings_cookbook,
				inside: m.settings_cookbook_inside,
			},
			{ href: '/settings/tags', title: m.settings_tags, inside: m.settings_tags_inside },
			{ href: '/foods', title: m.settings_foods, inside: m.settings_foods_inside },
		],
	},
	{
		heading: m.settings_group_add,
		entries: [
			{ href: '/settings/import', title: m.settings_import_page, inside: appsInBrief },
			// A Demo refuses imports, so it does not offer the page (ADR 0055).
			{
				href: '/settings/share',
				title: m.settings_share_page,
				inside: m.settings_share_inside,
				offered: () => !demo.is,
			},
			{ href: '/imports', title: m.settings_brought_in, inside: m.settings_brought_in_inside },
			// What went to the agent and how it ended (#463). A Demo refuses
			// what a visitor sends it, so it has nothing to list.
			{
				href: '/settings/agent',
				title: m.settings_sent_page,
				inside: m.settings_sent_inside,
				offered: () => !demo.is,
			},
		],
	},
	{
		heading: m.settings_group_instance,
		operator: true,
		entries: [
			{ href: '/settings/people', title: m.settings_people, inside: m.settings_people_inside },
			{ href: '/settings/backups', title: m.settings_backups, inside: m.settings_backups_inside },
			{ href: '/settings/runner', title: m.settings_runner, inside: m.settings_runner_inside },
			{ href: '/settings/address', title: m.settings_address, inside: m.settings_address_inside },
			{ href: '/settings/meaning', title: m.settings_meaning, inside: m.settings_meaning_inside },
			{ href: '/settings/upkeep', title: m.settings_upkeep, inside: m.settings_upkeep_inside },
			{
				href: '/settings/instance-security',
				title: m.settings_instance_record,
				inside: () => m.settings_instance_record_inside({ days: KEPT_FOR_DAYS }),
			},
		],
	},
];

/** At the foot of the menu, outside the groups, for everyone. */
export const ABOUT: PageEntry = {
	href: '/settings/about',
	title: m.settings_about,
	inside: m.settings_about_inside,
};
