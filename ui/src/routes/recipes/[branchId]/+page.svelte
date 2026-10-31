<!-- The route itself: reads the URL's Branch id, and the amount a planned recipe opens at, and hands them to Recipe.svelte, which holds everything the screen actually does. -->
<script lang="ts">
	import { page } from '$app/state';
	import { fromSearch } from '$lib/how-much';
	import Recipe from './Recipe.svelte';
</script>

{#if page.params.branchId}
	<Recipe
		branchId={page.params.branchId}
		openAt={page.url.searchParams.has('written') ? null : fromSearch(page.url.searchParams)}
	/>
{/if}
