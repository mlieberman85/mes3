# Feature Specification: GitHub Pages Deployment

**Feature Branch**: `002-github-pages`
**Created**: 2026-02-25
**Status**: Draft
**Input**: User description: "I want to publish this project as a github pages. So take what would work on localhost:5173 and get it publishable as something on github pages"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Visit the Published Emulator (Priority: P1)

A user navigates to the GitHub Pages URL (e.g., `https://mlieberman85.github.io/mes3/`) in their browser and sees the fully functional NES emulator. They can load a ROM file, play the game with keyboard input, hear audio, and use save states — the same experience they would get running the project locally via `localhost:5173`.

**Why this priority**: This is the core deliverable. Without the emulator being accessible and fully functional on the public URL, no other story matters.

**Independent Test**: Can be fully tested by visiting the published URL and loading a ROM file — the emulator renders frames, accepts input, plays audio, and save states work.

**Acceptance Scenarios**:

1. **Given** the site is deployed, **When** a user visits the GitHub Pages URL, **Then** the emulator UI loads with all controls visible (Load ROM, Reset, Mute, Save State, Load State, Settings).
2. **Given** the emulator UI is loaded, **When** the user loads a valid `.nes` ROM file, **Then** the game runs at approximately 60 FPS with correct video rendering and audio playback.
3. **Given** a ROM is loaded and running, **When** the user presses keyboard keys mapped to NES buttons, **Then** the emulator responds to input correctly.
4. **Given** a ROM is loaded, **When** the user saves to a slot and later reloads the page, **Then** the save state persists in IndexedDB and can be restored.

---

### User Story 2 - Automated Deployment on Push (Priority: P2)

A developer pushes a commit to the `main` branch, and GitHub Actions automatically builds the WASM module, bundles the web frontend, and deploys the result to GitHub Pages without any manual steps.

**Why this priority**: Automation ensures the published site stays up-to-date with the latest code changes. Without CI/CD, deployment would require manual steps that are error-prone and tedious.

**Independent Test**: Can be tested by pushing a trivial change (e.g., updating a status message string) to `main` and verifying the change appears on the published site within a few minutes.

**Acceptance Scenarios**:

1. **Given** a commit is pushed to `main`, **When** the GitHub Actions workflow runs, **Then** it installs Rust, wasm-bindgen-cli, wasm-opt, and Node.js dependencies, builds the WASM module, bundles the frontend, and deploys to GitHub Pages.
2. **Given** the workflow completes successfully, **When** a user visits the GitHub Pages URL, **Then** the site reflects the latest committed code.
3. **Given** the WASM build or frontend bundle fails, **When** the workflow encounters an error, **Then** the deployment does not proceed and the previous working version remains live.

---

### User Story 3 - Correct Asset Loading on Subpath (Priority: P3)

The deployed site loads all assets (WASM binary, JavaScript modules, CSS, audio worklet) correctly when served from a repository subpath (e.g., `/mes3/`) rather than the domain root.

**Why this priority**: GitHub Pages for project repositories serve content under `/<repo-name>/`. If asset paths are hardcoded as absolute from root, the site breaks. This is a prerequisite that directly supports User Story 1, but is separated because it involves specific build configuration concerns.

**Independent Test**: Can be tested by running the production build locally with a simulated base path and verifying all assets load without 404 errors.

**Acceptance Scenarios**:

1. **Given** the site is deployed at `https://mlieberman85.github.io/mes3/`, **When** the browser loads the page, **Then** all JavaScript modules, the WASM binary, CSS files, and the audio worklet file resolve correctly without 404 errors.
2. **Given** the Vite build is configured with a base path, **When** the production bundle is generated, **Then** all internal asset references use the correct subpath prefix.

---

### Edge Cases

- What happens when a user visits the GitHub Pages URL before the first deployment completes? They see GitHub's default 404 page.
- What happens if the WASM binary fails to load due to missing COOP/COEP headers? The emulator shows an error message; note that GitHub Pages does not support custom response headers, so SharedArrayBuffer-dependent features may need alternative approaches.
- What happens if a user's browser does not support WebAssembly? The emulator displays a meaningful error message indicating WASM support is required.
- What happens if the audio worklet file path is incorrect on the subpath? Audio initialization fails silently and the emulator runs without sound; the worklet path must be base-path-aware.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The production build MUST generate a self-contained static site bundle (HTML, JS, CSS, WASM) deployable to any static hosting service.
- **FR-002**: The build process MUST configure asset paths relative to the repository subpath (`/mes3/`) so all resources load correctly on GitHub Pages.
- **FR-003**: A CI/CD workflow MUST automatically build and deploy the site to GitHub Pages when changes are pushed to the `main` branch.
- **FR-004**: The CI/CD workflow MUST install all required build tools (Rust toolchain, wasm-bindgen-cli, wasm-opt, Node.js) as part of the pipeline.
- **FR-005**: The audio worklet file MUST be included in the production build output and referenced with the correct base path.
- **FR-006**: The deployed site MUST function without custom server-side response headers (no reliance on COOP/COEP headers that GitHub Pages cannot provide).
- **FR-007**: The CI/CD workflow MUST NOT deploy if the build step fails, preserving the last successful deployment.

### Assumptions

- The target repository is `mlieberman85/mes3` on GitHub.
- GitHub Pages is configured to deploy from GitHub Actions (not from a branch).
- The WASM build uses `wasm-bindgen` v0.2.113 and `wasm-opt` (binaryen) for production optimization.
- The Vite-based frontend build already works locally and only needs base path configuration for production.
- SharedArrayBuffer is not strictly required for core emulator functionality; the emulator can function without COOP/COEP headers by avoiding SharedArrayBuffer usage.

### Scope

**In scope**:
- Vite production build configuration with correct base path
- GitHub Actions workflow for automated build and deployment
- Audio worklet file handling in production builds
- WASM build integration in CI pipeline

**Out of scope**:
- Custom domain configuration
- CDN or caching optimization
- Server-side rendering
- Analytics or visitor tracking
- Progressive Web App (PWA) features

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can visit `https://mlieberman85.github.io/mes3/` and see the emulator UI load successfully with zero console errors related to missing assets.
- **SC-002**: The full deployment pipeline (push to `main` through site live) completes in under 10 minutes.
- **SC-003**: All emulator features (ROM loading, gameplay, audio, save states, input settings) function identically on the deployed site as they do on `localhost:5173`.
- **SC-004**: The CI/CD workflow succeeds on a clean run without manual intervention or pre-installed tooling.
