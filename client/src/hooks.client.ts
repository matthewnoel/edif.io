import type { HandleClientError } from '@sveltejs/kit';

export const handleError: HandleClientError = ({ error }) => {
	const msg = error instanceof Error ? error.message : String(error);
	const isChunkError =
		/Failed to fetch dynamically imported module|Importing a module script failed|ChunkLoadError/i.test(
			msg
		);
	return {
		message: isChunkError ? 'A new version was just deployed. Please refresh.' : 'Unexpected error.'
	};
};
