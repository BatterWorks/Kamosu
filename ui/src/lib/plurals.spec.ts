/**
 * The phrases a count of one reaches with no word of its own at the call site
 * (#533). Each says one thing in the singular and two in the plural, in every
 * Reading Language, and keeps the rest of what it was handed.
 */

import { describe, expect, it } from 'vitest';
import { m } from '#lib/paraglide/messages.js';

type Locale = 'en' | 'fr' | 'es' | 'nl';

const SAID = {
	en: [
		[
			(count: number, locale: Locale) => m.divergence_put_away({ count }, { locale }),
			'Just the recipe. 1 difference put away.',
			'Just the recipe. 2 differences put away.',
		],
		[
			(count: number, locale: Locale) => m.report_relate({ count }, { locale }),
			'Relate 1 pair',
			'Relate 2 pairs',
		],
		[
			(count: number, locale: Locale) => m.report_site_icons({ count }, { locale }),
			"Left out: 1 website icon, the site's logo rather than the dish.",
			"Left out: 2 website icons, the site's logo rather than the dish.",
		],
		[
			(count: number, locale: Locale) =>
				m.offline_library_body({ count, size: '6 KB' }, { locale }),
			'1 recipe and its pictures, about 6 KB. Best on wifi.',
			'2 recipes and their pictures, about 6 KB. Best on wifi.',
		],
		[
			(count: number, locale: Locale) => m.offline_phone_kept({ count, date: 'today' }, { locale }),
			'1 recipe on this phone, updated today.',
			'2 recipes on this phone, updated today.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_phone_kept_device({ count, date: 'today' }, { locale }),
			'1 recipe on this device, updated today.',
			'2 recipes on this device, updated today.',
		],
		[
			(count: number, locale: Locale) => m.report_lede_pairs({ count }, { locale }),
			'1 pair might be related.',
			'2 pairs might be related.',
		],
		[
			(count: number, locale: Locale) => m.settings_meaning_catching_up({ count }, { locale }),
			'1 recipe changed since the last index. Kamosu catches up within a minute.',
			'2 recipes changed since the last index. Kamosu catches up within a minute.',
		],
	],
	fr: [
		[
			(count: number, locale: Locale) => m.divergence_put_away({ count }, { locale }),
			'La recette seule. 1 différence rangée.',
			'La recette seule. 2 différences rangées.',
		],
		[
			(count: number, locale: Locale) => m.report_relate({ count }, { locale }),
			'Lier 1 paire',
			'Lier 2 paires',
		],
		[
			(count: number, locale: Locale) => m.report_lede_unreadable({ count }, { locale }),
			"1 n'a pas pu être lue.",
			"2 n'ont pas pu être lues.",
		],
		[
			(count: number, locale: Locale) => m.report_bare({ count, titles: 'Soupe' }, { locale }),
			'1 est arrivée avec seulement un nom et un lien : Soupe.',
			'2 sont arrivées avec seulement un nom et un lien : Soupe.',
		],
		[
			(count: number, locale: Locale) => m.report_site_icons({ count }, { locale }),
			'Laissée de côté : 1 icône de site, le logo du site plutôt que le plat.',
			'Laissées de côté : 2 icônes de sites, le logo du site plutôt que le plat.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_library_body({ count, size: '6 Ko' }, { locale }),
			'1 recette et ses photos, environ 6 Ko. Mieux vaut le wifi.',
			'2 recettes et leurs photos, environ 6 Ko. Mieux vaut le wifi.',
		],
		[
			(count: number, locale: Locale) => m.offline_phone_kept({ count, date: 'hier' }, { locale }),
			'1 recette sur ce téléphone, à jour le hier.',
			'2 recettes sur ce téléphone, à jour le hier.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_phone_kept_device({ count, date: 'hier' }, { locale }),
			'1 recette sur cet appareil, à jour le hier.',
			'2 recettes sur cet appareil, à jour le hier.',
		],
		[
			(count: number, locale: Locale) => m.report_lede_pairs({ count }, { locale }),
			'1 paire est peut-être liée.',
			'2 paires sont peut-être liées.',
		],
		[
			(count: number, locale: Locale) => m.settings_meaning_catching_up({ count }, { locale }),
			'1 recette a changé depuis le dernier index. Kamosu se met à jour dans la minute.',
			'2 recettes ont changé depuis le dernier index. Kamosu se met à jour dans la minute.',
		],
	],
	es: [
		[
			(count: number, locale: Locale) => m.divergence_put_away({ count }, { locale }),
			'Solo la receta. 1 diferencia guardada.',
			'Solo la receta. 2 diferencias guardadas.',
		],
		[
			(count: number, locale: Locale) => m.report_relate({ count }, { locale }),
			'Relacionar 1 par',
			'Relacionar 2 pares',
		],
		[
			(count: number, locale: Locale) => m.report_lede_unreadable({ count }, { locale }),
			'1 no se pudo leer.',
			'2 no se pudieron leer.',
		],
		[
			(count: number, locale: Locale) => m.report_bare({ count, titles: 'Sopa' }, { locale }),
			'1 llegó solo con un nombre y un enlace: Sopa.',
			'2 llegaron solo con un nombre y un enlace: Sopa.',
		],
		[
			(count: number, locale: Locale) => m.report_site_icons({ count }, { locale }),
			'Dejado fuera: 1 icono de sitio web, el logo del sitio y no el plato.',
			'Dejados fuera: 2 iconos de sitios web, el logo del sitio y no el plato.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_library_body({ count, size: '6 KB' }, { locale }),
			'1 receta y sus fotos, unos 6 KB. Mejor con wifi.',
			'2 recetas y sus fotos, unos 6 KB. Mejor con wifi.',
		],
		[
			(count: number, locale: Locale) => m.offline_phone_kept({ count, date: 'hoy' }, { locale }),
			'1 receta en este teléfono, actualizada el hoy.',
			'2 recetas en este teléfono, actualizadas el hoy.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_phone_kept_device({ count, date: 'hoy' }, { locale }),
			'1 receta en este dispositivo, actualizada el hoy.',
			'2 recetas en este dispositivo, actualizadas el hoy.',
		],
		[
			(count: number, locale: Locale) => m.report_lede_pairs({ count }, { locale }),
			'1 par podría estar relacionado.',
			'2 pares podrían estar relacionados.',
		],
		[
			(count: number, locale: Locale) => m.settings_meaning_catching_up({ count }, { locale }),
			'1 receta ha cambiado desde el último índice. Kamosu se pone al día en un minuto.',
			'2 recetas han cambiado desde el último índice. Kamosu se pone al día en un minuto.',
		],
	],
	nl: [
		[
			(count: number, locale: Locale) => m.divergence_put_away({ count }, { locale }),
			'Alleen het recept. 1 verschil opgeborgen.',
			'Alleen het recept. 2 verschillen opgeborgen.',
		],
		[
			(count: number, locale: Locale) => m.report_relate({ count }, { locale }),
			'1 paar koppelen',
			'2 paren koppelen',
		],
		[
			(count: number, locale: Locale) => m.report_lede_unreadable({ count }, { locale }),
			'1 kon niet worden gelezen.',
			'2 konden niet worden gelezen.',
		],
		[
			(count: number, locale: Locale) => m.report_bare({ count, titles: 'Soep' }, { locale }),
			'1 kwam binnen als alleen een naam en een link: Soep.',
			'2 kwamen binnen als alleen een naam en een link: Soep.',
		],
		[
			(count: number, locale: Locale) =>
				m.offline_phone_kept({ count, date: 'vandaag' }, { locale }),
			'1 recept op deze telefoon, bijgewerkt vandaag.',
			'2 recepten op deze telefoon, bijgewerkt vandaag.',
		],
	],
} as const;

describe('a count of one (#533)', () => {
	it.each(Object.entries(SAID))('is said in the singular in %s', (locale, phrases) => {
		for (const [say, one, two] of phrases) {
			expect(say(1, locale as Locale)).toBe(one);
			expect(say(2, locale as Locale)).toBe(two);
		}
	});
});
