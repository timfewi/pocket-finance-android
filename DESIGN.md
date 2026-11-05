# Receipt-first design

Implementation direction for this milestone: warm paper backgrounds, graphite/green text and a restrained emerald action color. The receipt is the visual anchor; money remains prominent and always accompanied by EUR. No remote fonts, image services, ornamental animation, invented balances or cloud-account sign-in.

The overview starts empty. On Android the two entry points open the system camera and Photo Picker. Review shows the image, editable fields, possible totals and an explicit confirmation checkbox. Save status comes from the successful Rust command. A failed/missing category model is an optional-helper state, not a blocker. Discard requires a separate confirmation.

Browser preview is labelled synthetic and has standard, ambiguous and OCR-failure scenarios. Its fallback storage is memory-only. Development can use the local Rust fixture bridge; that HTTP bridge is compiled out of production bundles. It must never be described as device OCR verification.

UI primitives are generated from shadcn-svelte1.8.0's public `nova` registry (Button, Input, Label, Badge, Card), with local semantic tokens and application layout. Icons use @lucide/svelte. Svelte5 runes, SvelteKit2.70 static SPA, Tailwind4 and TypeScript6 are pinned in package-lock.json. All user/OCR text uses Svelte escaped interpolation. Preview URLs accept only the native receipt protocol or explicit local demo fixtures.

## Mobile-first layout contract

The phone is the primary layout and interaction target, including the Solana
Seeker. Start with a single column and bottom navigation for **Übersicht, Erfassen,
Belege**. The capture tab offers camera/import and resumes an existing draft.
Switching sections or using the brand button preserves edits in the running app;
only explicit discard removes the draft. This is not process-recreation recovery.

Use at least 48×48 CSS pixel action targets and 16px form text. Keep labels visible,
allow long merchant names and amounts to wrap, and reserve scrolling space above
the bottom bar. `src/mobile-first.css` contains the phone defaults and adds wider
layouts with minimum-width queries. The sidebar replaces bottom navigation at
1024px. Review can place the image beside the form when space permits; the phone
keeps them in one column. These are layout breakpoints, not device-identification
rules or claims about the Seeker's configured display density.

Android applies system-bar, cutout and IME insets to the native WebView bounds and
consumes them once. CSS safe-area variables remain available for browser layouts;
do not add a guessed keyboard-height offset. In a short viewport with an active
form field, the bottom navigation hides and resize handling keeps that field
reachable. Verify landscape, font scaling and actual keyboard behavior on devices.

The same Svelte app adapts to desktop widths and supports mouse/keyboard use.
Desktop native file selection and OCR remain future work; browser fixtures do not
establish a functioning desktop receipt scanner.
