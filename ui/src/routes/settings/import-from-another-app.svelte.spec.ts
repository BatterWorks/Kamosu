/**
 * The screen-seam test: Import from another app, on its own Settings page
 * (#412, option B of 23 October 2026).
 *
 * The file travels out of band (ADR 0001), so the test stands in for the
 * upload and checks what happens around it: `import_from_another_app` is
 * asked with the id it answers, and the page waits for the file to be
 * recognised. A recognised file opens the Report; a file turned away is said
 * on the page, in the screen's own words for each reason.
 */

import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import ImportTestHarness from './ImportFromAnotherAppTestHarness.svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import { OperationError } from '$lib/api/client';
import type { GetJobOutput, ImportFromAnotherAppOutput } from '$lib/api/catalogue';
import { went } from '../../testing/navigation';
import { carrying } from '../../testing/drops';

const zip = () => new File(['PK'], 'Recipes 2026-10-20.zip', { type: 'application/zip' });

const job = (over: Partial<GetJobOutput> = {}): GetJobOutput => ({
	id: 'j_7',
	operation: 'import_from_another_app',
	status: 'running',
	progress: { done: 0, total: null, message: null },
	result: null,
	error: null,
	errorCode: null,
	created_at: '2026-10-23T10:00:00.000Z',
	updated_at: '2026-10-23T10:00:00.000Z',
	...over,
});

const refusedWith = (refused: ImportFromAnotherAppOutput['refused']) =>
	job({
		status: 'completed',
		progress: { done: 0, total: null, message: null },
		result: { refused },
	});

function drawn(answers: Answers) {
	const kamosu = standIn({
		list_jobs: { jobs: [] },
		import_from_another_app: { job_id: 'j_7' },
		runner_status: { has_runner: false, last_asked_at: null },
		...answers,
	});
	const upload = vi.fn(async () => 'u_staged');
	render(ImportTestHarness, { props: { client: kamosu.client, upload } });
	return { kamosu, upload };
}

async function choose() {
	await fireEvent.change(screen.getByLabelText('Choose an export file'), {
		target: { files: [zip()] },
	});
}

