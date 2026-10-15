<!--
	Handing a file the agent failed on to Kamosu's AI agent again, from what
	Kamosu kept of it (#273, #279): option 2 as chosen on 15 October 2026 (the
	decision is a comment on #279), a full-width button under the file that
	could not be read, so the cook need not find the file.

	**Drawn only for a photo, a PDF or pasted text whose Job failed before the
	agent made a recipe.** Nobody but the Person who handed it over reads such
	a Report (a Job with no recipe has no Co-authors), so whoever sees it may
	ask. While Kamosu keeps the file it is offered on an instance with a
	Runner; once the file is deleted, 30 days after the Job ended (#270), the
	Report says so instead and points at the +.

	Sending lands on the new Job's own Report, which follows it (#277), or on
	the recipe, where an agent has made one from the same file since.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { goto } from '$app/navigation';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { aDay } from '$lib/dates';
	import NeedsServer from '$lib/offline/NeedsServer.svelte';

	interface Props {
		jobId: string;
		/** What was handed over. */
		kind: 'picture' | 'pdf' | 'pasted_text';
		/** When the Job ended, which is when its 30 days began. */
		endedAt: string;
		/** Kamosu has deleted what was handed over. */
		gone: boolean;
	}

	let { jobId, kind, endedAt, gone }: Props = $props();

	const kamosu = useKamosu();

	/** Whether there is a Runner to send it to, read from the server each time. */
	let runner = $state(false);
	$effect(() => {
		if (gone) return;
		let current = true;
		kamosu
			.runnerStatus({})
			.then((status) => {
				if (current) runner = status.has_runner;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
			});
		return () => {
			current = false;
		};
	});

	/** The day Kamosu deletes it: 30 days after the Job ended. */
	const until = $derived(aDay(new Date(new Date(endedAt).getTime() + 30 * 86_400_000)));

	let sending = $state(false);
	let said = $state<string | undefined>(undefined);

	async function send() {
		sending = true;
		said = undefined;
		try {
			const answer = await kamosu.readJobAgain({ job_id: jobId });
			if (answer.job_id) await goto(`/imports/${answer.job_id}`);
			else if (answer.branch_id) await goto(`/recipes/${answer.branch_id}`);
		} catch (error) {
			if (!(error instanceof OperationError)) throw error;
			said = error.message;
		} finally {
			sending = false;
		}
	}

	const LABEL = {
		picture: () => m.report_again_picture(),
		pdf: () => m.report_again_pdf(),
		pasted_text: () => m.report_again_pasted_text(),
	};
	const GONE = {
		picture: () => m.report_gone_picture(),
		pdf: () => m.report_gone_pdf(),
		pasted_text: () => m.report_gone_pasted_text(),
	};
</script>

{#if gone}
	<p class="mt-2 text-read text-ink-2">{GONE[kind]()}</p>
{:else if runner}
	<div class="mt-3">
		<NeedsServer
			label={sending ? m.report_again_sending() : LABEL[kind]()}
			waiting={m.report_again_waits()}
			disabled={sending}
			onclick={send}
			shapeClass="min-h-12 w-full rounded-sm px-4 font-display text-body font-semibold disabled:opacity-60"
			lookClass="bg-accent text-on-accent"
		/>
		<p class="mt-2 text-read text-ink-2">
			{kind === 'pasted_text'
				? m.report_again_said_text({ date: until })
				: m.report_again_said_file({ date: until })}
		</p>
		{#if said}
			<p class="mt-2 text-read text-support" role="alert">{said}</p>
		{/if}
	</div>
{/if}
