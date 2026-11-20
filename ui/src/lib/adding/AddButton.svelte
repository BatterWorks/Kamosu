<!--
	*Add a recipe*, with its +: the one way into the add sheet (`AddRecipe`),
	beside the title of Recipes and Home, and on an empty Home as its main
	button (#650, option A). Drawn once here so the three stay one control.
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';

	interface Props {
		/** The screen's main act, filled: an empty Home's. Otherwise outlined beside a title. */
		main?: boolean;
		onclick: () => void;
	}

	let { main = false, onclick }: Props = $props();
</script>

<button
	type="button"
	aria-haspopup="dialog"
	class={main ? 'main-button w-full' : 'second-button px-3'}
	{onclick}
>
	<svg
		viewBox="0 0 24 24"
		aria-hidden="true"
		class="h-[20px] w-[20px] shrink-0"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg
	>
	{m.plus_label()}
</button>
