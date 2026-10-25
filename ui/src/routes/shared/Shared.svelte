<!--
	What Android's Share menu answers (#452, option A of 24 October 2026,
	`.dev/prototypes/452-share/android-a-agent-phone.html`): a quiet page of
	its own, drawn bare, with the mark, what Kamosu said, and two ways on.
	Most people press Back to where they were; the button is for the rest.

	The service worker caught the share and handed it to `add_this`
	(`$lib/offline/shared.ts`), keeps the answer, and brings the page here
	with its address naming it. Nothing is asked of the server from here, and
	nothing the address says is shown.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { SharedAnswer } from '$lib/offline/shared';

	interface Props {
		/** What the worker kept for this page, or none where nothing is kept. */
		answer: SharedAnswer | undefined;
	}

	let { answer }: Props = $props();
	const outcome = $derived(answer?.outcome ?? 'nothing');
	const missed = $derived(
		outcome === 'not_added' || outcome === 'offline' || outcome === 'nothing',
	);

	const heading = $derived(
		{
			added: m.shared_added(),
			sent: m.shared_sent(),
			already: m.shared_already(),
			not_added: m.shared_not_added(),
			offline: m.shared_not_sent(),
			nothing: m.shared_not_sent(),
		}[outcome],
	);
	const said = $derived(
		outcome === 'offline'
			? m.shared_offline()
			: outcome === 'nothing'
				? m.shared_nothing()
				: (answer?.message ?? ''),
	);
	const recipe = $derived(answer?.branchId);
</script>

<svelte:head>
	<title>{heading} · Kamosu</title>
</svelte:head>

<div class="mx-auto flex min-h-dvh max-w-[520px] flex-col px-gutter pt-8 pb-6">
	<p class="font-display text-label text-ink-2 uppercase">Kamosu</p>
	<div
		class={[
			'mt-12 grid h-[64px] w-[64px] place-items-center round-mark text-on-accent',
			missed ? 'bg-support' : 'bg-accent',
		]}
		aria-hidden="true"
	>
		<svg
			viewBox="0 0 24 24"
			class="h-[36px] w-[36px]"
			fill="none"
			stroke="currentColor"
			stroke-width="1.75"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			{#if missed}
				<path d="M12 7v6M12 17h.01" /><circle cx="12" cy="12" r="9" />
			{:else}
				<path d="M5 12.5l4.5 4.5L19 7.5" />
			{/if}
		</svg>
	</div>
	<h1 class="mt-4 font-display text-title font-semibold">{heading}</h1>
	<p class="mt-2 text-body" role="status">{said}</p>
	<div class="mt-auto pt-8">
		<a
			href={recipe ? `/recipes/${recipe}` : '/'}
			class="flex min-h-12 items-center justify-center rounded-sm bg-accent px-4 font-semibold text-on-accent"
		>
			{recipe
				? answer?.title
					? m.shared_open_titled({ title: answer.title })
					: m.shared_open_recipe()
				: m.shared_open_kamosu()}
		</a>
		<p class="mt-3 text-center text-read text-ink-2">{m.shared_back()}</p>
	</div>
</div>
