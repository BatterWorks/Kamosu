/**
 * What Android's Share menu answers (#452, option A): the page the service
 * worker sends a share to, drawn from what its address carries.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import type { SharedAnswer } from '$lib/offline/shared';

import Shared from './Shared.svelte';

function openAt(answer: SharedAnswer | undefined) {
	render(Shared, { props: { answer } });
}

describe('the page after sharing', () => {
	afterEach(cleanup);

	it('says what Kamosu said, and offers Kamosu or the way back', () => {
		openAt({
			outcome: 'sent',
			message:
				"Sent. Kamosu's AI agent is reading it, and the recipe will be on your shelf in a few minutes.",
		});
		expect(screen.getByRole('heading', { level: 1, name: 'Sent' })).toBeInTheDocument();
		expect(screen.getByRole('status')).toHaveTextContent("Kamosu's AI agent is reading it");
		expect(screen.getByRole('link', { name: 'Open Kamosu' })).toHaveAttribute('href', '/');
		expect(screen.getByText('Or press Back to return to where you were.')).toBeInTheDocument();
	});

	it('opens the recipe that was added, or found already on the shelf', () => {
		openAt({
			outcome: 'already',
			message: 'Already on your shelf: Lemon tart',
			branchId: 'b_1',
			title: 'Lemon tart',
		});
		expect(screen.getByRole('heading', { level: 1, name: 'Already there' })).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'Open Lemon tart' })).toHaveAttribute(
			'href',
			'/recipes/b_1',
		);
	});

	it('says nothing came where nothing was kept for it, whatever its address says', () => {
		openAt(undefined);
		expect(screen.getByRole('heading', { level: 1, name: 'Not sent' })).toBeInTheDocument();
		expect(screen.getByRole('status')).toHaveTextContent('Nothing came with the share.');
	});

	it('says offline that nothing was sent', () => {
		openAt({ outcome: 'offline' });
		expect(screen.getByRole('heading', { level: 1, name: 'Not sent' })).toBeInTheDocument();
		expect(screen.getByRole('status')).toHaveTextContent(
			"Kamosu couldn't be reached, so nothing was sent.",
		);
	});
});
