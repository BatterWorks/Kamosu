<!--
	A Demo's welcome card (#407, option B chosen on 23 October 2026), set into
	the top of Home above its heading the first time a browser arrives.

	It says who the visitor is (every visitor is the same Camille), what saves,
	when the library goes back, and how a visitor's own AI reads it: the MCP
	address and the read-only Access Key the Demo publishes. Got it puts it away
	on this device, and the strip's "A Kamosu demo" brings it back.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { getLocale } from '$lib/paraglide/runtime';
	import { copyText } from '$lib/clipboard';
	import { cleaningInLocalTime, closeWelcome, demo, INSTALL_KAMOSU } from '$lib/demo.svelte';

	const id = $props.id();

	/** Where an agent reaches this Demo's MCP door. */
	const address = `${location.origin}/mcp`;

	/** Which of the two went onto the clipboard last. */
	let copied = $state<'address' | 'key' | undefined>(undefined);

	async function copy(what: 'address' | 'key', text: string) {
		copied = (await copyText(text)) ? what : undefined;
	}
</script>

{#snippet copyable(what: 'address' | 'key', label: string, text: string)}
	<div class="mt-2 flex items-center gap-2">
		<span class="sr-only">{label}</span>
		<code
			class="min-w-0 flex-1 rounded-sm border border-rule bg-card px-3 py-2 text-read break-all text-ink select-all"
			>{text}</code
		>
		<button
			type="button"
			class="min-h-12 shrink-0 rounded-sm border border-rule bg-card px-3 text-read text-accent"
			onclick={() => copy(what, text)}
		>
			{copied === what ? m.demo_copied() : m.demo_copy()}
			<!-- Two Copy buttons sit one above the other; a screen reader hears which. -->
			<span class="sr-only">{label}</span>
		</button>
	</div>
{/snippet}

<section
	aria-labelledby="{id}-title"
	class="border border-t-4 border-rule border-t-accent bg-card px-4 pt-4"
>
	<p class="text-label text-ink-2 uppercase">{m.demo_name()}</p>
	<h2 id="{id}-title" class="mt-2 font-display text-title text-ink">{m.demo_welcome_title()}</h2>
	<p class="mt-3 text-body text-ink">{m.demo_welcome_who()}</p>

	<p class="mt-4 text-label text-ink-2 uppercase">{m.demo_welcome_saves_label()}</p>
	<p class="mt-1 text-body text-ink">{m.demo_welcome_saves()}</p>

	<p class="mt-4 text-label text-ink-2 uppercase">{m.demo_welcome_daily_label()}</p>
	<p class="mt-1 text-body text-ink">
		{m.demo_welcome_daily({ local: cleaningInLocalTime(getLocale()) })}
	</p>

	{#if demo.key}
		<p class="mt-4 text-label text-ink-2 uppercase">{m.demo_welcome_ai_label()}</p>
		<p class="mt-1 text-body text-ink">{m.demo_welcome_ai()}</p>
		{@render copyable('address', m.demo_welcome_address(), address)}
		{@render copyable('key', m.demo_welcome_key(), demo.key)}
	{/if}

	<div class="mt-6 grid gap-2 pb-6">
		<button
			type="button"
			class="block w-full bg-accent p-4 text-center font-display text-body text-on-accent"
			onclick={closeWelcome}
		>
			{m.demo_welcome_close()}
		</button>
		<a
			href={INSTALL_KAMOSU}
			target="_blank"
			rel="noopener"
			class="block w-full border border-rule bg-card p-4 text-center font-display text-body text-accent"
		>
			{m.demo_welcome_install()}
		</a>
	</div>
</section>
