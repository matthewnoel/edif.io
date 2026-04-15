import adapter from '@sveltejs/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter(),
		version: {
			name: process.env.PUBLIC_BUILD_ID ?? String(Date.now()),
			pollInterval: 5 * 60 * 1000
		}
	}
};

export default config;
