<!--
	A small tag on a row in Brought in, while an agent's Job on that arrival
	waits or is read (#277): outlined while it waits, filled while it is read,
	in the words the tracker's own steps use.
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Working } from './agent';

	interface Props {
		at: Working;
	}

	let { at }: Props = $props();
</script>

<span
	class={[
		// The step's own words, without the letter-spacing a label carries.
		'ml-1 inline-block rounded-sm border border-accent px-[6px] align-middle text-label tracking-[0]',
		at === 'reading' ? 'bg-accent text-on-accent' : 'text-accent',
	]}>{at === 'reading' ? m.agent_step_reading() : m.agent_step_waiting()}</span
>
