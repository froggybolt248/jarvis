# Product

## Register

product

## Users

One person: the owner (a student/builder on Windows 11). Jarvis is their private,
local-first life assistant — calendar, diet, gym, study, notes — living as a desktop
app they glance at many times a day and talk to via a command palette. Context:
quick check-ins between tasks, not long sessions.

## Product Purpose

A single quiet dashboard over the owner's whole life, backed by a markdown vault and
a local LLM. Success = every tile shows real, current data; logging anything takes
seconds (chat or one-line form); Jarvis proactively surfaces what needs attention
(briefings, nudges) without ever interrupting.

## Brand Personality

Confident, energetic, personal. A capable aide with presence — not an austere
terminal. The owner explicitly asked for **bolder**: more color, stronger hierarchy,
more visual energy than the original near-monochrome scheme. Amber stays Jarvis's
own voice; each life domain gets its own committed hue so screens are instantly
recognizable. Boldness comes from decisive hierarchy, scale, and color roles — never
gradients-on-text, neon glow, or glassmorphism sprinkled everywhere.

## Anti-references

- The original placeholder look: timid `text-lg` titles, everything `max-w-xl` in a
  corner of a huge empty canvas, identical gray cards.
- Generic SaaS dashboards (hero-metric cards, cyan/purple gradients).
- Anything modal-heavy or notification-spammy; Jarvis never interrupts.

## Design Principles

1. **Real data or nothing.** No mock content ever renders; empty states teach the
   next action and dead-end copy (pointing at features that don't exist) is a bug.
2. **Bold hierarchy, calm behavior.** Big committed type and domain color up front;
   motion and interruptions stay minimal (150–250ms, state-driven only).
3. **One domain, one hue.** Calendar/diet/gym/study/knowledge each own a hue used
   for identity and data (icons, rings, progress) — amber is reserved for Jarvis
   itself (voice, briefing, streaming).
4. **Log in one line.** Every write path is reachable in ≤2 interactions from
   anywhere (palette or inline form), and every action echoes in the Quiet Feed.
5. **Use the canvas.** Desktop-first: screens commit to the full width with real
   layout (primary column + side rail), not a floating column.

## Accessibility & Inclusion

WCAG AA contrast (≥4.5:1 body text on all surfaces, including on-hue text);
`prefers-reduced-motion` honored on all animation; full keyboard reach (palette,
forms, nav); focus rings visible on the dark theme.
