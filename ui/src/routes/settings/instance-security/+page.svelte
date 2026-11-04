<!--
	The instance's security record (#371, #469): an Operator's page of This
	instance. OperatorOnly asks first, so a cook is never sent to ask the
	instance's record and be refused (#533).
-->
<script lang="ts">
	import { m } from '#lib/paraglide/messages.js';
	import OperatorOnly from '../OperatorOnly.svelte';
	import SecurityRecord from '../SecurityRecord.svelte';
</script>

<OperatorOnly title={m.settings_instance_record()}>
	<SecurityRecord whose="instance" />
</OperatorOnly>
