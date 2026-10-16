/**
 * The screen-seam test: the Import Report for a Crouton library (#69),
 * option A as chosen — one page, three parts.
 *
 * `get_job`'s declared result is "anything", so the stand-in cannot hold the
 * Report to its shape here. The generated `ImportCroutonOutput` type does
 * instead: every report below is typed as one, so a field renamed in
 * `src/catalogue.rs` fails svelte-check in the same commit.
 *
 * What these tests are for is what no behaviour test can reach: that the
 * three parts are drawn as three different kinds of thing, that the page is
 * already there while the Job runs, and that ticking relates exactly the pairs
 * ticked and nothing else.
 */

import { describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, fireEvent, within } from '@testing-library/svelte';
import { m } from '$lib/paraglide/messages';
import ReportTestHarness from './ReportTestHarness.svelte';
import { standIn, type Answers } from '$lib/api/stand-in';
import type { GetJobOutput, ImportCroutonOutput } from '$lib/api/catalogue';
import { recipeAnswer } from '../../../testing/recipes';
import { went } from '../../../testing/navigation';

const side = (branch: string, title: string, over: Record<string, unknown> = {}) => ({
	lineage_id: `l_${branch}`,
	branch_id: `b_${branch}`,
	title,
	main_photo: null,
	ingredients: 11,
	...over,
});

const arrived = (branch: string, title: string, over: Record<string, unknown> = {}) => ({
	foreign_id: `crouton-${branch}`,
	status: 'created' as const,
	lineage_id: `l_${branch}`,
	branch_id: `b_${branch}`,
	title,
	main_photo: `photo-${branch}`,
	bare: false,
	...over,
});

const report = (over: Partial<ImportCroutonOutput> = {}): ImportCroutonOutput => ({
	import_id: 'i_1',
	cookbook_id: 'c_1',
	source_kind: 'crouton',
	arrived: [
		arrived('beef1', 'Beef Bourguignon'),
		arrived('beef2', 'Beef Bourguignon'),
		arrived('kfc1', 'Korean Fried Chicken'),
		arrived('kfc2', 'Korean Fried Chicken'),
		arrived('dan', 'Dan Dan Noodles', { main_photo: null, bare: true }),
		arrived('ribs', 'Beef Short Ribs'),
	],
	offered: [],
	unreadable: [],
	left_out: [
		{
			foreign_id: 'crouton-kfc1',
			branch_id: 'b_kfc1',
			title: 'Korean Fried Chicken',
			what: 'site_icon',
			count: 1,
			icon: 'data:image/png;base64,iVBORw0KGgo=',
		},
		{
			foreign_id: 'crouton-kfc2',
			branch_id: 'b_kfc2',
			title: 'Korean Fried Chicken',
			what: 'site_icon',
			count: 1,
			icon: 'data:image/png;base64,iVBORw0KGgo=',
		},
		{
			foreign_id: 'crouton-ribs',
			branch_id: 'b_ribs',
			title: 'Beef Short Ribs',
			what: 'extra_photos',
			count: 1,
		},
	],
	related_candidates: [
		{
			recipes: [
				side('beef1', 'Beef Bourguignon'),
				side('beef2', 'Beef Bourguignon', { ingredients: 16 }),
			],
			shared: ['name'],
		},
		{
			recipes: [side('kfc1', 'Korean Fried Chicken'), side('kfc2', 'Korean Fried Chicken')],
			shared: ['name', 'page'],
		},
	],
	...over,
});

const job = (over: Partial<GetJobOutput> = {}): GetJobOutput => ({
	id: 'j_1',
	operation: 'import_crouton',
	status: 'completed',
	progress: { done: 86, total: 86, message: 'finished' },
	result: report(),
	error: null,
	errorCode: null,
	created_at: '2026-09-19T10:00:00.000Z',
	updated_at: '2026-09-19T10:02:00.000Z',
	...over,
});

function open(answers: Answers) {
	localStorage.clear();
	const kamosu = standIn(answers);
	render(ReportTestHarness, { props: { client: kamosu.client, jobId: 'j_1' } });
	return kamosu;
}

