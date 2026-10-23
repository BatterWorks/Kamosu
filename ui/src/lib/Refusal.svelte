<!--
	The words of a refusal, said where the save was asked.

	Every screen keeps what a refusal said as text and draws it with this, so a
	Demo's refusal (#407) carries its way to installing as a link wherever it
	is shown, and every other refusal is drawn as the text it always was. The
	wording itself is `OperationError`'s, which says a Demo's refusal in the
	reader's own Language.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { INSTALL_KAMOSU } from '$lib/demo.svelte';

	interface Props {
		/** What the refusal said, if anything yet. */
		words: string | undefined;
	}

	let { words }: Props = $props();

	/** The Demo's sentence, cut round the words that lead to installing. */
	const around = $derived.by(() => {
		if (words === undefined || words !== m.demo_refused()) return undefined;
		const link = m.demo_refused_install();
		const at = words.indexOf(link);
		return at < 0
			? undefined
			: { before: words.slice(0, at), link, after: words.slice(at + link.length) };
	});
</script>

{#if around}{around.before}<a href={INSTALL_KAMOSU} class="underline" target="_blank" rel="noopener"
		>{around.link}</a
	>{around.after}{:else}{words}{/if}
