/// AI Prompts for Plan Review System
///
/// This module contains all prompts used in the multiplayer ChatGPT/plans system.
/// These prompts wrap user interactions and provide context to the AI model.
/// System prompt for plan review context
#[allow(dead_code)]
pub const SYSTEM_CONTEXT: &str = r#"You are an AI assistant helping with a collaborative engineering plan review system.

Your role is to:
1. Analyze markdown-formatted engineering plans
2. Review user comments on specific sections
3. Suggest improvements based on the feedback
4. Maintain the original structure and formatting
5. Be concise and focused on the specific issue raised

Guidelines:
- Only modify the lines that need changes based on the comment
- Preserve markdown formatting (headers, lists, code blocks, etc.)
- Keep the same level of technical detail
- Be precise and actionable in your suggestions
- If the comment is unclear, make your best interpretation"#;

/// Generate the prompt for plan changes based on a user comment
///
/// # Arguments
/// * `relevant_lines` - The specific lines from the plan that the comment refers to
/// * `line_start` - Starting line number
/// * `line_end` - Ending line number
/// * `comment_text` - The user's comment/feedback
pub fn generate_plan_review_prompt(
    relevant_lines: &str,
    line_start: i32,
    line_end: i32,
    comment_text: &str,
) -> String {
    format!(
        r#"Here is a section from an engineering plan document (lines {}-{}):

```markdown
{}
```

User's comment: "{}"

Please provide a revised version of these lines that addresses the user's comment.

Important:
- Only output the revised markdown content
- Do not include explanations, preambles, or additional commentary
- Maintain the same structure and formatting as the original
- Focus on addressing the specific issue raised in the comment
- Keep changes minimal and targeted"#,
        line_start, line_end, relevant_lines, comment_text
    )
}

/// Alternate prompt for more complex plan revisions (reserved for future use)
#[allow(dead_code)]
pub fn generate_detailed_review_prompt(
    full_plan: &str,
    relevant_lines: &str,
    line_start: i32,
    line_end: i32,
    comment_text: &str,
    _plan_title: &str,
) -> String {
    format!(
        r#"You are reviewing an engineering plan. Here is the full context:

<full_plan>
{}
</full_plan>

The user has commented on lines {}-{}:

<commented_section>
{}
</commented_section>

User's comment: "{}"

Please provide a revised version of ONLY the commented section (lines {}-{}) that:
1. Addresses the user's feedback
2. Maintains consistency with the rest of the plan
3. Preserves all markdown formatting
4. Keeps the same level of technical depth

Output only the revised markdown for lines {}-{}, with no additional explanation."#,
        full_plan,
        line_start,
        line_end,
        relevant_lines,
        comment_text,
        line_start,
        line_end,
        line_start,
        line_end
    )
}

/// Prompt for generating change descriptions (for version history)
#[allow(dead_code)]
pub fn generate_change_description_prompt(original: &str, revised: &str, comment: &str) -> String {
    format!(
        r#"Summarize the changes between these two versions in one concise sentence (max 100 characters):

Original:
{}

Revised:
{}

Context: This change was made in response to the comment: "{}"

Provide only the summary sentence, no additional text."#,
        original, revised, comment
    )
}