describe('the Import Report', () => {
	it('names the source it is reporting, and draws no way back of its own (#219)', async () => {
		// A Crouton library. The way back is the shell's arrow since #219.
		open({ get_job: job() });
		expect(
			await screen.findByRole('heading', { name: 'Your Crouton library' }),
		).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: /‹/ })).toBeNull();
	});

	it('does not call one recipe file a Crouton library', async () => {
		// A recipe file reaches this page too, since #93 — it is where *How it
		// went* leads. Before that this heading was a fixed string, so a friend's
		// recipe was reported as *your Crouton library* under a back link to
		// Settings, which is not where it was brought in from.
		open({
			get_job: job({
				operation: 'import_bundle',
				result: report({
					source_kind: 'bundle',
					arrived: [arrived('soba', 'Soba with walnut miso', { subject: true })],
					left_out: [],
					related_candidates: [],
				}),
			}),
		});

		expect(
			await screen.findByRole('heading', { name: 'The recipe file you brought in' }),
		).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Your Crouton library' })).not.toBeInTheDocument();
	});

	it('does not call a web link no agent read a Crouton library (#308)', async () => {
		// #276 named the Reports an agent worked on, and left a link with no
		// agent in it under the Crouton title.
		open({
			get_job: job({
				operation: 'import_web_link',
				result: report({
					source_kind: 'web',
					arrived: [arrived('tart', 'Tarte au citron', { subject: true })],
					left_out: [],
					related_candidates: [],
				}),
			}),
		});

		expect(
			await screen.findByRole('heading', { name: 'The link you brought in' }),
		).toBeInTheDocument();
		expect(screen.queryByRole('heading', { name: 'Your Crouton library' })).not.toBeInTheDocument();
	});

	it('names a link while its import still runs, before any Report says so', async () => {
		open({
			get_job: job({
				operation: 'import_web_link',
				status: 'running',
				progress: { done: 0, total: 1, message: 'reading' },
				result: null,
			}),
		});

		expect(
			await screen.findByRole('heading', { name: 'The link you brought in' }),
		).toBeInTheDocument();
	});

	it('names a link the generic import brought in by its Report', async () => {
		// `import` takes whatever source its caller names, so only the Report
		// says this one was a web link.
		open({
			get_job: job({
				operation: 'import',
				result: report({
					source_kind: 'web',
					arrived: [arrived('tart', 'Tarte au citron', { subject: true })],
					left_out: [],
					related_candidates: [],
				}),
			}),
		});

		expect(
			await screen.findByRole('heading', { name: 'The link you brought in' }),
		).toBeInTheDocument();
	});

	it('is already a page while the import runs, and becomes the Report when it ends', async () => {
		const kamosu = open({
			get_job: job({
				status: 'running',
				progress: { done: 37, total: 86, message: '37 of 86' },
				result: null,
			}),
		});

		expect(await screen.findByText(/37 of 86 recipes in\./)).toBeInTheDocument();
		expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '37');
		// The pairs need both halves in, so the work is promised, not shown.
		expect(screen.getByText(/Once all are in/)).toBeInTheDocument();

		kamosu.answer('get_job', job());
		expect(await screen.findByText(/6 recipes are on your shelf/)).toBeInTheDocument();
		expect(screen.getByText(/2 pairs might be related/)).toBeInTheDocument();
	});

	it('relates exactly the pairs ticked, then says what was linked', async () => {
		const kamosu = open({
			get_job: job(),
			set_related_recipe: { related_recipes: [] },
		});

		const pairs = await screen.findByText(/Tick the pairs that are the same dish/);
		const card = pairs.closest('div') as HTMLElement;
		const relate = within(card).getByRole('button', { name: 'Relate pairs' });
		expect(relate).toBeDisabled();

		const boxes = within(card).getAllByRole('checkbox');
		expect(boxes).toHaveLength(2);
		expect(within(card).getByText(/same name · same page/)).toBeInTheDocument();
		expect(within(card).getByText(/11 and 16 ingredients/)).toBeInTheDocument();
		await fireEvent.click(boxes[1]);
		await fireEvent.click(within(card).getByRole('button', { name: 'Relate 1 pairs' }));

		expect(
			await screen.findByText(/Linked: Korean Fried Chicken. The other 1 kept apart./),
		).toBeInTheDocument();
		const related = kamosu.calls.filter((call) => call.operation === 'set_related_recipe');
		expect(related.map((call) => call.input)).toEqual([
			{ branch_id: 'b_kfc1', related_branch_id: 'b_kfc2', related: true },
		]);
	});

	it('unlinks a pair again when the answer is changed to "None of them"', async () => {
		const kamosu = open({
			get_job: job(),
			set_related_recipe: { related_recipes: [] },
		});
		const boxes = await screen.findAllByRole('checkbox');
		await fireEvent.click(boxes[0]);
		await fireEvent.click(boxes[1]);
		await fireEvent.click(screen.getByRole('button', { name: 'Relate 2 pairs' }));
		expect(
			await screen.findByText(/Linked: Beef Bourguignon, Korean Fried Chicken\./),
		).toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Change' }));
		await fireEvent.click(screen.getByRole('button', { name: 'None of them' }));

		expect(await screen.findByText(/None linked: all 2 kept apart./)).toBeInTheDocument();
		const answers = kamosu.calls
			.filter((call) => call.operation === 'set_related_recipe')
			.map((call) => (call.input as { related: boolean }).related);
		expect(answers).toEqual([true, true, false, false]);
	});

	it('keeps every pair apart on "None of them" without touching a recipe', async () => {
		const kamosu = open({ get_job: job() });
		await fireEvent.click(await screen.findByRole('button', { name: 'None of them' }));
		expect(await screen.findByText(/None linked: all 2 kept apart./)).toBeInTheDocument();
		expect(kamosu.calls.some((call) => call.operation === 'set_related_recipe')).toBe(false);
	});

	it('names a file it could not read, with what to do about it', async () => {
		open({
			get_job: job({
				result: report({
					unreadable: [
						{
							foreign_id: null,
							name: 'Îles Flottantes.crumb',
							reason: 'the file is damaged; export the library from Crouton again',
						},
					],
				}),
			}),
		});
		expect(await screen.findByText('Îles Flottantes.crumb')).toBeInTheDocument();
		expect(screen.getByText(/export the library from Crouton again/)).toBeInTheDocument();
		expect(screen.getByText(/1 couldn't be read/)).toBeInTheDocument();
	});

	it('says every file was read when none was lost', async () => {
		open({ get_job: job() });
		expect(await screen.findByText('Every file could be read.')).toBeInTheDocument();
	});

	it('says what arrived, what came bare, and what was left out on purpose', async () => {
		open({ get_job: job() });
		expect(await screen.findByText('6 recipes · 5 photos')).toBeInTheDocument();
		expect(
			screen.getByText(/1 came as just a name and a link: Dan Dan Noodles/),
		).toBeInTheDocument();
		expect(screen.getByText(/Left out: 2 website icons/)).toBeInTheDocument();
		// The two recipes share one icon, so it is drawn once.
		expect(document.querySelectorAll('img[src^="data:image/png"]')).toHaveLength(1);
		expect(screen.getByText(/one extra photo from Beef Short Ribs/)).toBeInTheDocument();
		expect(screen.getByText('Show all 6')).toBeInTheDocument();
	});

	it('says why an import stopped', async () => {
		open({
			get_job: job({
				status: 'failed',
				result: null,
				error: 'this zip holds no .crumb files, so it is not a Crouton export',
			}),
		});
		expect(await screen.findByRole('alert')).toHaveTextContent(/not a Crouton export/);
	});
});

