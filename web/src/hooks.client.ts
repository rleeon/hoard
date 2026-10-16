import type { HandleClientError } from '@sveltejs/kit';
import { isChunkError } from '$lib/chunkReload';

// Only tags the error. This hook also runs for hover preloads that fail, and a
// preload must never reload the page under the user; `+error.svelte` is what
// acts, and it only renders for a real navigation.
export const handleError: HandleClientError = ({ error, message }) => {
  if (isChunkError(error)) return { message, chunk: true };
  return { message };
};
