<!--
	Asking for a video's address and handing it to Kamosu's AI agent (#349).

	It opens inside the add sheet, as From a web page does (#650), and only
	on an instance with a Runner, whose sheet offers the row. The link goes to the
	agent as it is: Kamosu does not read the page first, so no recipe made
	from the video page's title lands before the agent's. No sheet asks
	first, since the link is already in view, as pasted text is (#278).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Refusal from '#lib/Refusal.svelte';
	import WatchExamples from './WatchExamples.svelte';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
	}

	let { adding }: Props = $props();

	let link = $state('');
</script>

<form
	class="grid gap-2"
	onsubmit={(event) => {
		event.preventDefault();
		adding.send({ link });
	}}
>
	<p class="text-read text-ink">{m.plus_video_hint()}</p>
	<label class="grid gap-1 text-body text-ink">
		{m.plus_video_link()}
		<input
			type="url"
			bind:value={link}
			required
			data-sheet-focus
			placeholder={m.plus_link_paste_here()}
			class="min-h-12 w-full rounded-sm border border-ink-2 bg-card px-3 text-body text-ink placeholder:text-ink-2"
		/>
	</label>
	{#if adding.failed}
		<p class="text-read text-support" role="alert"><Refusal words={adding.failed} /></p>
	{/if}
	<WatchExamples {adding} />
	<button
		class="main-button w-full"
		disabled={adding.working !== null}
		aria-busy={adding.working === 'send' ? 'true' : undefined}
	>
		{adding.working === 'send' ? m.send_doing() : m.send_do()}
	</button>
</form>
