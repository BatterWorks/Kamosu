<!--
	The Security Record (#371, choice A of 20 October 2026). Since #469 each
	half is a page of its own: yours under You, for everybody, and the
	instance's under This instance, for an Operator. Rows are grouped by day,
	a line saying what happened and one under it saying how, by whom and
	when. A run of wrong passwords and a crowd at the sign-in carry a
	support-coloured edge. The prototype is
	`.dev/prototypes/371-security-record/pages/a-record-operator.phone.html`
	and its states.

	The instance's record refused is a record that is not here, and the page
	says what a room that is not here says (ADR 0040).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import { byDay, say, type SecurityEvent } from '$lib/security-record';
	import type { ListSecurityRecordOutput } from '$lib/api/catalogue';

	interface Props {
		whose: 'yours' | 'instance';
	}

	let { whose }: Props = $props();

	const kamosu = useKamosu();

	let record = $state<ListSecurityRecordOutput | undefined>(undefined);
	let refused = $state(false);

	$effect(() => {
		let current = true;
		(whose === 'yours' ? kamosu.listSecurityRecord() : kamosu.listInstanceSecurityRecord()).then(
			(answer) => {
				if (current) record = answer;
			},
			(error) => {
				// Refused: this Person is no Operator. Anything else is a
				// Mistake the page reports.
				if (!(error instanceof OperationError && error.kind === 'unauthorized')) throw error;
				if (current) refused = true;
			},
		);
		return () => {
			current = false;
		};
	});
</script>

{#snippet row(event: SecurityEvent)}
	{@const said = say(event)}
	<li
		class={[
			'border-rule px-3 py-[10px] [&+&]:border-t',
			said.warn && 'border-l-3 border-l-support',
		]}
	>
		<p class="text-body break-words text-ink">{said.line}</p>
		<p class="text-read break-words text-ink-2">{said.under}</p>
	</li>
{/snippet}

{#if refused}
	<p class="text-body text-ink-2">{m.operator_not_here()}</p>
{:else if record}
	<p class="mb-2 text-read text-ink-2">
		{whose === 'yours'
			? m.security_yours_says({ days: record.kept_for_days })
			: m.security_instance_says({ days: record.kept_for_days })}
	</p>
	{#if record.events.length === 0}
		<p class="text-body text-ink-2">{m.security_empty({ days: record.kept_for_days })}</p>
	{:else}
		{#each byDay(record.events) as day (day.heading)}
			<h3 class="mt-[18px] mb-[6px] text-label text-ink-2 uppercase">{day.heading}</h3>
			<ul class="rounded-sm border border-rule bg-card">
				{#each day.events as event (event.id)}
					{@render row(event)}
				{/each}
			</ul>
		{/each}
	{/if}
{/if}
