<!--
	A link for somebody else, drawn one way wherever Kamosu hands one over: a
	recipe's Share Link and every invite (#648, `docs/ux-rules.md` U9 and the
	audit's S8). Send opens the phone's own share menu where the browser has
	one, so the link goes to Messages or Mail in one tap rather than by copy,
	switch app and paste (S5). Copy is always there, and is the main button
	where the browser has no share menu (Firefox on a computer). The address
	sits under both, small, for whoever wants to see or select it.

	The share menu is offered only where `navigator.share` exists and says it
	takes a link: it is a promise the browser keeps, not one Kamosu makes.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { copyText } from '#lib/clipboard.js';

	interface Props {
		url: string;
	}

	let { url }: Props = $props();

	/** Whether this browser has a share menu that takes a link. */
	const sends = $derived(
		typeof navigator !== 'undefined' &&
			typeof navigator.share === 'function' &&
			(navigator.canShare?.({ url }) ?? true),
	);

	let copied = $state(false);

	async function copy() {
		// A browser that will not copy still shows the link to select by hand.
		copied = await copyText(url);
	}

	/**
	 * Straight from the tap with nothing awaited first, since a phone opens its
	 * share menu only for a tap it can still see. Closing the menu is no
	 * failure; any other refusal copies instead, so the tap still does
	 * something.
	 */
	function send() {
		void navigator.share({ url }).catch((error: unknown) => {
			if (error instanceof DOMException && error.name === 'AbortError') return;
			void copy();
		});
	}
</script>

{#if sends}
	<button type="button" onclick={send} class="main-button w-full">
		<svg
			viewBox="0 0 24 24"
			aria-hidden="true"
			class="mr-2 size-[20px] shrink-0"
			fill="none"
			stroke="currentColor"
			stroke-width="1.75"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="M12 15V3" />
			<path d="m7 8 5-5 5 5" />
			<path d="M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7" />
		</svg>
		{m.share_send()}
	</button>
	<button type="button" onclick={copy} class="mt-2 second-button w-full">
		{copied ? m.share_copied() : m.share_copy()}
	</button>
{:else}
	<button type="button" onclick={copy} class="main-button w-full">
		{copied ? m.share_copied() : m.share_copy()}
	</button>
{/if}
<p
	class="mt-3 border border-dashed border-rule bg-ground px-3 py-2 text-read break-all text-ink-2 select-all"
>
	{url}
</p>
