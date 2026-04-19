<script lang="ts">
  import type { PublishState } from '$lib/api/types';

  let { state }: { state: PublishState } = $props();

  const labels: Record<PublishState, string> = {
    PROPOSED: 'Awaiting review',
    REJECTED: 'Rejected',
    NEEDS_SIGNATURE: 'Awaiting approval',
    SIGNED: 'Scheduled',
    SCHEDULED: 'Scheduled',
    PUBLISHING: 'Publishing',
    PUBLISHED: 'Published',
    FAILED: 'Failed',
    CANCELLED: 'Cancelled'
  };
</script>

<span class="badge" data-state={state}>{labels[state]}</span>