describe('importing from another app', () => {
	it('names only the apps Kamosu reads, with where each keeps its export', () => {
		drawn({});
		expect(screen.getByRole('heading', { name: 'The apps Kamosu reads' })).toBeInTheDocument();
		expect(screen.getByText('Crouton')).toBeInTheDocument();
		expect(screen.getByText(/whole-library export, a zip of \.crumb files/)).toBeInTheDocument();
		expect(screen.getByText('Mealie')).toBeInTheDocument();
		expect(screen.getByText(/Data Management › Recipes › Export/)).toBeInTheDocument();
		expect(screen.getByText('Paprika')).toBeInTheDocument();
		expect(screen.getByText(/\.paprikarecipes file: File › Export/)).toBeInTheDocument();
		expect(screen.getByText('Tandoor')).toBeInTheDocument();
		expect(screen.getByText(/Settings › Space › Export, of the Default type/)).toBeInTheDocument();
		expect(screen.getByText('Nextcloud Cookbook')).toBeInTheDocument();
		expect(screen.getByText(/folder of recipes in Nextcloud Files/)).toBeInTheDocument();
		expect(screen.getByText('RecipeSage')).toBeInTheDocument();
		expect(screen.getByText(/Export Recipe Data › Export to JSON-LD/)).toBeInTheDocument();
		expect(screen.getByText('Recipya')).toBeInTheDocument();
		expect(screen.getByText(/Settings › Data › Export data, as JSON/)).toBeInTheDocument();
		expect(screen.getByText('CookCLI')).toBeInTheDocument();
		expect(screen.getByText(/cook recipe -f schema/)).toBeInTheDocument();
		expect(screen.getByText('Bar Assistant')).toBeInTheDocument();
		expect(screen.getByText(/JSON \(Schema draft 2\) or Schema\.org Recipe/)).toBeInTheDocument();
		for (const later of ['KitchenOwl', 'Recipe Keeper', 'Mela']) {
			expect(screen.queryByText(new RegExp(later))).not.toBeInTheDocument();
		}
		expect(screen.getByRole('link', { name: 'Ask for it on GitHub' })).toHaveAttribute(
			'href',
			'https://github.com/BatterWorks/Kamosu/issues',
		);
	});

	it('sends the file, asks with its id, and opens the Report once the file is recognised', async () => {
		let reads = 0;
		const { kamosu, upload } = drawn({
			// Still looking, then counting the recipes: recognised.
			get_job: () =>
				++reads < 2 ? job() : job({ progress: { done: 0, total: 86, message: '0 of 86' } }),
		});
		await choose();

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
		expect(upload).toHaveBeenCalledOnce();
		expect(
			kamosu.calls.find((call) => call.operation === 'import_from_another_app')?.input,
		).toEqual({ upload_id: 'u_staged' });
		expect(reads).toBe(2);
	});

	it('says a file no app made is refused, on the page, with the ways one recipe comes in', async () => {
		drawn({ get_job: refusedWith({ reason: 'unrecognised', sentence: 'not known' }) });
		await choose();

		const card = await screen.findByRole('alert');
		expect(card).toHaveTextContent('Recipes 2026-10-20.zip');
		expect(card).toHaveTextContent('Kamosu could not tell which app made this file.');
		expect(card).toHaveTextContent(
			'It reads exports from Mealie, Paprika, Tandoor, Crouton, Nextcloud Cookbook, RecipeSage, Recipya, CookCLI and Bar Assistant.',
		);
		expect(screen.getByRole('link', { name: 'A PDF' })).toHaveAttribute('href', '/recipes?add');
		expect(screen.getByRole('link', { name: 'Pasted text' })).toHaveAttribute(
			'href',
			'/recipes?add=paste',
		);
		// No Runner, so no photo of a page: the + offers none either.
		expect(screen.queryByRole('link', { name: 'A photo of the page' })).not.toBeInTheDocument();
		expect(went).not.toHaveBeenCalledWith('/imports/j_7');
	});

	it('offers a photo of the page where a Runner is', async () => {
		drawn({
			get_job: refusedWith({ reason: 'unrecognised', sentence: 'not known' }),
			runner_status: { has_runner: true, last_asked_at: '2026-10-23T09:00:00.000Z' },
		});
		await choose();
		expect(await screen.findByRole('link', { name: 'A photo of the page' })).toBeInTheDocument();
	});

	it('tells the cook to update an app whose shape Kamosu was not tested with', async () => {
		drawn({
			get_job: refusedWith({ reason: 'untested_shape', app: 'Mealie', sentence: 'update' }),
		});
		await choose();
		const card = await screen.findByRole('alert');
		expect(card).toHaveTextContent(
			'This is a Mealie export, in a shape Kamosu has not been tested with.',
		);
		expect(card).toHaveTextContent('Update Mealie, export again, and bring the new file.');
		expect(screen.queryByRole('link', { name: 'A PDF' })).not.toBeInTheDocument();
	});

	it('says a file nested too deep was not unpacked', async () => {
		drawn({ get_job: refusedWith({ reason: 'nested_too_deep', sentence: 'deep' }) });
		await choose();
		expect(await screen.findByRole('alert')).toHaveTextContent('It was not unpacked.');
	});

	it('moves nobody who left the page while it looked inside the file', async () => {
		const kamosu = standIn({
			list_jobs: { jobs: [] },
			import_from_another_app: { job_id: 'j_7' },
			get_job: job(),
		});
		const upload = vi.fn(async () => 'u_staged');
		const { unmount } = render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		await choose();
		await vi.waitFor(() =>
			expect(screen.getByText('Looking inside the file…')).toBeInTheDocument(),
		);
		unmount();
		kamosu.answer('get_job', job({ progress: { done: 0, total: 3, message: '0 of 3' } }));
		const asked = () => kamosu.calls.filter((call) => call.operation === 'get_job').length;
		const before = asked();
		await new Promise((wake) => setTimeout(wake, 50));
		expect(asked()).toBeLessThanOrEqual(before + 1);
		expect(went).not.toHaveBeenCalled();
	});

	it('opens the Report of an import that failed, which says why', async () => {
		drawn({ get_job: job({ status: 'failed', error: 'no such upload' }) });
		await choose();
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
	});

	it('says why a file could not be sent', async () => {
		const kamosu = standIn({ list_jobs: { jobs: [] } });
		const upload = vi.fn(async () => {
			throw new OperationError('upload', 'bad_request', 'the file is larger than 2 GB');
		});
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		await choose();
		expect(await screen.findByRole('alert')).toHaveTextContent('the file is larger than 2 GB');
	});

	it('links the last import, from this page or the Crouton button before it', async () => {
		drawn({
			list_jobs: {
				jobs: [
					{
						id: 'j_web',
						operation: 'import_web_link',
						status: 'completed',
						created_at: '2026-10-19T10:00:00Z',
					},
					{
						id: 'j_old',
						operation: 'import_crouton',
						status: 'completed',
						created_at: '2026-09-18T10:00:00Z',
					},
				],
			},
		});
		const link = await screen.findByRole('link', { name: "See the last import's report" });
		expect(link).toHaveAttribute('href', '/imports/j_old');
	});
});

