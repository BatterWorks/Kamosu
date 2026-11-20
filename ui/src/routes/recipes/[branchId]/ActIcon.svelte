<!-- An act's icon (#649), from `act-icons.ts`. The act's word is beside or under it. -->
<script lang="ts">
	import { ACT_ICONS, type ActIconName } from './act-icons';

	let { name, class: more = 'h-[22px] w-[22px] shrink-0' }: { name: ActIconName; class?: string } =
		$props();
</script>

<svg
	viewBox="0 0 24 24"
	aria-hidden="true"
	class={more}
	fill="none"
	stroke="currentColor"
	stroke-width="1.75"
	stroke-linecap="round"
	stroke-linejoin="round"
>
	<path d={ACT_ICONS[name]} />
</svg>