/**
 * The Report beside the list of imports (#200, ADR 0044). On the wide layout
 * it is a card, as an open Food is, and the list stays beside it.
 */
describe('on the wide layout', () => {
	const arrival = (status: 'running' | 'completed') => ({
		job_id: 'j_1',
		created_at: '2026-09-19T10:00:00.000Z',
		status,
		arrived: status === 'completed' ? 6 : 0,
		created: status === 'completed' ? 6 : 0,
		offered: 0,
		unreadable: 0,
	});
	const listed = (status: 'running' | 'completed') => ({
		imports: [
			{
				import_id: 'imp_crouton',
				source_kind: 'crouton',
				created_at: '2026-09-19T10:00:00.000Z',
				remembered: 6,
				arrivals: [arrival(status)],
			},
		],
	});

	function beside(answers: Answers) {
		localStorage.clear();
		const kamosu = standIn(answers);
		render(ReportTestHarness, {
			props: { client: kamosu.client, jobId: 'j_1', room: 'wide', beside: true },
		});
		return kamosu;
	}

	it("is a card, and the phone's page is not", async () => {
		beside({ get_job: job(), list_imports: listed('completed') });
		const title = await screen.findByRole('heading', { level: 1 });
		expect(title.parentElement).toHaveClass('open-card');

		cleanup();
		open({ get_job: job() });
		expect((await screen.findByRole('heading', { level: 1 })).parentElement).not.toHaveClass(
			'open-card',
		);
	});

	it('tells the list when the import it was watching ends, so its row says what arrived', async () => {
		const kamosu = beside({
			get_job: job({
				status: 'running',
				progress: { done: 3, total: 6, message: '' },
				result: null,
			}),
			list_imports: listed('running'),
		});
		expect(await screen.findByText(m.imports_row_running())).toBeInTheDocument();

		kamosu.answer('list_imports', listed('completed'));
		kamosu.answer('get_job', job());

		expect(await screen.findByText(m.imports_row_new_many({ count: 6 }))).toBeInTheDocument();
	});

	it('leaves the list alone for an import that had ended before the Report opened', async () => {
		const kamosu = beside({ get_job: job(), list_imports: listed('completed') });
		await screen.findByText(/6 recipes are on your shelf/);

		expect(kamosu.calls.filter((call) => call.operation === 'list_imports')).toHaveLength(1);
	});
});

