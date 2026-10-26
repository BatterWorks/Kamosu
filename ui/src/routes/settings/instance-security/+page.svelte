<!-- The instance's security record (#371, #469): an Operator's page of This instance. -->
<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import Screen from '$lib/shell/Screen.svelte';
	import SecurityRecord from '../SecurityRecord.svelte';
</script>

<Screen title={m.settings_instance_record()}>
	<SecurityRecord whose="instance" />
</Screen>
