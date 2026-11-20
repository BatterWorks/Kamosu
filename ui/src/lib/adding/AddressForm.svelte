<!--
	Asking for a web page's address and importing it (#62, #93), inside the add
	sheet (#650, chosen 20 November 2026).

	The field is labelled with what goes in it, holds no `https://` of its own
	that a paste could double (L4), and wraps, so the whole address can be read
	back before it goes, where a one-line field showed only its end (L7). Its
	button is greyed until there is an address. Enter sends it, as it did when
	the field was one line.

	The address is held by the sheet (`link`), so going to check a page read
	from its own text and coming back finds it still typed.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import type { Adding } from './adding.svelte';

	interface Props {
		adding: Adding;
		link?: string;
	}

	let { adding, link = $bindable('') }: Props = $props();

	const field = $props.id();
	const empty = $derived(link.trim() === '');

	function send() {
		if (!empty && adding.working === null) adding.link(link);
	}
</script>

<form
	class="grid gap-3"
	onsubmit={(event) => {
		event.preventDefault();
		send();
	}}
>
	<label for={field} class="text-body text-ink">{m.recipes_import_link()}</label>
	<textarea
		id={field}
		bind:value={link}
		rows="3"
		inputmode="url"
		autocapitalize="off"
		autocomplete="off"
		spellcheck="false"
		data-sheet-focus
		placeholder={m.plus_link_paste_here()}
		onkeydown={(event) => {
			if (event.key === 'Enter' && !event.isComposing) {
				event.preventDefault();
				send();
			}
		}}
		class="block w-full resize-none rounded-sm border border-ink-2 bg-card px-3 py-2 text-body break-all text-ink placeholder:text-ink-2"
	></textarea>
	<button
		class="main-button w-full"
		disabled={empty || adding.working !== null}
		aria-busy={adding.working === 'link' ? 'true' : undefined}
	>
		{adding.working === 'link' ? m.recipes_import_working() : m.plus_link_add()}
	</button>
</form>
