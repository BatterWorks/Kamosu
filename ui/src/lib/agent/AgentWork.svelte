<!--
	An agent's Job, while it waits or is read (#277): option B, "the tracking
	line", as chosen on 15 October 2026 (the decision is a comment on #277).

	Four steps, Sent, Waiting, Reading and Ready, with the one the Job is at in
	bold, and one sentence under them. It sits above "Cook this" on a recipe's
	page and at the top of an Import Report. Nothing about the recipe is greyed
	out or taken away while it shows: the recipe under it is cookable.

	**It reads the Job again every few seconds**, through `get_job`, so it
	follows the change without the page being reloaded. These reads are never
	answered from what the phone kept (`reads.ts`). When one fails, offline or
	otherwise, the card goes rather than show a state it can no longer vouch
	for, and comes back with the next read that succeeds.

	**When no agent is answering**, the Waiting step turns red and says so. It
	is said only beside a Job that waits: one being read has an agent. The
	Operator is pointed at the Runner Keys, everyone else at the Operator.

	**Only the Person who asked stops it** (#267), and they are asked first, in
	place. Whatever the agent wrote stays. Once the Job has ended any other way,
	the card goes and `ended` is said, for the screen to read itself again.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { aTime } from '$lib/dates';
	import { asksWhetherAdministering } from '$lib/operator/administering';
	import type { GetJobOutput } from '$lib/api/catalogue';
	import { EVERY_MS, RUNNER_KEYS, noAgentAnswering, working, type Saying } from './agent';

	interface Props {
		jobId: string;
		/** The Person reading asked for the Job, and so may stop it. */
		mine: boolean;
		/**
		 * Which sentence goes under the steps. On a recipe's page a Job reading
		 * the recipe again says it is updating it, whoever reads the page.
		 */
		saying?: Saying;
		/** Whether a recipe is there to stay as the agent left it, once stopped. */
		hasRecipe?: boolean;
		/** The card's ground: a card on the page, or the page's ground inside a card. */
		panel?: string;
		/** Set in from the page's edges, as a recipe's page sets what it draws. */
		gutter?: boolean;
		/** How long between reads. Tests shorten it. */
		every?: number;
		/** Said once the Job has ended, however it ended. */
		ended?: (status: GetJobOutput['status']) => void;
	}

	let {
		jobId,
		mine,
		saying = 'leave',
		hasRecipe = true,
		panel = 'bg-card',
		gutter = true,
		every = EVERY_MS,
		ended,
	}: Props = $props();

	const kamosu = useKamosu();

	let job = $state<GetJobOutput | undefined>(undefined);
	let unanswered = $state(false);
	let operator = $state<boolean | undefined>(undefined);
	let asking = $state(false);
	let stopping = $state(false);
	let stopped = $state(false);
	let said = $state<string | undefined>(undefined);

	const at = $derived(job ? working(job.status) : undefined);
	/** A Job reading a recipe again is updating it, which the page says (#272). */
	const readingAgain = $derived(
		(job?.result as { source_kind?: string } | null)?.source_kind === 'read_again',
	);
	const sentence = $derived(saying === 'leave' && readingAgain ? 'updating' : saying);

	$effect(() => {
		const id = jobId;
		let current = true;
		let timer: ReturnType<typeof setTimeout> | undefined;
		const read = async () => {
			try {
				const read = await kamosu.getJob({ job_id: id });
				if (!current) return;
				// Stopped here: `ended` was said when it was stopped, and is not said twice.
				if (stopped) return;
				job = read;
				if (!working(read.status)) {
					ended?.(read.status);
					return;
				}
				if (read.status === 'queued') {
					const status = await kamosu.runnerStatus({});
					if (!current) return;
					unanswered = noAgentAnswering(status);
				} else {
					unanswered = false;
				}
			} catch (error) {
				if (!(error instanceof OperationError)) throw error;
				// Out of reach, or refused: say nothing rather than something old.
				if (current) job = undefined;
			}
			if (current) timer = setTimeout(read, every);
		};
		void read();
		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	// Who to point at, asked once and only when there is something to say.
	$effect(() => {
		if (!unanswered || operator !== undefined) return;
		asksWhetherAdministering(kamosu).then((answer) => {
			// A Kamosu that could not say is not a no: the cook's words stand meanwhile.
			if (answer.may !== 'unknown') operator = answer.may;
		});
	});

	async function stop() {
		stopping = true;
		said = undefined;
		try {
			await kamosu.cancelJob({ job_id: jobId });
			stopped = true;
			asking = false;
			ended?.('cancelled');
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			stopping = false;
		}
	}

	const STEPS = [
		() => m.agent_step_sent(),
		() => m.agent_step_waiting(),
		() => m.agent_step_reading(),
		() => m.agent_step_ready(),
	];
	/** The step the Job is at: Waiting is the second, Reading the third. */
	const current = $derived(at === 'waiting' ? 1 : 2);
	const under = (index: number) =>
		index === 0 && job
			? aTime(job.created_at)
			: index === 2 && at === 'reading' && job
				? // A Job is held from the moment its Runner took it, and nothing
					// moves an agent's Job's `updated_at` until it ends.
					m.agent_since({ time: aTime(job.updated_at) })
				: '';
</script>

{#if stopped}
	<p
		class={[
			'mt-4 border-l-3 border-rule px-3 py-2 text-read text-ink-2',
			gutter && 'mx-gutter',
			panel,
		]}
		role="status"
	>
		{hasRecipe ? m.agent_stopped() : m.agent_stopped_nothing()}
	</p>
{:else if at}
	<section
		class={['mt-4 rounded-sm border border-rule px-3 py-3', gutter && 'mx-gutter', panel]}
		aria-label={m.agent_steps()}
	>
		<ol class="flex">
			{#each STEPS as step, index (index)}
				{@const done = index < current}
				{@const here = index === current}
				<li class="relative flex-1 text-center" aria-current={here ? 'step' : undefined}>
					{#if index < STEPS.length - 1}
						<span
							aria-hidden="true"
							class={[
								'absolute top-[6px] right-[calc(-50%+8px)] left-[calc(50%+8px)] h-[2px]',
								done ? 'bg-accent' : 'bg-rule',
							]}
						></span>
					{/if}
					<span
						aria-hidden="true"
						class={[
							'mx-auto block h-[14px] w-[14px] rounded-[50%] border-2',
							here && unanswered ? 'border-support bg-support' : 'border-accent',
							(done || here) && !unanswered ? 'bg-accent' : '',
							done && unanswered ? 'bg-accent' : '',
						]}
					></span>
					<span
						class={[
							'mt-1 block text-label',
							here ? 'font-semibold' : 'text-ink-2',
							here && unanswered ? 'text-support' : here ? 'text-ink' : '',
						]}>{step()}</span
					>
					<span class="block text-label text-ink-2">{under(index)}</span>
				</li>
			{/each}
		</ol>

		<p class={['mt-3 text-read', unanswered ? 'text-support' : 'text-ink-2']} aria-live="polite">
			{#if unanswered}
				{m.agent_no_answer()}
				{#if operator}
					{m.agent_no_answer_operator()}
					<a href="/operator#{RUNNER_KEYS}" class="text-accent underline">{m.agent_runner_keys()}</a
					>.
				{:else}
					{m.agent_no_answer_cook()}
				{/if}
			{:else if sentence === 'updating'}
				{m.agent_updating()}
			{:else if sentence === 'report'}
				{m.agent_leave_report()}
			{:else}
				{m.agent_leave()}
			{/if}
		</p>

		{#if mine}
			{#if asking}
				<div class="mt-3 border-t border-rule pt-3">
					<p class="text-read">{m.agent_stop_ask()}</p>
					<div class="mt-2 grid grid-cols-2 gap-2">
						<button
							type="button"
							class="min-h-12 rounded-sm bg-support text-body font-semibold text-on-accent"
							disabled={stopping}
							onclick={stop}>{m.agent_stop_yes()}</button
						>
						<button
							type="button"
							class="min-h-12 rounded-sm border border-rule text-body text-accent"
							disabled={stopping}
							onclick={() => (asking = false)}>{m.agent_stop_no()}</button
						>
					</div>
				</div>
			{:else}
				<button
					type="button"
					class="mt-3 min-h-12 w-full rounded-sm border border-rule text-body text-accent"
					onclick={() => (asking = true)}>{m.agent_stop()}</button
				>
			{/if}
			{#if said}
				<p class="mt-2 text-read text-support" role="alert">{said}</p>
			{/if}
		{/if}
	</section>
{/if}
