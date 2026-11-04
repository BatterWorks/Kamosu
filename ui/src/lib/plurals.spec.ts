/**
 * The phrases a count of one reaches with no word of its own at the call site
 * (#533). Each says one thing in the singular and two in the plural, in every
 * Reading Language, and keeps the rest of what it was handed.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { m } from '#lib/paraglide/messages.js';
import { setLocale } from '#lib/paraglide/runtime.js';

afterEach(() => setLocale('en', { reload: false }));

const SAID = {
	en: [
		[
			(count: number) => m.report_site_icons({ count }),
			"Left out: 1 website icon, the site's logo rather than the dish.",
			"Left out: 2 website icons, the site's logo rather than the dish.",
		],
		[
			(count: number) => m.offline_library_body({ count, size: '6 KB' }),
			'1 recipe and its pictures, about 6 KB. Best on wifi.',
			'2 recipes and their pictures, about 6 KB. Best on wifi.',
		],
		[
			(count: number) => m.offline_phone_kept({ count, date: 'today' }),
			'1 recipe on this phone, updated today.',
			'2 recipes on this phone, updated today.',
		],
		[
			(count: number) => m.offline_phone_kept_device({ count, date: 'today' }),
			'1 recipe on this device, updated today.',
			'2 recipes on this device, updated today.',
		],
		[
			(count: number) => m.report_lede_pairs({ count }),
			'1 pair might be related.',
			'2 pairs might be related.',
		],
		[
			(count: number) => m.settings_meaning_catching_up({ count }),
			'1 recipe changed since the last index. Kamosu catches up within a minute.',
			'2 recipes changed since the last index. Kamosu catches up within a minute.',
		],
	],
	fr: [
		[
			(count: number) => m.report_site_icons({ count }),
			'Laissée de côté : 1 icône de site, le logo du site plutôt que le plat.',
			'Laissées de côté : 2 icônes de sites, le logo du site plutôt que le plat.',
		],
		[
			(count: number) => m.offline_library_body({ count, size: '6 Ko' }),
			'1 recette et ses photos, environ 6 Ko. Mieux vaut le wifi.',
			'2 recettes et leurs photos, environ 6 Ko. Mieux vaut le wifi.',
		],
		[
			(count: number) => m.offline_phone_kept({ count, date: 'hier' }),
			'1 recette sur ce téléphone, à jour le hier.',
			'2 recettes sur ce téléphone, à jour le hier.',
		],
		[
			(count: number) => m.offline_phone_kept_device({ count, date: 'hier' }),
			'1 recette sur cet appareil, à jour le hier.',
			'2 recettes sur cet appareil, à jour le hier.',
		],
		[
			(count: number) => m.report_lede_pairs({ count }),
			'1 paire est peut-être liée.',
			'2 paires sont peut-être liées.',
		],
		[
			(count: number) => m.settings_meaning_catching_up({ count }),
			'1 recette a changé depuis le dernier index. Kamosu se met à jour dans la minute.',
			'2 recettes ont changé depuis le dernier index. Kamosu se met à jour dans la minute.',
		],
	],
	es: [
		[
			(count: number) => m.report_site_icons({ count }),
			'Dejado fuera: 1 icono de sitio web, el logo del sitio y no el plato.',
			'Dejados fuera: 2 iconos de sitios web, el logo del sitio y no el plato.',
		],
		[
			(count: number) => m.offline_library_body({ count, size: '6 KB' }),
			'1 receta y sus fotos, unos 6 KB. Mejor con wifi.',
			'2 recetas y sus fotos, unos 6 KB. Mejor con wifi.',
		],
		[
			(count: number) => m.offline_phone_kept({ count, date: 'hoy' }),
			'1 receta en este teléfono, actualizada el hoy.',
			'2 recetas en este teléfono, actualizadas el hoy.',
		],
		[
			(count: number) => m.offline_phone_kept_device({ count, date: 'hoy' }),
			'1 receta en este dispositivo, actualizada el hoy.',
			'2 recetas en este dispositivo, actualizadas el hoy.',
		],
		[
			(count: number) => m.report_lede_pairs({ count }),
			'1 par podría estar relacionado.',
			'2 pares podrían estar relacionados.',
		],
		[
			(count: number) => m.settings_meaning_catching_up({ count }),
			'1 receta ha cambiado desde el último índice. Kamosu se pone al día en un minuto.',
			'2 recetas han cambiado desde el último índice. Kamosu se pone al día en un minuto.',
		],
	],
} as const;

describe('a count of one (#533)', () => {
	it.each(Object.entries(SAID))('is said in the singular in %s', (locale, phrases) => {
		setLocale(locale as keyof typeof SAID, { reload: false });
		for (const [say, one, two] of phrases) {
			expect(say(1)).toBe(one);
			expect(say(2)).toBe(two);
		}
	});
});
