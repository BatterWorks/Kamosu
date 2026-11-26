<!--
	A Step's own words, with each conversion drawn straight after what it
	converts (#150): "preheat to 425° (about 220 °C)", "1 lb. (about 455 g)
	ground chicken". Option A, the choice on 25 September 2026, over all
	of a Step's figures on one line beneath it, which left a cook counting along
	a row of numbers to match them back.

	The words are the Step's truth and are drawn exactly as written; what sits
	between them is the screen's addition and was never written into the Step
	(ADR 0016). The two screens that read a Step set its size and colour. The
	written text and its figure never break across a line: found live, "3" ended one
	line and "oz. (about 85 g)" began the next.

	A COOKING AT ANOTHER AMOUNT puts tonight's figure in the words, in their
	place and at their size, and the recipe's figure is not drawn (round 1 of
	#681's climb). Drawn as an addition, the recipe's figures stood at the
	Step's size and tonight's small and grey beside them: "Melt 30 g butter
	(about 2 oz)" at twice the recipe, where the 30 g was for half the pot. The
	scaled Ingredient Lines say only tonight's amount too, since #681; the
	recipe's own stays on the recipe page.
-->
<script lang="ts">
	import { stepPieces, type StepConversions } from '#lib/step-conversions.js';

	interface Props {
		text: string;
		conversions: StepConversions;
		/** The addition's type and colour, which the two screens set differently. */
		readingClass: string;
		/** The figures are tonight's, scaled: they take the written ones' place. */
		lead?: boolean;
	}

	let { text, conversions, readingClass, lead = false }: Props = $props();

	const pieces = $derived(stepPieces(text, conversions));
</script>

{#each pieces as piece, index (index)}{piece.text}{#if piece.written && lead}<span
			class="whitespace-nowrap">{piece.measured}</span
		>{:else if piece.written}<span class="whitespace-nowrap"
			>{piece.written}&nbsp;<span class={readingClass}>({piece.measured})</span></span
		>{/if}{/each}
