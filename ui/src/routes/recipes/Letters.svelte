<!--
	The letters A to Z down the right edge of Recipes (#650, option A, chosen
	20 November 2026): a tap on one, or a finger sliding along them, takes the
	list to the first recipe filed under it, as a phone's contacts list does.
	The UX audit found reaching S meant passing about thirty cards (R7).

	**Each letter is drawn about 18px tall and 22 wide, and touches the next**,
	under the 48px rule of `docs/ux-rules.md` (U10). Raised with the person who chose and
	accepted with the choice: the strip is slid along rather than aimed at, and
	a letter missed lands one away, on the recipes beside the ones wanted.

	A letter no recipe on the shelf starts with is greyed, and still takes the
	list to the next one held, so a finger sliding over a gap keeps moving it.
	Only the shelf in its A to Z order has letters: a search's results are in
	the order they matched, which no letter describes.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import { LETTERS, nearest, type Letter } from './letters';

	interface Props {
		/** The letters some recipe on the shelf starts with. */
		held: ReadonlySet<Letter>;
		/** Take the list to this letter's first recipe. */
		onjump: (letter: Letter) => void;
	}

	let { held, onjump }: Props = $props();

	let strip = $state<HTMLElement | undefined>(undefined);
	/** The letter the finger is on, lit while it is down. */
	let touching = $state<Letter | null>(null);

	function go(letter: Letter) {
		const to = nearest(letter, held);
		touching = letter;
		if (to) onjump(to);
	}

	/** The letter under a point on the strip, by its height alone, so a finger drifting sideways still counts. */
	function under(y: number): Letter | null {
		if (!strip) return null;
		const box = strip.getBoundingClientRect();
		const at = Math.floor(((y - box.top) / box.height) * LETTERS.length);
		return LETTERS[Math.min(LETTERS.length - 1, Math.max(0, at))] ?? null;
	}

	function down(event: PointerEvent) {
		if (event.pointerType === 'mouse') return;
		strip?.setPointerCapture(event.pointerId);
		const letter = under(event.clientY);
		if (letter) go(letter);
	}

	function move(event: PointerEvent) {
		if (!strip?.hasPointerCapture(event.pointerId)) return;
		const letter = under(event.clientY);
		if (letter && letter !== touching) go(letter);
	}

	function up(event: PointerEvent) {
		if (strip?.hasPointerCapture(event.pointerId)) strip.releasePointerCapture(event.pointerId);
		touching = null;
	}
</script>

<nav
	bind:this={strip}
	aria-label={m.recipes_letters()}
	class="fixed top-1/2 right-0 z-20 flex -translate-y-1/2 touch-none flex-col items-center px-[2px] py-1 select-none wide:right-2"
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
>
	{#each LETTERS as letter (letter)}
		<!-- The letter under the finger is lit on a disc of its own, as the
		     chosen page drew it, inside the button, which is otherwise the bare
		     letter. Round is a clash with the near-square house rule, raised with
		     the person who chose, as the shelf's arrows were (#223). -->
		<button
			type="button"
			class="flex w-[22px] justify-center font-sans text-letter leading-[1.6] font-semibold wide:w-[28px] wide:text-letter-wide"
			onclick={() => {
				go(letter);
				touching = null;
			}}
		>
			<span
				class={[
					'px-[3px]',
					touching === letter
						? 'rounded-[50%] bg-accent text-on-accent'
						: held.has(letter)
							? 'text-accent'
							: 'text-rule',
				]}>{letter}</span
			>
		</button>
	{/each}
</nav>
