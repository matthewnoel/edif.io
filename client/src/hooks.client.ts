import type { HandleClientError } from '@sveltejs/kit';
import { m } from '$lib/paraglide/messages';

export const handleError: HandleClientError = ({ error }) => {
	const msg = error instanceof Error ? error.message : String(error);
	const isChunkError =
		/Failed to fetch dynamically imported module|Importing a module script failed|ChunkLoadError/i.test(
			msg
		);
	return {
		message: isChunkError ? m.error_chunk_reload() : m.error_unexpected()
	};
};
