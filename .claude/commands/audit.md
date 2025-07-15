
# /audit Custom Command Documentation

## Overview

The `/audit` command is a custom tool designed for use in Claude Code (based on Anthropic's Claude AI coding assistant). It enables developers to perform automated audits on selected code snippets or entire files, focusing on three key areas:

- **Security Issues**: Identification of potential vulnerabilities, unsafe practices, and security risks.
- **Optimizations**: Suggestions for improving code efficiency, performance, and resource usage.
- **Incomplete Logic**: Detection of missing edge cases, unfinished implementations, or logical gaps.

This command leverages the `mcp_code-audit_audit_code` tool to conduct thorough analyses and generates reports that can be logged directly into the project's knowledge base (kb/) for tracking and resolution.

## Purpose

The primary goal of `/audit` is to enhance code quality by providing actionable insights during development. It helps maintain high standards in the Script language project by:
- Ensuring secure coding practices in async operations, closures, and FFI interactions.
- Optimizing runtime performance, especially in areas like async transformations and garbage collection.
- Verifying complete and robust logic in parsers, semantic analyzers, and code generators.

By integrating with the knowledge base, it facilitates team collaboration on issue resolution and maintains a historical record of audits.

## Usage

1. **Invocation**:
   - In the Cursor editor (or compatible IDE with Claude integration), select the code you want to audit.
   - Type `/audit` in the chat interface to invoke the command.
   - Optionally, specify parameters like audit type (e.g., "security", "performance", "all").

2. **Parameters**:
   - **code**: The selected code snippet (automatically provided).
   - **language**: Automatically detected, but can be specified (e.g., "rust" for Script's backend).
   - **auditType**: Optional; defaults to "all". Options: "security", "performance", "quality", "completeness", etc.
   - **includeFixSuggestions**: Boolean; defaults to true for solution proposals.

3. **Process**:
   - Claude will call the `mcp_code-audit_audit_code` tool with the provided code and parameters.
   - The tool performs a comprehensive audit using AI models.
   - Results are analyzed, and if issues are found, they are formatted and logged to the knowledge base.

4. **Output**:
   - A summary of findings in the chat.
   - If issues are detected, a new Markdown file is created in `kb/active/` using the `mcp_kb_update` tool.
   - Notification of the new KB entry for tracking.

## Integration with Tools

- **Audit Tool**: Uses `mcp_code-audit_audit_code` for the core auditing logic. This tool supports various audit types and provides detailed reports with fix suggestions.
- **Knowledge Base Integration**: Issues are stored in `kb/active/[ISSUE_NAME].md` with a structured format for easy reference.
- **Error Handling**: If the audit tool fails or access is denied, fallback to manual review prompts.

## Example Workflow

1. Select code in `src/runtime/async_ffi.rs`.
2. Invoke `/audit`.
3. Claude runs the audit and finds a security issue.
4. A new file `kb/active/ASYNC_FFI_SECURITY_ISSUE.md` is created with details.

## Issue Format in Knowledge Base

Each issue file in `kb/active/` follows this structure:

```
# [Issue Title]

## File Path
[path/to/file.rs]

## Issue Description
[Detailed description of the issue, including type (security/optimization/incomplete logic)]

## Severity
[Low/Medium/High/Critical]

## Solutions
- [Solution 1]
- [Solution 2]
- ...

## Additional Notes
[Any extra context or references]
```

## Best Practices

- Run `/audit` frequently during development, especially after major changes.
- Use specific audit types for focused reviews (e.g., "security" for async code).
- Review and verify AI-generated suggestions before implementation.
- Update the KB entry to "completed/" once resolved.

## Limitations

- Dependent on the accuracy of the underlying AI audit models.
- May not catch all issues; combine with manual code reviews.
- Requires proper configuration of MCP tools and access permissions.

For more details on Claude Code, refer to: https://docs.anthropic.com/en/docs/claude-code/overview

This documentation ensures the `/audit` command is used effectively to maintain high-quality code in the Script project.

