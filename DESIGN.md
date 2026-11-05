# Receipt-first design

Implementation direction for this milestone: warm paper backgrounds, graphite/green text and a restrained emerald action color. The receipt is the visual anchor; money remains prominent and always accompanied by EUR. No remote fonts, image services, ornamental animation, invented balances or cloud-account sign-in.

The overview starts empty. On Android the two entry points open the system camera and Photo Picker. Review shows the image, editable fields, possible totals and an explicit confirmation checkbox. Save status comes from the successful Rust command. A failed/missing category model is an optional-helper state, not a blocker. Discard requires a separate confirmation.

Browser preview is labelled synthetic and has standard, ambiguous and OCR-failure scenarios. Its fallback storage is memory-only. Development can use the local Rust fixture bridge; that HTTP bridge is compiled out of production bundles. It must never be described as device OCR verification.

UI primitives are generated from shadcn-svelte1.8.0's public `nova` registry (Button, Input, Label, Badge, Card), with local semantic tokens and application layout. Icons use @lucide/svelte. Svelte5 runes, SvelteKit2.70 static SPA, Tailwind4 and TypeScript6 are pinned in package-lock.json. All user/OCR text uses Svelte escaped interpolation. Preview URLs accept only the native receipt protocol or explicit local demo fixtures.
