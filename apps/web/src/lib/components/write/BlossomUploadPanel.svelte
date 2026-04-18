<script lang="ts">
  import {
    BlossomUploadError,
    parseServerListJson,
    resolveBlossomServers,
    uploadBlobToBlossom
  } from '$lib/nostr/blossom';

  let { onInsertUrl }: { onInsertUrl: (url: string) => void } = $props();

  let serverEventText = $state('');
  let directServerText = $state('');
  let manualUrl = $state('');
  let selectedFile = $state<File | null>(null);
  let resolvedServers = $state(resolveBlossomServers());
  let uploading = $state(false);
  let message = $state('');
  let error = $state('');
  const serverListPlaceholder =
    '{"kind":10063,"tags":[["server","https://blossom.primal.net"]]}';

  function setMessage(value: string) {
    message = value;
    error = '';
  }

  function setError(err: unknown, fallback: string) {
    if (err instanceof BlossomUploadError) {
      error = `${err.kind}: ${err.message}`;
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

  function serverListEvents() {
    return parseServerListJson(serverEventText);
  }

  function resolveServers() {
    try {
      const direct = directServers();
      resolvedServers = direct.length ? direct : resolveBlossomServers(serverListEvents());
      setMessage('Blossom servers resolved.');
    } catch (err) {
      setError(err, 'Failed to parse Blossom server list.');
    }
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
    setMessage('Blossom URL inserted.');
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
        servers: directServers(),
        serverListEvents: serverListEvents()
      });
      onInsertUrl(result.descriptor.url);
      resolvedServers = [result.server];
      setMessage('Blossom URL inserted.');
    } catch (err) {
      setError(err, 'Blossom upload failed.');
    } finally {
      uploading = false;
    }
  }
</script>

<div class="card-form">
  <div class="section-header">
    <h2>Blossom</h2>
    <button class="secondary-action" type="button" onclick={resolveServers}>Resolve</button>
  </div>

  {#if message}
    <p class="meta-line success-text">{message}</p>
  {/if}
  {#if error}
    <p class="meta-line error-text">{error}</p>
  {/if}

  <label class="field">
    <span>Kind 10063 event JSON</span>
    <textarea
      bind:value={serverEventText}
      rows="5"
      spellcheck="false"
      placeholder={serverListPlaceholder}
    ></textarea>
  </label>

  <label class="field">
    <span>Server URLs</span>
    <textarea
      bind:value={directServerText}
      rows="3"
      placeholder="https://blossom.primal.net"
    ></textarea>
  </label>

  <div class="rows compact">
    {#each resolvedServers as server}
      <article class="row">
        <p>{server}</p>
      </article>
    {/each}
  </div>

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
    <input bind:value={manualUrl} placeholder="https://server.example/<sha256>.jpg" />
    <button class="secondary-action" type="button" onclick={insertManualUrl}>
      Insert URL
    </button>
  </form>
</div>
