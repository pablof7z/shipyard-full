<script lang="ts">
  import {
    BlossomUploadError,
    resolveBlossomServers,
    uploadBlobToBlossom
  } from '$lib/nostr/blossom';

  let { onInsertUrl }: { onInsertUrl: (url: string) => void } = $props();

  let directServerText = $state('');
  let manualUrl = $state('');
  let selectedFile = $state<File | null>(null);
  let uploading = $state(false);
  let message = $state('');
  let error = $state('');

  function setMessage(value: string) {
    message = value;
    error = '';
  }

  function setError(err: unknown, fallback: string) {
    if (err instanceof BlossomUploadError) {
      error = err.message;
    } else {
      error = err instanceof Error ? err.message : fallback;
    }
    message = '';
  }

  function directServers() {
    return directServerText
      .split(/[\n,]/)
      .map((server) => server.trim())
      .filter(Boolean);
  }

  function pickFile(event: Event) {
    selectedFile = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
  }

  function insertManualUrl() {
    if (!manualUrl.trim()) {
      return;
    }

    onInsertUrl(manualUrl.trim());
    manualUrl = '';
    setMessage('Image added.');
  }

  async function uploadSelectedFile() {
    if (!selectedFile) {
      error = 'Choose a file first.';
      message = '';
      return;
    }

    uploading = true;
    try {
      const result = await uploadBlobToBlossom({
        blob: selectedFile,
        signer: window.nostr,
        servers: directServers()
      });
      onInsertUrl(result.descriptor.url);
      setMessage('Image added.');
    } catch (err) {
      setError(err, 'Upload failed.');
    } finally {
      uploading = false;
    }
  }
</script>

<div class="card-form">
  <div class="section-header">
    <h2>Media</h2>
  </div>

  {#if message}
    <p class="meta-line success-text">{message}</p>
  {/if}
  {#if error}
    <p class="meta-line error-text">{error}</p>
  {/if}

  <label class="field">
    <span>File</span>
    <input class="file-input" type="file" onchange={pickFile} />
  </label>

  <div class="inline-actions">
    <button
      class="primary-action"
      type="button"
      onclick={uploadSelectedFile}
      disabled={uploading || !selectedFile}
    >
      Upload
    </button>
  </div>

  <form class="inline-form" onsubmit={(event) => event.preventDefault()}>
    <input bind:value={manualUrl} placeholder="Paste image URL" />
    <button class="secondary-action" type="button" onclick={insertManualUrl}>
      Add URL
    </button>
  </form>

  <details class="advanced-details">
    <summary>Custom server (optional)</summary>
    <label class="field">
      <textarea
        bind:value={directServerText}
        rows="2"
        placeholder="https://blossom.primal.net"
      ></textarea>
    </label>
  </details>
</div>

<style>
  .advanced-details {
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .advanced-details summary {
    cursor: pointer;
    user-select: none;
    padding: 4px 0;
  }

  .advanced-details .field {
    margin-top: 8px;
  }
</style>
