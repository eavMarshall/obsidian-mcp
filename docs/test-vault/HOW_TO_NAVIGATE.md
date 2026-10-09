# How to Navigate This Vault

> **[AUTO-GENERATED FILE - DO NOT EDIT]**
> This vault is actively managed by an autonomous AI Assistant. It is organized as an interconnected **Knowledge Graph** rather than a strict top-down folder hierarchy. To prevent breaking the AI's internal graph, please adhere to the following structure when manually creating or editing files.

## 1. How to Traverse (For Humans)
If you are not using AI tools to navigate, here is the easiest way to traverse the vault:
- **Start at the MOCs:** Begin in the `/MOCs` directory or the root `_index.md` (if it exists). These act as dashboards grouping links to related concepts.
- **Follow the Parent Links (Bottom-Up):** If you land on a deeply nested note, look for a `Part of [[...]]` line in the body or the folder's `_index.md` to zoom out to its parent category. No note is an orphan; you can always find your way back up.
- **Lateral Links & Backlinks:** Use standard `[[WikiLinks]]` in the body text to explore laterally. Use your markdown editor's **Backlinks pane** to see every atomic concept that mentions the note you are currently viewing.

## 2. Folder Structure
Notes are topics that hold many cases. Use folders freely to organize them.

## 3. The Mandatory Parent Link
Every single markdown file in this vault **MUST** link back to a parent concept in the Knowledge Graph. This is taken from the folder's `_index.md`, or from the body's `Part of [[...]]` line. `up:` in frontmatter is forbidden.

Expected frontmatter keys are `title`, `tags`, and `summary`.

Example:
```yaml
---
title: "Topic"
tags: []
summary: ""
---
```
Part of [[Main Topic]]
