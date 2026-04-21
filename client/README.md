# client

SvelteKit client for edif.io.

For full monorepo setup and local run instructions, see the root `README.md`.

## Local client development

```sh
npm install
npm run dev
```

## Quality checks

```sh
npm run check
npm run lint
npm run test:unit -- --run
npm run test:e2e
```

`test:e2e` uses Playwright against a production preview build; the first
run downloads browsers via `npx playwright install chromium`.
