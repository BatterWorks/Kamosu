<!--
	About Kamosu (#469): for everyone, at the foot of the Settings menu. The
	version, and the story (#158), the same page an Invite opens with.

	"What Kamosu does not protect" was here and on Account before, and left
	Settings with #469: no app shows its users such a list. It is in
	`SECURITY.md`, which the README links (ADR 0034).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Screen from '$lib/shell/Screen.svelte';
	import type { InstanceStatusOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	let status = $state<InstanceStatusOutput | undefined>(undefined);
	let failed = $state(false);

	$effect(() => {
		let current = true;
		kamosu
			.instanceStatus()
			.then((answer) => {
				if (current) status = answer;
			})
			.catch((error: unknown) => {
				if (!(error instanceof OperationError)) throw error;
				if (current) failed = true;
			});
		return () => {
			current = false;
		};
	});
</script>

<Screen title={m.settings_about()}>
	<p class="text-body text-ink">
		{#if failed}
			{m.instance_unreachable()}
		{:else if status}
			{m.settings_about_version({ version: status.version })}
		{:else}
			{m.loading()}
		{/if}
	</p>
	<a
		href="/about"
		class="mt-3 flex min-h-12 items-center justify-between gap-3 rounded-sm border border-rule
		bg-card px-3 text-body font-medium text-ink"
	>
		{m.story_what_is()}
		<span aria-hidden="true" class="text-ink-2">›</span>
	</a>
</Screen>
