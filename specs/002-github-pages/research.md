# Research: GitHub Pages Deployment

## R1: Vite Base Path Configuration

**Decision**: Set `base: '/mes3/'` in `vite.config.ts` for production builds.

**Rationale**: Vite's `base` option rewrites all asset URLs (JS imports, CSS `url()`, HTML references, WASM files) at build time to include the configured prefix. GitHub Pages for project repositories serves content under `/<repo-name>/`, so this must match exactly.

**Alternatives considered**:
- `base: './'` (relative base) — works but less predictable for audio worklet URLs that use `new URL()` pattern; explicit subpath is safer and matches the known deployment target.
- No base configuration — would result in all assets loading from root `/`, causing 404s on GitHub Pages.

## R2: Audio Worklet File Handling in Production

**Decision**: The existing `new URL('./audio-worklet.ts', import.meta.url)` pattern is handled correctly by Vite in production builds. No changes needed to audio.ts.

**Rationale**: Vite recognizes `new URL('./file', import.meta.url)` as a static asset reference pattern. At build time, it compiles the TypeScript file, emits it as a separate hashed asset in `dist/assets/`, and rewrites the URL to include the `base` prefix. The string must be a static literal (which it already is).

**Alternatives considered**:
- Moving audio-worklet.ts to `public/` as a pre-compiled JS file — unnecessary since Vite handles the transform automatically; would also lose TypeScript checking during development.

## R3: COOP/COEP Headers on GitHub Pages

**Decision**: The emulator does not use SharedArrayBuffer, so COOP/COEP headers are not required. The existing `server.headers` config is dev-server-only and has no effect on the production build. No workaround needed.

**Rationale**: Reviewing the codebase confirms:
- Audio uses `AudioWorkletNode` with `postMessage` transfer (not SharedArrayBuffer)
- Rendering uses `putImageData` on Canvas 2D (not SharedArrayBuffer)
- WASM module is single-threaded (no WASM threads)

The `server.headers` in `vite.config.ts` setting COOP/COEP only applies to the Vite dev server and is ignored during `vite build`.

**Alternatives considered**:
- Using `coi-serviceworker` to inject COOP/COEP headers via service worker — unnecessary since no feature requires SharedArrayBuffer. Can be added later if WASM threads are introduced.

## R4: vite-plugin-wasm Production Behavior

**Decision**: Keep existing `vite-plugin-wasm` configuration with `build.target: 'esnext'`. The WASM binary will be emitted as a separate hashed asset file.

**Rationale**: The NES WASM binary is well over 4KB (Vite's inline threshold), so it will always be emitted as a separate file in `dist/assets/`. The plugin rewrites the import to use `WebAssembly.instantiateStreaming` with the correct base-prefixed URL. The `target: 'esnext'` setting avoids the need for `vite-plugin-top-level-await`.

**Alternatives considered**:
- Setting `assetsInlineLimit: 0` explicitly — unnecessary since the WASM binary naturally exceeds the threshold.

## R5: GitHub Actions Workflow Design

**Decision**: Use the official GitHub Pages deployment actions: `actions/upload-pages-artifact@v4` + `actions/deploy-pages@v4`. Single job that builds WASM, bundles frontend, and deploys.

**Rationale**: This is the officially recommended approach from both Vite docs and GitHub docs. It requires the repo's Pages settings to use "GitHub Actions" as the source (not a branch). The workflow needs `pages: write` and `id-token: write` permissions.

**Key CI steps**:
1. Checkout code
2. Install Rust toolchain (via `dtolnay/rust-toolchain@stable`)
3. Install wasm-bindgen-cli and binaryen (wasm-opt)
4. Install Node.js and npm dependencies (via `actions/setup-node`)
5. Build WASM (release) + Vite production bundle (`npm run build` from `web/`)
6. Upload `web/dist/` as Pages artifact
7. Deploy to GitHub Pages

**Alternatives considered**:
- Building to a `gh-pages` branch — older pattern, requires push access, more error-prone with force-pushes.
- Separate build and deploy jobs — adds complexity with artifact passing; single job is simpler for this project size.
