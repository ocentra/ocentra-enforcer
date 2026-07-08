# Parity Gaps — rules-frontend

Scope: ADBP `rules/react-nextjs/*.md` + `rules/vite-react/*.md`.
Registry: `C:/Projects/ocentra-enforcer/rules/rules.json` has **no React/frontend rule family** (prefixes are backend/infra/TS-generic: ARCH, BOUND, TS, SEC, TEST…). All frontend normative rules below are unbacked unless noted. Only gaps listed.

Tier legend: T1 deterministic/blocking (AST/regex/glob detectable) · T2 scored/advisory · T3 not mechanically testable (labeled).

| ADBP point | ADBP source | Backed? | Tier | Proposed ruleId | Fail-fixture | Pass-fixture |
|---|---|---|---|---|---|---|
| `app/` route files contain only route segments (page/layout/loading/error/route); no fetch/business logic in page.tsx | react-nextjs/architecture | NO | T1 | FE-ARCH-1.1 | `app/orders/page.tsx` with `fetch(` + `.filter(` in body | page.tsx that only imports+mounts a feature component |
| Pages kept under 50 lines | react-nextjs/architecture | NO | T2 | FE-ARCH-1.2 | 90-line page.tsx | 30-line page.tsx |
| Features MUST NOT import from other features | react-nextjs/architecture; vite-react/architecture | NO | T1 | FE-ARCH-1.3 | `features/orders/*` importing `@/features/auth/...` | orders importing only `@/lib`, `@/components` |
| `components/` are stateless/presentational: no data fetching, no business logic | react-nextjs/architecture | NO | T2 | FE-ARCH-1.4 | `components/ui/Button.tsx` calling `useQuery`/`fetch` | Button taking data via props only |
| `hooks/` must not make direct API calls (delegate to services) | react-nextjs/architecture | NO | T1 | FE-ARCH-1.5 | shared hook with `fetch(`/`axios.` | hook calling a service fn |
| Server data must NOT live in Zustand/`useState`; use TanStack Query | react-nextjs/architecture; state-patterns | NO | T1 | FE-STATE-1.1 | Zustand store field `orders: Order[]` populated from API | store holds only UI flags |
| Default to Server Components; `"use client"` only for interactivity | react-nextjs/architecture | NO | T2 | FE-ARCH-1.6 | `"use client"` file with zero hooks/handlers/browser APIs | client file that uses `onClick`/`useState` |
| Push `"use client"` boundary down (smallest interactive piece) | react-nextjs/architecture | NO | T3 (locality judgment) | FE-ARCH-1.7 | — | — |
| Never pass functions as props from Server → Client components | react-nextjs/architecture | NO | T3 (needs SC/CC graph) | FE-ARCH-1.8 | — | — |
| Client data fetching must use TanStack Query, never `useEffect`+`useState` | react-nextjs/architecture; components; state-patterns | NO | T1 | FE-STATE-1.2 | `useEffect(()=>{ fetch(...).then(setData) },[])` | `useQuery({queryKey,queryFn})` |
| Component file SHOULD NOT exceed 150 lines | react-nextjs/components | NO | T2 | FE-CMP-1.1 | 200-line component file | 100-line file |
| Component function SHOULD NOT exceed 80 lines of JSX | react-nextjs/components | NO | T2 | FE-CMP-1.2 | return block >80 lines | small return |
| One exported component per file | react-nextjs/components | NO | T1 | FE-CMP-1.3 | two `export function` components in one file | single exported component |
| File name matches component name (PascalCase file → same export) | react-nextjs/components | NO | T1 | FE-CMP-1.4 | `orderCard.tsx` exporting `OrderCard` | `OrderCard.tsx` exporting `OrderCard` |
| Avoid prop drilling beyond 2 levels | react-nextjs/components | NO | T3 (cross-tree depth) | FE-CMP-1.5 | — | — |
| Props defined as named type/interface, not inline | react-nextjs/components; typing-style | NO | T1 | FE-CMP-1.6 | `function C({x}:{x:string})` inline props | `type CProps={...}; function C(p:CProps)` |
| Props type named `<ComponentName>Props` | react-nextjs/components | NO | T1 | FE-CMP-1.7 | `type Foo` used as OrderCard props | `type OrderCardProps` |
| Use JS defaults in destructuring, not `defaultProps` | react-nextjs/components | NO | T1 | FE-CMP-1.8 | `Component.defaultProps = {...}` | default in destructure `{x=false}` |
| Never call hooks conditionally | react-nextjs/components | NO (react-hooks/rules-of-hooks) | T1 | FE-HOOK-1.1 | `if(cond){ useState() }` | hooks at top level |
| `useEffect` is last resort; every `useEffect` needs a WHY comment | react-nextjs/components | NO | T1 | FE-HOOK-1.2 | `useEffect(...)` with no preceding comment | `useEffect` with `// why:` comment |
| >3 `useState` calls signals need for hook/reducer | react-nextjs/components | NO | T2 | FE-HOOK-1.3 | component with 5 `useState` | ≤3 or custom hook |
| No nested ternaries in JSX | react-nextjs/components | NO | T1 | FE-CMP-1.9 | `a ? b : c ? d : e` in return | early returns |
| Handlers named `handle<Event>`; props `on<Event>` | react-nextjs/components | NO | T2 | FE-CMP-1.10 | `const clickIt = ...` bound to onClick | `const handleClick` |
| No non-trivial inline arrow functions in JSX | react-nextjs/components | NO | T2 | FE-CMP-1.11 | `onClick={()=>{ 5+ lines }}` | extracted handler |
| `React.memo`/`useMemo`/`useCallback` only with measured need | react-nextjs/components | NO | T3 (needs profiling) | FE-PERF-1.1 | — | — |
| Images must use `next/image` with explicit width/height | react-nextjs/components | NO | T1 | FE-CMP-1.12 | raw `<img src>` / `next/image` w/o width | `<Image width height>` |
| Each feature route wrapped with `error.tsx`/ErrorBoundary | react-nextjs/components; patterns | NO | T1 | FE-PAT-1.1 | route dir with page.tsx, no error.tsx | route dir with error.tsx |
| Never show raw error objects/stack traces to users | react-nextjs/components; patterns | NO | T2 | FE-PAT-1.2 | JSX rendering `{error.stack}`/`{String(error)}` | rendering friendly message |
| Route-level `loading.tsx` / Suspense for loading states | react-nextjs/patterns | NO | T2 | FE-PAT-1.3 | async route dir with no loading.tsx/Suspense | loading.tsx present |
| API services throw typed error classes (e.g. ApiError), not raw `Error` | react-nextjs/patterns | NO | T1 | FE-PAT-1.4 | `throw new Error(...)` in services/ | `throw new ApiError(...)` |
| Forms use React Hook Form + Zod; one shared schema | react-nextjs/patterns; components | NO | T2 | FE-FORM-1.1 | `<form>` with manual `useState` fields | `useForm({resolver:zodResolver})` |
| Disable submit during submission | react-nextjs/patterns | NO | T2 | FE-FORM-1.2 | submit button no `disabled={isSubmitting}` | disabled while submitting |
| Pagination/filter/sort stored in URL search params, not component state | react-nextjs/patterns; state-patterns | NO | T2 | FE-STATE-1.3 | `useState` for `page`/`filters` | `nuqs`/`useSearchParams` |
| Debounce search/filter inputs (300ms) | react-nextjs/patterns | NO | T2 | FE-FORM-1.3 | filter input calling API on each keystroke | debounced input |
| Protect routes via middleware, not client-side checks | react-nextjs/patterns | NO | T2 | FE-AUTH-1.1 | client `if(!user) redirect` as sole guard | `middleware.ts` matcher present |
| Single shared API client in `lib/api-client.ts`; no scattered fetch | react-nextjs/patterns | NO | T1 | FE-PAT-1.5 | `fetch(`/`axios.` inside feature service bypassing client | service imports shared `apiClient` |
| Env validated with Zod at startup; never access `process.env` directly | react-nextjs/patterns; typing-style | PARTIAL (SEC/CFG env families exist but not client-var Zod-schema pattern — verify) | T1 | FE-CFG-1.1 | `process.env.X` outside `lib/env.ts` | typed `env` import |
| Client-exposed vars prefixed `NEXT_PUBLIC_` / `VITE_` | react-nextjs/patterns; vite-react/tooling | NO | T1 | FE-CFG-1.2 | client code reading `process.env.API_URL` | `NEXT_PUBLIC_`/`VITE_` prefix |
| Interactive elements keyboard-accessible; use semantic HTML | react-nextjs/patterns | NO | T2 | FE-A11Y-1.1 | `<div onClick>` acting as button | `<button>` |
| All images have `alt` text | react-nextjs/patterns | NO | T1 | FE-A11Y-1.2 | `<img>`/`<Image>` without `alt` | with `alt` |
| Form inputs have associated labels (`<label>`/`aria-label`) | react-nextjs/patterns | NO | T1 | FE-A11Y-1.3 | `<input>` with no label/aria-label | labeled input |
| Color never the only indicator | react-nextjs/patterns | NO | T3 (visual judgment) | FE-A11Y-1.4 | — | — |
| Screen-reader test per major feature | react-nextjs/patterns | NO | T3 (manual) | FE-A11Y-1.5 | — | — |
| NEVER use `useContext` for frequently-changing state | react-nextjs/state-patterns | NO | T3 (churn judgment) | FE-STATE-1.4 | — | — |
| `useReducer` over `useState` when transitions complex/interdependent | react-nextjs/state-patterns | NO | T3 | FE-STATE-1.5 | — | — |
| One Zustand store per domain; no god store; actions defined inside store | react-nextjs/state-patterns | NO | T2 | FE-STATE-1.6 | single store with 20+ unrelated fields / external mutators | domain store, actions in `create` |
| TanStack query keys as const arrays in central `query-keys.ts` | react-nextjs/state-patterns | NO | T2 | FE-STATE-1.7 | inline `queryKey:["orders",id]` literal | keys from `orderKeys` factory |
| Wrap useQuery/useMutation in custom hooks (`useOrders`) | react-nextjs/state-patterns | NO | T2 | FE-STATE-1.8 | `useQuery` called directly in component | called inside `use*` hook |
| Invalidate queries after mutation (not manual cache edits) | react-nextjs/state-patterns | NO | T2 | FE-STATE-1.9 | `useMutation` w/ manual `setQueryData` only | `onSuccess` invalidateQueries |
| **FSM MANDATORY for any UI flow with 3+ states + constrained transitions** (wizards, entity lifecycle, auth flow, retry) | react-nextjs/state-patterns | NO | T2 | FE-FSM-1.1 | multi-step wizard driven by `useState('step')` + if-chains | XState/`canTransition` FSM |
| FSM transitions defined explicitly (states→transitions map) | react-nextjs/state-patterns | NO | T1 | FE-FSM-1.2 | ad-hoc `setStatus` string mutation | explicit transition table `as const` |
| Test names read as specs: `it("should <behavior> when <scenario>")` | react-nextjs/testing; vite-react/testing | NO | T2 | FE-TEST-1.1 | `it("sets isLoading true")` | `it("should show empty state when no orders")` |
| Query by role/label/text, not class/id/test-id | react-nextjs/testing | NO | T2 | FE-TEST-1.2 | `container.querySelector('.btn')` | `getByRole('button')` |
| Never assert on internal state (useState/hook internals) | react-nextjs/testing | NO | T2 | FE-TEST-1.3 | assert on `result.current.isLoading` internal | assert visible output |
| Never assert on className/inline styles | react-nextjs/testing | NO | T1 | FE-TEST-1.4 | `expect(el).toHaveClass('active')` | role/text assertion |
| MSW for API mocking at network level (not `jest.mock` of fns) | react-nextjs/testing | NO | T2 | FE-TEST-1.5 | `vi.mock('./getOrders')` | MSW `http.get` handler |
| FSM tests cover all valid + all invalid transitions | react-nextjs/testing | NO | T2 | FE-TEST-1.6 | FSM with test for happy path only | tests asserting invalid transitions rejected |
| Co-locate unit tests next to source (`x.tsx`→`x.test.tsx`) | react-nextjs/testing | NO | T2 | FE-TEST-1.7 | component with test only under `/tests` mirror | sibling `.test.tsx` |
| Package manager pnpm/bun only; never npm or yarn | react-nextjs/tooling; vite-react/tooling | PARTIAL (NPM-* family exists; verify it forbids yarn & mandates pnpm/bun for FE) | T1 | FE-TOOL-1.1 | repo has `package-lock.json`/`yarn.lock` | `pnpm-lock.yaml`/`bun.lockb` |
| Lockfile committed | tooling | PARTIAL (may overlap DEP/NPM) | T1 | FE-TOOL-1.2 | no lockfile committed | lockfile present |
| ESLint extends strict-type-checked; `no-explicit-any` error; `no-unused-vars` error; `import/order`; `react-hooks/exhaustive-deps` error | react-nextjs/tooling; vite-react/tooling | PARTIAL (a TS ESLint rule mentions no-explicit-any/no-floating-promises; react-hooks/exhaustive-deps + import/order + strict-type-checked in FE config NOT backed) | T1 | FE-TOOL-1.3 | eslint config missing `react-hooks/exhaustive-deps`/`import/order`/strict-type-checked | config enabling all |
| `strict: true` in tsconfig | typing-style | PARTIAL (TS family may cover; verify FE tsconfig scope) | T1 | FE-TS-1.1 | tsconfig `strict:false`/absent | `strict:true` |
| `noUncheckedIndexedAccess: true` | typing-style | PARTIAL (registry mentions noUncheckedIndexedAccess — verify it enforces =true) | T1 | FE-TS-1.2 | flag absent/false | true |
| `exactOptionalPropertyTypes: true` when possible | typing-style | NO | T2 | FE-TS-1.3 | flag absent | true |
| All fn params AND return types explicitly typed | typing-style | NO | T2 | FE-TS-1.4 | exported fn with inferred return | explicit return type |
| Never use `any`; use `unknown`+guards; disable-comment must justify | typing-style | PARTIAL (no-explicit-any in ESLint snippet only; the justify-comment rule not backed) | T1 | FE-TS-1.5 | `: any` with no eslint-disable+reason | `unknown` or justified disable |
| Use `as const` for literals/readonly arrays; `satisfies` for typed literals | typing-style | NO | T2 | FE-TS-1.6 | mutable const array of literals | `as const` / `satisfies` |
| Use `as const` object / union types instead of TS `enum` | typing-style | PARTIAL (registry has 20 "enum" hits — verify none forbid TS enum in TSX) | T1 | FE-TS-1.7 | `enum OrderStatus {}` in .ts | `const OrderStatus = {...} as const` |
| Naming: PascalCase types/components, camelCase fns/vars, UPPER_SNAKE consts, kebab-case files | typing-style | NO | T1 | FE-TS-1.8 | `MyComponent.ts` file `snake_var` | conventions followed |
| No `I` prefix on interfaces; prefer `type` unless extending | typing-style | NO | T1 | FE-TS-1.9 | `interface IUser` | `type User` |
| Boolean props/vars use is/has/can/should prefix | typing-style | NO | T2 | FE-TS-1.10 | `const active = true` prop | `isActive` |
| Zod schemas as single source of truth; derive types with `z.infer`; validate at boundaries | typing-style | CONFLICT (registry TS-rule *forbids* direct Zod, mandates Effect Schema — direct contradiction with ADBP FE) | T3 (policy conflict — needs reconciliation, not a mechanical rule) | FE-TS-1.11 | — | — |
| Absolute imports with `@/` alias | typing-style | NO | T1 | FE-TS-1.12 | `import '../../lib/x'` deep relative | `@/lib/x` |
| Import grouping order (react→3rd-party→@/lib→@/components→@/features→relative) | typing-style | NO (import/order not FE-configured) | T2 | FE-TS-1.13 | ungrouped imports | grouped per order |
| Type-only imports use `import type` | typing-style | NO | T1 | FE-TS-1.14 | `import { User }` used only as type | `import type { User }` |
| Never use `require()` | typing-style | NO | T1 | FE-TS-1.15 | `const x = require('y')` | ESM `import` |
| (Vite) `@/` alias via vite-tsconfig-paths; source maps off in prod; vite-plugin-checker in dev | vite-react/tooling | NO | T2 | FE-VITE-1.1 | vite.config missing tsconfig-paths / prod sourcemap:true | configured |
| (Vite) DataTable built on TanStack Table for complex tables; server-side pagination + URL params | vite-react/architecture | NO | T3 (design guidance) | FE-VITE-1.2 | — | — |
| (Vite) Role-based access gated via `<Can>`/`useCan`; permissions as const; enforce server-side too | vite-react/architecture | PARTIAL (registry has generic "permission" hits — not this FE gating pattern) | T2 | FE-VITE-1.3 | conditional UI on raw `role==='admin'` string | `<Can permission=...>` gate |
| (Vite) Prefer invalidation over optimistic updates for internal tools | vite-react/architecture | NO | T3 (judgment) | FE-VITE-1.4 | — | — |
| (Vite) Toast system for mutation feedback (sonner) | vite-react/architecture; tooling | NO | T2 | FE-VITE-1.5 | mutation with no success/error toast | `onSuccess`/`onError` toast |

## Notes
- Entire ADBP frontend surface is a greenfield gap: **no FE rule family (`FE-*`) exists** in rules.json today.
- Highest-value T1 blockers: FE-STATE-1.1/1.2 (server data placement + useEffect fetching), FE-ARCH-1.3 (cross-feature imports), FE-PAT-1.4 (typed errors), FE-A11Y-1.2/1.3 (alt/labels), FE-CMP-1.12 (next/image), FE-FSM-1.2 (explicit transitions), FE-TS-1.12/1.14/1.15 (imports).
- **CONFLICT to escalate**: ADBP mandates Zod as SoT; registry rule forbids direct Zod (Effect Schema mandated). Cannot back FE-TS-1.11 as-is — needs a policy decision on whether frontend is exempt.
- PARTIAL rows need verification against the exact TS/NPM/CFG/SEC rule scope (do their globs/paths actually cover `.tsx` FE code, or only backend TS?).