/**
 * An agent's work (#276), option B as chosen on 2026-10-15: what to look at
 * goes under Needs you, and Arrived keeps the bare landing and the agent's
 * work as one row.
 */
describe("the Import Report of an agent's work", () => {
	const TART = 'Tarte au citron meringuée';

	const agentReport = (
		over: Partial<ImportCroutonOutput> = {},
		agent: Record<string, unknown> = {},
	) =>
		report({
			source_kind: 'web',
			arrived: [
				arrived('tart', TART, {
					foreign_id: 'https://example.com/tart',
					bare: true,
					main_photo: null,
				}),
			],
			left_out: [],
			related_candidates: [],
			agent_jobs: [
				{
					job_id: 'j_agent',
					branch_id: 'b_tart',
					status: 'completed',
					made_by_agent: true,
					agent_report: {
						left_undone: ['The page gives no cooking time.'],
						unsure: ['Step 2 says 180 °C in one place and 160 °C in another.'],
						likely_duplicates: ['b_pie'],
						unused_foods: ['caster sugar'],
					},
					...agent,
				},
			],
			...over,
		});

	const pie = recipeAnswer({ branch_id: 'b_pie', title: 'Lemon meringue pie' });

	it('says an agent wrote the recipe, and puts what it noted under Needs you', async () => {
		open({
			get_job: job({ operation: 'import_web_link', result: agentReport() }),
			get_recipe: pie,
		});

		expect(
			await screen.findByRole('heading', { name: 'The link you brought in' }),
		).toBeInTheDocument();
		const duplicate = await screen.findByRole('link', { name: 'Lemon meringue pie' });
		expect(duplicate).toHaveAttribute('href', '/recipes/b_pie');
		expect(duplicate.closest('p')).toHaveTextContent(
			'The AI agent thinks this may be the same dish as Lemon meringue pie. Both are kept.',
		);

		const look = screen.getByText(/Have a look at/).closest('div') as HTMLElement;
		expect(within(look).getByRole('link', { name: TART })).toHaveAttribute(
			'href',
			'/recipes/b_tart',
		);
		expect(within(look).getByText('The AI agent left undone')).toBeInTheDocument();
		expect(within(look).getByText('The page gives no cooking time.')).toBeInTheDocument();
		expect(within(look).getByText('The AI agent was not sure of')).toBeInTheDocument();
		expect(within(look).getByText("Foods its corrections didn't use")).toBeInTheDocument();
		expect(within(look).getByText('caster sugar')).toBeInTheDocument();
		expect(screen.queryByText(m.report_needs_you_none())).not.toBeInTheDocument();

		// One arrival: the bare landing is not said apart from the agent's work.
		expect(screen.getByText(`Written by Kamosu's AI agent: ${TART}.`)).toBeInTheDocument();
		expect(screen.queryByText(/came as just a name and a link/)).not.toBeInTheDocument();
		expect(screen.getByText("written by Kamosu's AI agent")).toBeInTheDocument();
	});

	it('shows three entries of a long list and the rest behind one tap', async () => {
		const unsure = Array.from({ length: 8 }, (_, n) => `Doubt number ${n + 1}.`);
		open({
			get_job: job({
				operation: 'import_web_link',
				result: agentReport({}, { agent_report: { unsure } }),
			}),
		});

		expect(await screen.findByText('Doubt number 3.')).toBeVisible();
		const more = screen.getByText('Show 5 more');
		expect(more.closest('details')).not.toHaveAttribute('open');
		expect(
			within(more.closest('details') as HTMLElement).getByText('Doubt number 8.'),
		).toBeInTheDocument();
	});

	it('leaves out a likely duplicate that cannot be opened', async () => {
		const kamosu = open({
			get_job: job({
				operation: 'import_web_link',
				result: agentReport({}, { agent_report: { likely_duplicates: ['b_gone'] } }),
			}),
			get_recipe: { refuse: 'not_found' },
		});

		// "Nothing needs you" waits for the lookup, so this is after it answered.
		expect(await screen.findByText(m.report_needs_you_none())).toBeInTheDocument();
		expect(kamosu.calls.filter((call) => call.operation === 'get_recipe')).toEqual([
			expect.objectContaining({ input: { branch_id: 'b_gone' } }),
		]);
		expect(screen.queryByText(/may be the same dish/)).not.toBeInTheDocument();
	});

	it('names the recipe of a Job on a recipe the link was only offered to', async () => {
		// A link brought in again names the Job already standing on a recipe
		// the cook had changed since: that recipe is offered, not arrived.
		open({
			get_job: job({
				operation: 'import_web_link',
				result: agentReport(
					{
						arrived: [],
						offered: [
							{
								foreign_id: 'https://example.com/tart',
								branch_id: 'b_tart',
								lineage_id: 'l_tart',
								candidate_version_id: 'v_candidate',
								title: TART,
							},
						],
					},
					{ agent_report: { left_undone: ['The page gives no cooking time.'] } },
				),
			}),
		});

		const look = (await screen.findByText(/Have a look at/)).closest('p') as HTMLElement;
		expect(look).toHaveTextContent(`Have a look at ${TART}.`);
		expect(within(look).getByRole('link', { name: TART })).toHaveAttribute(
			'href',
			'/recipes/b_tart',
		);
	});

	it('words a failure Kamosu ended as the agent not finishing, and keeps the bare recipe arrived', async () => {
		const reason = 'the recipe this Job was for is gone';
		open({
			get_job: job({
				operation: 'import_web_link',
				result: agentReport(
					{
						unreadable: [
							{ foreign_id: 'https://example.com/tart', name: TART, branch_id: 'b_tart', reason },
						],
					},
					{ status: 'failed', made_by_agent: undefined, agent_report: undefined },
				),
			}),
		});

		const loss = (await screen.findByText(`The AI agent couldn't finish: ${reason}`)).closest('li');
		expect(loss).toHaveTextContent(TART);
		expect(screen.queryByText(/The agent said/)).not.toBeInTheDocument();
		expect(screen.getByText(`1 came as just a name and a link: ${TART}.`)).toBeInTheDocument();
		expect(screen.getByText(m.report_needs_you_none())).toBeInTheDocument();
	});

	it('quotes the agent where it said a file holds no recipe, and says nothing arrived', async () => {
		const reason = 'The picture shows a wine label, not a recipe.';
		open({
			get_job: job({
				operation: 'read_by_agent',
				status: 'failed',
				error: reason,
				errorCode: -32602,
				result: agentReport(
					{
						source_kind: 'picture',
						arrived: [],
						unreadable: [{ foreign_id: null, reason }],
					},
					{ branch_id: null, status: 'failed', could_not_be_read: true, made_by_agent: undefined },
				),
			}),
			runner_status: { has_runner: false, last_asked_at: null },
		});

		expect(
			await screen.findByRole('heading', { name: 'The picture you brought in' }),
		).toBeInTheDocument();
		const loss = screen.getByText(`The AI agent said: "${reason}"`).closest('li');
		expect(loss).toHaveTextContent('Your picture');
		expect(screen.getByText(/Nothing arrived\./)).toBeInTheDocument();
		expect(screen.queryByText(/The import stopped/)).not.toBeInTheDocument();
	});

	describe('a file the agent failed on, sent again (#279)', () => {
		const reason = 'The scan holds a wine label.';
		const failedPdf = (gone: boolean) =>
			job({
				operation: 'read_by_agent',
				status: 'failed',
				error: reason,
				errorCode: -32602,
				updated_at: '2026-10-14T10:00:00.000Z',
				result: agentReport(
					{ source_kind: 'pdf', arrived: [], unreadable: [{ foreign_id: null, reason }] },
					{
						branch_id: null,
						status: 'failed',
						could_not_be_read: true,
						made_by_agent: undefined,
						original_gone: gone,
					},
				),
			});
		const RUNNER = { has_runner: true, last_asked_at: '2026-10-15T10:00:00.000Z' };

		it('offers the same PDF again while Kamosu keeps it, and opens the new Job', async () => {
			const kamosu = open({
				get_job: failedPdf(false),
				runner_status: RUNNER,
				read_job_again: { job_id: 'j_2', source_kind: 'pdf' },
			});

			const again = await screen.findByRole('button', { name: m.report_again_pdf() });
			// Its 30 days run from when the Job ended.
			expect(screen.getByText(/until 13 November/)).toBeInTheDocument();
			// Under the file that could not be read.
			const loss = screen.getByText(/The scan holds a wine label/).closest('li')!;
			expect(loss.compareDocumentPosition(again) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

			await fireEvent.click(again);
			await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/imports/j_2'));
			expect(kamosu.calls.find((call) => call.operation === 'read_job_again')?.input).toEqual({
				job_id: 'j_1',
			});
		});

		it('opens the recipe an agent made from the same file since', async () => {
			open({
				get_job: failedPdf(false),
				runner_status: RUNNER,
				read_job_again: { branch_id: 'b_tart', title: 'Tart', source_kind: 'pdf' },
			});
			await fireEvent.click(await screen.findByRole('button', { name: m.report_again_pdf() }));
			await vi.waitFor(() => expect(went).toHaveBeenCalledWith('/recipes/b_tart'));
		});

		it('says the PDF is gone once Kamosu has deleted it, and offers nothing', async () => {
			open({ get_job: failedPdf(true), runner_status: RUNNER });
			expect(await screen.findByText(m.report_gone_pdf())).toBeInTheDocument();
			expect(screen.queryByRole('button', { name: m.report_again_pdf() })).toBeNull();
		});

		it('offers nothing on an instance with no Runner', async () => {
			const kamosu = open({
				get_job: failedPdf(false),
				runner_status: { has_runner: false, last_asked_at: null },
			});
			await screen.findByText(/The scan holds a wine label/);
			await vi.waitFor(() =>
				expect(kamosu.calls.some((call) => call.operation === 'runner_status')).toBe(true),
			);
			await new Promise((wake) => setTimeout(wake, 20));
			expect(screen.queryByRole('button', { name: m.report_again_pdf() })).toBeNull();
			expect(screen.queryByText(m.report_gone_pdf())).toBeNull();
		});
	});

	it('says a recipe read again by an agent that failed is as it was', async () => {
		open({
			get_job: job({
				operation: 'read_by_agent',
				status: 'failed',
				error: 'The page has gone.',
				result: agentReport(
					{
						source_kind: 'read_again',
						arrived: [arrived('tart', TART, { status: 'read_again', main_photo: null })],
						unreadable: [
							{
								foreign_id: 'crouton-tart',
								name: TART,
								branch_id: 'b_tart',
								reason: 'The page has gone.',
							},
						],
					},
					{
						status: 'failed',
						could_not_be_read: true,
						made_by_agent: undefined,
						agent_report: undefined,
					},
				),
			}),
		});

		expect(
			await screen.findByRole('heading', { name: `Reading ${TART} again` }),
		).toBeInTheDocument();
		expect(screen.getByText(new RegExp(`^${TART} is as it was\\.`))).toBeInTheDocument();
		expect(screen.getByText(`${TART} is as it was before.`)).toBeInTheDocument();
		expect(screen.getByText('as it was')).toBeInTheDocument();
	});

	it('says a link brought in again was written by an agent earlier', async () => {
		open({
			get_job: job({
				operation: 'import_web_link',
				result: agentReport(
					{
						arrived: [arrived('tart', TART, { status: 'unchanged', bare: true, main_photo: null })],
					},
					{ agent_report: undefined },
				),
			}),
		});

		expect(
			await screen.findByText(
				`Written by Kamosu's AI agent earlier, and already here, unchanged: ${TART}.`,
			),
		).toBeInTheDocument();
		expect(
			screen.getByText("written by Kamosu's AI agent · already here, unchanged"),
		).toBeInTheDocument();
		expect(screen.getByText(m.report_needs_you_none())).toBeInTheDocument();
	});
});

/**
 * While an agent waits or reads (#277), option B as chosen on 2026-10-15: the
 * tracking line at the top of the Report.
 */
describe('the Import Report while an agent waits or reads', () => {
	const TART = 'Tarte au citron meringuée';
	const HEARD = { has_runner: true, last_asked_at: new Date().toISOString() };

	const waitingReport = (status: 'queued' | 'running') =>
		report({
			source_kind: 'web',
			arrived: [
				arrived('tart', TART, {
					foreign_id: 'https://example.com/tart',
					bare: true,
					main_photo: null,
				}),
			],
			left_out: [],
			related_candidates: [],
			agent_jobs: [{ job_id: 'j_agent', branch_id: 'b_tart', status }],
		});

	/** The agent's Job, read by its own id beside the Report's. */
	const agentJob = (status: GetJobOutput['status']) =>
		job({
			id: 'j_agent',
			operation: 'read_by_agent',
			status,
			result: null,
			progress: { done: null, total: null, message: null },
		});

	it("draws a link's waiting agent on the tracking line, and the recipe as waited on", async () => {
		open({
			get_job: (input) =>
				input.job_id === 'j_agent'
					? agentJob('queued')
					: job({ operation: 'import_web_link', result: waitingReport('queued') }),
			get_recipe_job: { job: { id: 'j_agent', status: 'queued', yours: true } },
			runner_status: HEARD,
		});

		expect(await screen.findByText(m.agent_leave_report())).toBeInTheDocument();
		expect(screen.getByRole('button', { name: m.agent_stop() })).toBeInTheDocument();
		expect(screen.getByText(m.report_waiting_for_agent({ title: TART }))).toBeInTheDocument();
		expect(screen.queryByText(new RegExp('just a name and a link'))).toBeNull();
	});

	it('reads itself again once the agent has finished', async () => {
		const kamosu = open({
			get_job: (input) =>
				input.job_id === 'j_agent'
					? agentJob('running')
					: job({ operation: 'import_web_link', result: waitingReport('running') }),
			get_recipe_job: { job: { id: 'j_agent', status: 'running', yours: true } },
			runner_status: HEARD,
		});
		await screen.findByText(m.agent_leave_report());

		const done = report({
			...waitingReport('running'),
			agent_jobs: [
				{
					job_id: 'j_agent',
					branch_id: 'b_tart',
					status: 'completed',
					made_by_agent: true,
					agent_report: { left_undone: [], unsure: [], likely_duplicates: [], unused_foods: [] },
				},
			],
		});
		kamosu.answer('get_job', (input) =>
			input.job_id === 'j_agent'
				? agentJob('completed')
				: job({ operation: 'import_web_link', result: done }),
		);

		expect(
			await screen.findByText(m.report_written_by_agent({ titles: TART }), {}, { timeout: 5000 }),
		).toBeInTheDocument();
		expect(screen.queryByText(m.agent_leave_report())).toBeNull();
	});

	it("titles an agent's own Import for what was handed over while nothing has arrived", async () => {
		const own = report({
			source_kind: 'pasted_text',
			arrived: [],
			left_out: [],
			related_candidates: [],
			agent_jobs: [{ job_id: 'j_1', branch_id: null, status: 'queued' }],
		});
		open({
			get_job: job({
				operation: 'read_by_agent',
				status: 'queued',
				result: own,
				progress: { done: null, total: null, message: null },
			}),
			runner_status: HEARD,
		});

		expect(
			await screen.findByRole('heading', { name: m.report_title_pasted_text() }),
		).toBeInTheDocument();
		expect(await screen.findByText(m.report_agent_nothing_yet())).toBeInTheDocument();
		expect(screen.queryByText(m.report_waiting())).toBeNull();
	});

	it("says an agent's own Import was stopped, and nothing went wrong", async () => {
		const own = report({
			source_kind: 'pasted_text',
			arrived: [],
			left_out: [],
			related_candidates: [],
			agent_jobs: [{ job_id: 'j_1', branch_id: null, status: 'queued' }],
		});
		const asked = job({
			operation: 'read_by_agent',
			status: 'queued',
			result: own,
			progress: { done: null, total: null, message: null },
		});
		const kamosu = open({ get_job: asked, runner_status: HEARD, cancel_job: { cancelled: true } });
		await fireEvent.click(await screen.findByRole('button', { name: m.agent_stop() }));
		kamosu.answer('get_job', { ...asked, status: 'cancelled' });
		await fireEvent.click(screen.getByRole('button', { name: m.agent_stop_yes() }));

		expect(await screen.findByText(m.agent_stopped_nothing())).toBeInTheDocument();
		await new Promise((wake) => setTimeout(wake, 50));
		expect(screen.queryByRole('alert')).toBeNull();
		expect(screen.getByText(m.agent_stopped_nothing())).toBeInTheDocument();
	});

	it("offers no stopping on a Co-author's Report of a Job the other Co-author asked for", async () => {
		const kamosu = open({
			get_job: (input) =>
				input.job_id === 'j_agent'
					? agentJob('queued')
					: job({ operation: 'import_web_link', result: waitingReport('queued') }),
			get_recipe_job: { job: { id: 'j_agent', status: 'queued', yours: false } },
			runner_status: HEARD,
		});

		expect(await screen.findByText(m.agent_leave_report())).toBeInTheDocument();
		await vi.waitFor(() =>
			expect(kamosu.calls.some((call) => call.operation === 'get_recipe_job')).toBe(true),
		);
		await new Promise((wake) => setTimeout(wake, 20));
		expect(screen.queryByRole('button', { name: m.agent_stop() })).toBeNull();
	});

	it("says an agent's own Import was stopped when it is opened afterwards", async () => {
		open({
			get_job: job({
				operation: 'read_by_agent',
				status: 'cancelled',
				result: report({
					source_kind: 'picture',
					arrived: [],
					left_out: [],
					related_candidates: [],
					agent_jobs: [{ job_id: 'j_1', branch_id: null, status: 'cancelled' }],
				}),
				progress: { done: null, total: null, message: null },
			}),
		});

		expect(await screen.findByText(m.agent_stopped_nothing())).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: m.report_title_picture() })).toBeInTheDocument();
		expect(screen.queryByRole('alert')).toBeNull();
	});
});
