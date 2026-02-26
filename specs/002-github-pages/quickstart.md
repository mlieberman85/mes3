# Quickstart: GitHub Pages Deployment

## Local Verification

### 1. Production Build

From the `web/` directory:

```bash
cd web
npm run build
```

This runs `build:wasm:release` (Rust → WASM with wasm-opt optimization) then `vite build` (TypeScript → bundled JS/CSS/HTML). Output goes to `web/dist/`.

### 2. Preview with Base Path

```bash
npx vite preview --base /mes3/
```

Visit `http://localhost:4173/mes3/` and verify:
- Emulator UI loads with all controls
- Load a ROM — game renders and runs at ~60 FPS
- Audio plays (click Mute/Unmute to confirm)
- Save state to a slot, reload page, restore from slot
- Open Settings — key mappings display and rebind works
- Check browser console for zero 404 errors on assets

### 3. Verify Asset Paths

Open browser DevTools Network tab on the preview. Confirm all requests use the `/mes3/` prefix:
- `index.html`
- JS bundles in `/mes3/assets/`
- WASM binary in `/mes3/assets/`
- Audio worklet JS in `/mes3/assets/`
- CSS in `/mes3/assets/`

## Deployment Verification

After the GitHub Actions workflow runs:

1. Visit `https://mlieberman85.github.io/mes3/`
2. Run the same functional checks as local preview above
3. Verify the Actions run completed in under 10 minutes
4. Push a trivial change to `main` and confirm the site updates automatically
