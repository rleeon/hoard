<script lang="ts">
  // Hoard-Wrapped entry. Free for everyone (Cloud and self-hosted), no Pro
  // gate. The recap reads its playtime from the server only: the device-merged
  // aggregate for the account (Hoard Cloud, or the user's own server when
  // self-hosted). No local read fallback, so the recap is always the account's
  // real cross-device history or empty.
  import { Wrapped } from "$pro";
</script>

<Wrapped />
