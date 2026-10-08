# How to Navigate This Vault

> **[AUTO-GENERATED FILE - DO NOT EDIT]**
> This vault is actively managed by an autonomous AI Assistant. It is organized as an interconnected **Knowledge Graph** rather than a strict top-down folder hierarchy. To prevent breaking the AI's internal graph, please adhere to the following structure when manually creating or editing files.

## 1. How to Traverse (For Humans)
If you are not using AI tools to navigate, here is the easiest way to traverse the vault:
- **Start at the MOCs:** Begin in the `/MOCs` directory or the root `Index.md` (if it exists). These act as dashboards grouping links to related concepts.
- **Follow the `up:` Links (Bottom-Up):** If you land on a deeply nested note inside `/Concepts`, look at the `up:` link in the YAML frontmatter to zoom out to its parent category. No note is an orphan; you can always find your way back up.
- **Lateral Links & Backlinks:** Use standard `[[WikiLinks]]` in the body text to explore laterally. Use your markdown editor's **Backlinks pane** to see every atomic concept that mentions the note you are currently viewing.

## 2. Core Ontological Zones (For Manual Edits)
If you add files manually, pure knowledge must be strictly organized into these root directories:
- `/MOCs`: Maps of Content (navigational hubs)
- `/Concepts`: Pure atomic knowledge (one indivisible idea per file, usually under 200 words)
- `/Entities`: Real-world instantiations (people, codebase mappings, projects)
- `/Sources`: Raw unprocessed data, articles, or web clippings
- `/Logs`: Chronological append-only entries (daily journals, meetings)

*Note: You may create any other folders you need for active operational work (e.g., `/Test Cases`, `/Drafts`).*

## 3. The Mandatory YAML Link
Every single markdown file in this vault **MUST** include YAML frontmatter linking it back to a parent concept in the Knowledge Graph. 

Example:
```yaml
---
up: "[[Main Topic]]"
---
```
