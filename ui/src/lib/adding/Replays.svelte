<!--
	The examples a Demo replays the agent's work on (#406): option B, "one row,
	then a panel", as chosen on 22 October 2026 (the decision is a comment on
	#406). The + offers one row, "Watch it read an example", and it opens this
	panel of the four things a cook handed the agent, each drawn as what was
	handed over: the card, the magazine page, the top of the blog post, a frame
	of the video.

	Picking one opens its Import Report as a Replay (`$lib/agent/replay`):
	Sent, Waiting and Reading in a few seconds, then the Report as the agent
	left it. Nothing is written, so every visitor picks from the same four.

	It is read again when it opens, since a visitor's own file refused in a
	Demo points here from wherever it was sent (`WatchExamples`).
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages';
	import SheetFrame from '$lib/SheetFrame.svelte';
	import { replayAddress } from '$lib/agent/replay';
	import type { ListReplaysOutput } from '$lib/api/catalogue';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	const uid = $props.id();

	type Kind = ListReplaysOutput['replays'][number]['kind'];
	/** What each example is called, and what it holds. */
	const WORDS: Record<Kind, [() => string, () => string]> = {
		picture: [m.replay_picture, m.replay_picture_why],
		pdf: [m.replay_pdf, m.replay_pdf_why],
		pasted_text: [m.replay_pasted_text, m.replay_pasted_text_why],
		video: [m.replay_video, m.replay_video_why],
	};

	$effect(() => {
		void adding.readReplays();
	});

	const close = () => (adding.showingReplays = false);

	/** Played from the moment it is picked, not from when the panel opened. */
	function pick(jobId: string) {
		close();
		void goto(replayAddress(jobId));
	}
</script>

<SheetFrame
	labelledby="{uid}-heading"
	tall={88}
	class="overflow-y-auto bg-ground px-gutter pt-4 pb-6 text-ink"
	onclose={close}
>
	<p id="{uid}-heading" class="text-label text-ink-2 uppercase">{m.replays_heading()}</p>
	<!-- Two phrases, one paragraph: the sentence is longer than the copy guard
	     lets one phrase be (#141). -->
	<p class="mt-2 text-read">{m.replays_pick()} {m.replays_then()}</p>
	<ul class="mt-3 grid grid-cols-2 gap-2">
		{#each adding.replays as replay (replay.job_id)}
			{@const [label, why] = WORDS[replay.kind]}
			<li>
				<button
					type="button"
					class="block h-full w-full rounded-sm border border-rule bg-card p-2 text-left"
					onclick={() => pick(replay.job_id)}
				>
					{#if replay.photograph_id}
						<img
							src="/api/photographs/{replay.photograph_id}/card"
							alt=""
							class="mb-1 block aspect-[4/3] w-full rounded-sm bg-ground-2 object-cover"
						/>
					{:else}
						<span
							aria-hidden="true"
							class="mb-1 block aspect-[4/3] w-full overflow-hidden rounded-sm bg-ground-2 p-2 text-read break-words text-ink-2"
							>{replay.excerpt}</span
						>
					{/if}
					<span class="block font-display text-body text-accent">{label()}</span>
					<span class="block text-read text-ink-2">{why()}</span>
				</button>
			</li>
		{/each}
	</ul>
	<button
		type="button"
		class="mt-4 block w-full border border-rule p-4 text-center font-display text-body text-ink-2"
		onclick={close}
	>
		{m.send_back()}
	</button>
</SheetFrame>
