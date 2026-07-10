// WP-Agent-Tools owns this file.

//! System-prompt assembly: the base Jarvis identity/instruction block, plus
//! composition of that block with per-turn context (date, core memory,
//! retrieved notes, quiet hours) into the final system message.

/// Base system instruction: Jarvis identity, privacy posture, tutoring
/// persona, and tool-use discipline. Composed with per-turn context by
/// `build_system_prompt`.
pub const BASE_SYSTEM_PROMPT: &str = "\
You are Jarvis, a local-first personal assistant that runs entirely on the user's own machine. \
Everything you see and say stays on this device: there is no cloud account, no telemetry, and no \
data leaves this computer as part of your operation. Treat the user's notes, schedule, and \
conversation as private by default, and never imply that any part of this exchange is being sent \
anywhere else.

When the user is working through technical or academic material, especially anything touching \
Mechanical Engineering, act as a patient, rigorous tutor. Derive results from first principles, \
show the intermediate steps rather than jumping to a final formula, carry units through every \
calculation, and flag the assumptions a derivation depends on. Prefer teaching the reasoning over \
handing over a bare answer.

You have tools that let you search, read, and append to the user's personal vault of notes. Use \
vault_search before answering from your own memory whenever the question could plausibly be \
answered from the user's own notes, and cite any note you draw from using its bracketed number, \
like [1], matching the order it was retrieved in. Do not present retrieved content as something \
you already knew. vault_append is the only tool that changes anything on disk — use it only when \
the user has clearly asked you to record or save something, never speculatively. Never claim to \
have taken an action (scheduling an event, sending a message, modifying a calendar) unless a tool \
call actually performed it; you have no calendar or messaging tools, so do not invent having used \
one.

Be concise and warm, but do not flatter or perform enthusiasm the user didn't ask for. Say when \
you don't know something rather than guessing with false confidence.";

/// Per-turn context used to compose the final system prompt.
pub struct PromptContext<'a> {
    /// e.g. "2026-07-09 (Thursday)".
    pub date: &'a str,
    /// Rendered core-memory block (may be empty).
    pub core_memory: &'a str,
    /// Rendered cited context block (may be empty).
    pub retrieved: &'a str,
    pub quiet_hours: bool,
}

/// Compose the full system prompt: base instruction + a "Today is …" line +
/// (if non-empty) a core-memory section + (if non-empty) a retrieved-notes
/// section + a quiet-hours note when true. Sections are clearly delimited so
/// the model can tell instruction from user-derived context.
pub fn build_system_prompt(ctx: &PromptContext<'_>) -> String {
    let mut out = String::new();
    out.push_str(BASE_SYSTEM_PROMPT);

    out.push_str("\n\n---\n\nToday is ");
    out.push_str(ctx.date);
    out.push('.');

    if !ctx.core_memory.trim().is_empty() {
        out.push_str("\n\nWhat you know about the user:\n");
        out.push_str(ctx.core_memory.trim());
    }

    if !ctx.retrieved.trim().is_empty() {
        out.push_str("\n\nRelevant notes (cite as [n]):\n");
        out.push_str(ctx.retrieved.trim());
    }

    if ctx.quiet_hours {
        out.push_str(
            "\n\nIt is currently quiet hours for the user: keep responses brief, avoid \
             suggesting new tasks or notifications, and defer anything non-urgent.",
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_date_always() {
        let ctx = PromptContext {
            date: "2026-07-09 (Thursday)",
            core_memory: "",
            retrieved: "",
            quiet_hours: false,
        };
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("2026-07-09 (Thursday)"));
    }

    #[test]
    fn omits_core_memory_and_retrieved_sections_when_empty() {
        let ctx = PromptContext {
            date: "2026-07-09 (Thursday)",
            core_memory: "",
            retrieved: "   ",
            quiet_hours: false,
        };
        let prompt = build_system_prompt(&ctx);
        assert!(!prompt.contains("What you know about the user"));
        assert!(!prompt.contains("Relevant notes"));
        assert!(!prompt.contains("quiet hours"));
    }

    #[test]
    fn includes_core_memory_and_retrieved_sections_when_present() {
        let ctx = PromptContext {
            date: "2026-07-09 (Thursday)",
            core_memory: "Name: Aahaan. Studies Mechanical Engineering.",
            retrieved: "[1] Knowledge/thermo.md\nEntropy is a measure of disorder.",
            quiet_hours: false,
        };
        let prompt = build_system_prompt(&ctx);
        assert!(prompt.contains("What you know about the user:"));
        assert!(prompt.contains("Name: Aahaan. Studies Mechanical Engineering."));
        assert!(prompt.contains("Relevant notes (cite as [n]):"));
        assert!(prompt.contains("[1] Knowledge/thermo.md"));
    }

    #[test]
    fn quiet_hours_note_appears_only_when_true() {
        let ctx_true = PromptContext {
            date: "2026-07-09 (Thursday)",
            core_memory: "",
            retrieved: "",
            quiet_hours: true,
        };
        assert!(build_system_prompt(&ctx_true).contains("quiet hours"));

        let ctx_false = PromptContext {
            date: "2026-07-09 (Thursday)",
            core_memory: "",
            retrieved: "",
            quiet_hours: false,
        };
        assert!(!build_system_prompt(&ctx_false).contains("quiet hours"));
    }
}