describe('dropping an export on the place', () => {
	const recognised = () => job({ progress: { done: 0, total: 3, message: '0 of 3' } });

	it('starts the import its button starts, whatever the file is called', async () => {
		const { kamosu, upload } = drawn({ get_job: recognised() });
		const place = screen.getByText('Drop your export here');
		// The server says which app made it, by what is in it: no name is judged here.
		const dataTransfer = carrying({ files: [new File(['{}'], 'export', { type: '' })] });
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('Let go to import it')).toBeInTheDocument();
		expect(await fireEvent.dragOver(place, { dataTransfer })).toBe(false);
		await fireEvent.drop(place, { dataTransfer });

		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
		expect(upload).toHaveBeenCalledOnce();
		expect(kamosu.calls.some((call) => call.operation === 'import_from_another_app')).toBe(true);
	});

	it('takes one file at a time, and says so', async () => {
		const { upload } = drawn({ get_job: recognised() });
		const place = screen.getByText('Drop your export here');
		const dataTransfer = carrying({ files: [zip(), zip()] });
		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.getByText('One file at a time')).toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		expect(screen.getByRole('alert')).toHaveTextContent(
			"That didn't go anywhere. Drop the one export file your old app made.",
		);
		expect(upload).not.toHaveBeenCalled();
	});

	it('takes no second export while one is being sent', async () => {
		const kamosu = standIn({
			list_jobs: { jobs: [] },
			import_from_another_app: { job_id: 'j_7' },
			get_job: recognised(),
		});
		let sent: (id: string) => void = () => {};
		const upload = vi.fn(() => new Promise<string>((resolve) => (sent = resolve)));
		render(ImportTestHarness, { props: { client: kamosu.client, upload } });
		const place = screen.getByText('Drop your export here');
		const dataTransfer = carrying({ files: [zip()] });
		await fireEvent.dragEnter(place, { dataTransfer });
		await fireEvent.drop(place, { dataTransfer });
		await vi.waitFor(() => expect(upload).toHaveBeenCalledOnce());

		await fireEvent.dragEnter(place, { dataTransfer });
		expect(screen.queryByText('Let go to import it')).not.toBeInTheDocument();
		await fireEvent.drop(place, { dataTransfer });
		expect(upload).toHaveBeenCalledOnce();
		sent('u_staged');
		await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_7'));
	});
});
