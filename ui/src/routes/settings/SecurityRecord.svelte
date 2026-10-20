<!--
	The Security Record (#371, choice A of 20 October 2026): a page of its own
	on the Settings menu. "Yours" for everybody, and "This instance" below it
	for an Operator. Rows are grouped by day, a line saying what happened and
	one under it saying how, by whom and when. A run of wrong passwords and a
	crowd at the sign-in carry a support-coloured edge. The prototype is
	`.dev/prototypes/371-security-record/pages/a-record-operator.phone.html`
	and its states.

	Whether the instance's record is shown is learnt the way the Operator
	screen learns it: by asking, and showing nothing where Kamosu refuses
	(ADR 0040).
-->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import { useKamosu } from '$lib/kamosu';
	import { OperationError } from '$lib/api/client';
	import Section from '$lib/shell/Section.svelte';
	import { byDay, say, type SecurityEvent } from '$lib/security-record';
	import type { ListSecurityRecordOutput } from '$lib/api/catalogue';

	const kamosu = useKamosu();

	let mine = $state<ListSecurityRecordOutput | undefined>(undefined);
	let instance = $state<ListSecurityRecordOutput | undefined>(undefined);

	$effect(() => {
		let current = true;
		kamosu.listSecurityRecord().then((answer) => {
			if (current) mine = answer;
		});
		kamosu.listInstanceSecurityRecord().then(
			(answer) => {
				if (current) instance = answer;
			},
			(error) => {
				// Refused: this Person is no Operator, and the section is simply
				// not here. Anything else is a Mistake the page reports.
				if (!(error instanceof OperationError && error.kind === 'unauthorized')) throw error;
			},
		);
		return () => {
			current = false;
		};
	});
</script>

{#snippet record(heading: string, says: string, answer: ListSecurityRecordOutput)}
	<Section {heading}>
		<p class="mb-2 text-read text-ink-2">{says}</p>
		{#if answer.events.length === 0}
			<p class="text-body text-ink-2">{m.security_empty({ days: answer.kept_for_days })}</p>
		{:else}
			{#each byDay(answer.events) as day (day.heading)}
				<h3 class="mt-[18px] mb-[6px] text-label text-ink-2 uppercase">{day.heading}</h3>
				<ul class="rounded-sm border border-rule bg-card">
					{#each day.events as event (event.id)}
						{@render row(event)}
					{/each}
				</ul>
			{/each}
		{/if}
	</Section>
{/snippet}

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

{#if mine}
	{@render record(m.security_yours(), m.security_yours_says({ days: mine.kept_for_days }), mine)}
{/if}
{#if instance}
	{@render record(
		m.settings_instance(),
		m.security_instance_says({ days: instance.kept_for_days }),
		instance,
	)}
{/if}
