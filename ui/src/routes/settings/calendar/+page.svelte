<!-- Your plan in your calendar (#491), a page of Settings' You. -->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import Screen from '#lib/shell/Screen.svelte';
	import CalendarLink from '../CalendarLink.svelte';
</script>

<Screen title={m.settings_calendar()}>
	<CalendarLink />
</Screen>
