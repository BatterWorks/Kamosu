/**
 * A Demo saying it is one (#407): its refusal said in the reader's own
 * Language with the way to installing, and the cleaning's hour on the
 * reader's own clock.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { OperationError } from './api/refusal';
import { m } from './paraglide/messages';
import { standIn } from './api/stand-in';
import {
	cleaningInLocalTime,
	demoRefusalOr,
	INSTALL_KAMOSU,
	learnWhetherADemo,
} from './demo.svelte';
import Refusal from './Refusal.svelte';

describe("a Demo's refusal", () => {
	it('is said in the one wording chosen for it, whatever the Core wrote', () => {
		const refused = new OperationError('edit_recipe', 'bad_request', 'The Core in English.', {
			reason: 'demo',
		});

		expect(refused.message).toBe(m.demo_refused());
		expect(refused.reason).toBe('demo');
	});

	it("leaves every other refusal in the Core's words", () => {
		const refused = new OperationError('edit_recipe', 'bad_request', 'The Core in English.', {
			reason: 'pdf_has_no_text',
		});

		expect(refused.message).toBe('The Core in English.');
	});

	it('leads to installing from where it is shown', () => {
		render(Refusal, { props: { words: m.demo_refused() } });

		const link = screen.getByRole('link', { name: m.demo_refused_install() });
		expect(link).toHaveAttribute('href', INSTALL_KAMOSU);
		expect(document.body).toHaveTextContent(m.demo_refused());
	});

	it('draws any other refusal as the text it is', () => {
		render(Refusal, { props: { words: 'That name is taken.' } });

		expect(screen.queryByRole('link')).toBeNull();
		expect(document.body).toHaveTextContent('That name is taken.');
	});
});

describe("the cleaning's hour", () => {
	it('is 04:00 UTC on the reader’s own clock, summer time and all', () => {
		const summer = new Date('2026-07-01T12:00:00Z');
		const winter = new Date('2026-01-15T12:00:00Z');

		expect(cleaningInLocalTime('en-GB', 'Europe/Amsterdam', summer)).toBe('06:00');
		expect(cleaningInLocalTime('en-GB', 'Europe/Amsterdam', winter)).toBe('05:00');
		expect(cleaningInLocalTime('en-GB', 'America/New_York', summer)).toBe('00:00');
	});
});

describe('a refusal a screen words itself', () => {
	it("is said in the Demo's words where a Demo refused", () => {
		const inDemo = new OperationError('rename_version', 'bad_request', 'x', { reason: 'demo' });
		const other = new OperationError('rename_version', 'bad_request', 'x');

		expect(demoRefusalOr(inDemo, 'The name could not be saved.')).toBe(m.demo_refused());
		expect(demoRefusalOr(other, 'The name could not be saved.')).toBe(
			'The name could not be saved.',
		);
	});
});

describe("a Demo's visits", () => {
	const STATUS = {
		version: '0.3.0',
		setup_complete: true,
		password_minimum: 15,
		passkey_origin: null,
	};
	const UMAMI = {
		script: 'https://umami.example.com/script.js',
		website_id: '94db1cb1-74f4-4a40-ad6c-962362670409',
	};
	const counters = () => document.head.querySelectorAll('script[data-website-id]');

	afterEach(() => counters().forEach((script) => script.remove()));

	it('are counted by the Umami the Core names, loaded once however often it is asked', async () => {
		const kamosu = standIn({
			instance_status: { ...STATUS, demo: true, demo_access_key: 'k', analytics: UMAMI },
		});

		await learnWhetherADemo(kamosu.client);
		await learnWhetherADemo(kamosu.client);

		expect(counters()).toHaveLength(1);
		const script = counters()[0] as HTMLScriptElement;
		expect(script.src).toBe(UMAMI.script);
		expect(script.dataset.websiteId).toBe(UMAMI.website_id);
		expect(script.defer).toBe(true);
	});

	it('are counted by nothing where the Core names no Umami', async () => {
		await learnWhetherADemo(
			standIn({ instance_status: { ...STATUS, demo: true, demo_access_key: 'k' } }).client,
		);
		await learnWhetherADemo(standIn({ instance_status: { ...STATUS, demo: false } }).client);

		expect(document.head.querySelectorAll('script')).toHaveLength(0);
	});
});
