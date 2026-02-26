# Tasks: GitHub Pages Deployment

**Input**: Design documents from `/specs/002-github-pages/`
**Prerequisites**: plan.md (required), spec.md (required), research.md

**Tests**: No automated tests — this is an infrastructure/deployment feature. Verification is done via local preview and post-deploy functional checks per quickstart.md.

**Organization**: Tasks grouped by user story. US3 (base path config) is foundational since US1 and US2 depend on correct asset paths.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Create directory structure for GitHub Actions

- [X] T001 Create `.github/workflows/` directory at repository root

---

## Phase 2: Foundational (Build Configuration)

**Purpose**: Configure Vite production build with correct base path — MUST complete before user story verification

**⚠️ CRITICAL**: US1 and US2 depend on correct production build output

- [X] T002 Add `base: '/mes3/'` to the Vite config in `web/vite.config.ts` so all asset URLs use the GitHub Pages subpath
- [X] T003 Run `npm run build` from `web/` and verify `web/dist/` contains HTML, JS bundles, CSS, WASM binary, and audio worklet file with correct `/mes3/` prefixed paths

**Checkpoint**: Production build generates a self-contained static site bundle with correct subpath asset references (FR-001, FR-002, FR-005)

---

## Phase 3: User Story 1 - Visit the Published Emulator (Priority: P1) 🎯 MVP

**Goal**: The emulator loads and runs correctly when served from the `/mes3/` subpath

**Independent Test**: Run `npx vite preview` from `web/` and visit `http://localhost:4173/mes3/` — emulator UI loads, ROM loading works, audio plays, save states persist

### Implementation for User Story 1

- [X] T004 [US1] Start local preview with `npx vite preview` from `web/` and verify the emulator UI loads at `/mes3/` with all controls visible (Load ROM, Reset, Mute, Save State, Load State, Settings)
- [X] T005 [US1] Verify in browser DevTools Network tab that all assets (JS, CSS, WASM binary, audio worklet) load without 404 errors from `/mes3/assets/` paths
- [X] T006 [US1] Confirm the emulator functions without COOP/COEP headers by verifying the preview server does not set those headers and audio/rendering still work (FR-006)

**Checkpoint**: Emulator fully functional on local preview at `/mes3/` subpath — same behavior as `localhost:5173` (SC-001, SC-003)

---

## Phase 4: User Story 2 - Automated Deployment on Push (Priority: P2)

**Goal**: A GitHub Actions workflow automatically builds and deploys to GitHub Pages on every push to `main`

**Independent Test**: Push to `main` and verify the Actions workflow completes and the site is live at `https://mlieberman85.github.io/mes3/`

### Implementation for User Story 2

- [X] T007 [US2] Create GitHub Actions workflow in `.github/workflows/deploy.yml` that: triggers on push to `main`, installs Rust toolchain via `dtolnay/rust-toolchain@stable`, installs `wasm-bindgen-cli` (v0.2.113) and `binaryen` (wasm-opt), sets up Node.js LTS via `actions/setup-node`, runs `npm ci` and `npm run build` from `web/`, uploads `web/dist/` via `actions/upload-pages-artifact@v4`, and deploys via `actions/deploy-pages@v4` with `pages: write` and `id-token: write` permissions (FR-003, FR-004, FR-007)
- [X] T008 [US2] Configure GitHub Pages source to "GitHub Actions" in repository settings via `gh api` or manual instruction (required for `actions/deploy-pages` to work)

**Checkpoint**: CI/CD workflow defined and repository configured for GitHub Actions-based Pages deployment (SC-002, SC-004)

---

## Phase 5: User Story 3 - Correct Asset Loading on Subpath (Priority: P3)

**Goal**: All assets resolve correctly when served from `/mes3/` rather than domain root

**Independent Test**: Inspect production build output in `web/dist/` and verify all internal references use `/mes3/` prefix; confirm zero 404s in browser

### Implementation for User Story 3

- [X] T009 [US3] Inspect `web/dist/index.html` and verify the script/link tags reference assets with `/mes3/assets/` prefix (not bare `/assets/`)
- [X] T010 [US3] Verify the audio worklet file is emitted as a separate asset in `web/dist/assets/` and the URL in the compiled JS output includes the `/mes3/` base path

**Checkpoint**: All asset references confirmed correct for subpath deployment (FR-002, FR-005)

---

## Phase 6: Polish & Validation

**Purpose**: End-to-end validation across all stories

- [X] T011 Run full quickstart.md validation checklist against local preview at `/mes3/`
- [X] T012 Verify `.gitignore` includes `web/dist/` (already present as build output)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 for directory structure
- **US1 (Phase 3)**: Depends on Phase 2 (needs correct production build)
- **US2 (Phase 4)**: Depends on Phase 1 (needs .github/workflows/ directory); can run in parallel with US1
- **US3 (Phase 5)**: Depends on Phase 2 (needs production build output to inspect)
- **Polish (Phase 6)**: Depends on Phases 3, 4, 5

### User Story Dependencies

- **User Story 1 (P1)**: Depends on Foundational (Phase 2) — base path config must be in place
- **User Story 2 (P2)**: Depends on Setup (Phase 1) — needs workflow directory; can run in parallel with US1
- **User Story 3 (P3)**: Depends on Foundational (Phase 2) — verification of build output

### Parallel Opportunities

- **T004 + T005 + T006**: All US1 verification tasks can run in parallel (all inspect the same preview)
- **US1 (Phase 3) + US2 (Phase 4)**: Can proceed in parallel after their respective prerequisites
- **T009 + T010**: Both US3 inspection tasks can run in parallel

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Create directory structure
2. Complete Phase 2: Configure Vite base path + verify build
3. Complete Phase 3: Verify emulator works on local preview at `/mes3/`
4. **STOP and VALIDATE**: Emulator fully functional on subpath

### Full Delivery

1. Setup + Foundational → production build ready
2. US1 verification → emulator confirmed working at subpath (MVP!)
3. US2 → CI/CD workflow created → automated deployment on push
4. US3 → asset path verification confirmed
5. Polish → full quickstart.md validation

---

## Notes

- Only 2 source files are modified: `web/vite.config.ts` (add base path) and `.github/workflows/deploy.yml` (new file)
- All other web source files (`main.ts`, `audio.ts`, `renderer.ts`, etc.) require NO changes
- The audio worklet `new URL('./audio-worklet.ts', import.meta.url)` pattern is already Vite-compatible for production (research R2)
- COOP/COEP headers are dev-server-only and not needed for production (research R3)
