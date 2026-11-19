/**
 * The list row and the notice card chosen on #632 (`rows-E.html`,
 * `notices-G.html`): what each draws, from what a screen hands it. How they
 * look is the stylesheet's, held by `tap-area.spec.ts` and the tokens page.
 */

import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import ListRow from './ListRow.svelte';
import NoticeCard from './NoticeCard.svelte';

const words = (text: string) => createRawSnippet(() => ({ render: () => `<p>${text}</p>` }));

describe('a list row', () => {
	it('goes somewhere, with its title, its line and an arrow, and no picture where the list has none', () => {
		const { container } = render(ListRow, {
			props: { href: '/settings/signing-in', title: 'Signing in', line: 'Password, passkeys' },
		});
		const row = screen.getByRole('link', { name: /Signing in/ });
		expect(row).toHaveAttribute('href', '/settings/signing-in');
		expect(row).toHaveClass('list-row');
		expect(row).toHaveTextContent('Password, passkeys');
		expect(container.querySelector('img')).toBeNull();
		expect(row.querySelectorAll('[aria-hidden="true"]')).toHaveLength(1); // the arrow alone
	});

	it('shows its picture, as decoration the title already names', () => {
		render(ListRow, {
			props: { href: '/r', title: 'Matcha Daifuku', picture: '/api/photographs/p_1' },
		});
		const picture = screen.getByRole('link').querySelector('img');
		expect(picture).toHaveAttribute('src', '/api/photographs/p_1');
		expect(picture).toHaveAttribute('alt', '');
	});

	it('keeps an empty square where a list with pictures has none for it', () => {
		render(ListRow, { props: { href: '/r', title: 'Matcha Daifuku', picture: null } });
		const row = screen.getByRole('link');
		expect(row.querySelector('img')).toBeNull();
		expect(row.querySelectorAll('[aria-hidden="true"]')).toHaveLength(2); // the square and the arrow
	});

	it('opens something on this screen when it has no address', async () => {
		const onclick = vi.fn();
		render(ListRow, { props: { onclick, title: 'Language and measures' } });
		await fireEvent.click(screen.getByRole('button', { name: /Language and measures/ }));
		expect(onclick).toHaveBeenCalledTimes(1);
	});
});

describe('a notice card', () => {
	it('says its title and words, and hands each act to the main or a secondary button', async () => {
		const fetch = vi.fn();
		const later = vi.fn();
		render(NoticeCard, {
			props: {
				title: 'Your library isn’t on this phone yet',
				tone: 'waiting',
				actions: [
					{ label: 'Fetch it now', act: fetch, main: true },
					{ label: 'When I’m on wifi', act: later },
				],
				children: words('191 recipes and their pictures.'),
			},
		});
		const card = screen.getByRole('status', { name: 'Your library isn’t on this phone yet' });
		expect(card).toHaveClass('border-t-accent');
		expect(
			screen.getByRole('heading', { name: 'Your library isn’t on this phone yet' }),
		).toBeVisible();
		expect(card).toHaveTextContent('191 recipes and their pictures.');
		expect(screen.getByRole('button', { name: 'Fetch it now' })).toHaveClass('main-button');
		expect(screen.getByRole('button', { name: 'When I’m on wifi' })).toHaveClass('second-button');

		await fireEvent.click(screen.getByRole('button', { name: 'Fetch it now' }));
		expect(fetch).toHaveBeenCalledTimes(1);
		expect(later).not.toHaveBeenCalled();
	});

	it('reads a warning out at once, its bar and its title in beni', () => {
		render(NoticeCard, {
			props: { title: 'Kamosu went wrong', tone: 'warning', children: words('Nothing is lost.') },
		});
		expect(screen.getByRole('alert')).toHaveClass('border-t-support');
		expect(screen.getByRole('heading', { name: 'Kamosu went wrong' })).toHaveClass('text-support');
	});

	it('is plain by default, and has no heading where its words say everything', () => {
		render(NoticeCard, {
			props: { children: words('This recipe is filed as English, but it reads as French.') },
		});
		const card = screen.getByRole('status');
		expect(card).not.toHaveClass('border-t-accent');
		expect(card).not.toHaveClass('border-t-support');
		expect(screen.queryByRole('heading')).toBeNull();
	});

	it('draws an act that loses something in beni, and a busy one as busy and not pressable', () => {
		render(NoticeCard, {
			props: {
				title: 'Leave this Kitchen?',
				actions: [
					{ label: 'Leaving…', act: () => {}, loses: true, busy: true },
					{ label: 'Keep it', act: () => {}, main: true },
				],
				children: words('Its recipes stay with it.'),
			},
		});
		const leaving = screen.getByRole('button', { name: 'Leaving…' });
		expect(leaving).toHaveClass('second-button', 'loses');
		expect(leaving).toHaveAttribute('aria-busy', 'true');
		expect(leaving).toBeDisabled();
	});
});
