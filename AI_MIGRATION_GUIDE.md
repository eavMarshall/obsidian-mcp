# AI Vault Migration Guide

> **TARGET AUDIENCE:** AI Agents and LLMs.
> **OBJECTIVE:** Convert an unstructured, legacy Obsidian vault into a strict, token-optimized **Bottom-Up Knowledge Graph** compatible with the `obsidian-mcp` gateway.

## The Paradigm Shift
This vault is managed by a strict MCP gateway that enforces referential integrity. You cannot arbitrarily create disconnected notes. Every note must be part of a continuous, traversable graph. 

## The Core Ontological Zones
Before migrating files, you must establish the core directory structure for pure knowledge. You have the freedom to keep active operational folders (like `/Projects`), but all knowledge must be refactored into:

- `/MOCs` (Maps of Content): Navigational hubs and domain indices.
- `/Concepts`: Pure, atomic knowledge (theories, logic, abstract ideas).
- `/Entities`: Real-world instantiations (people, tools, specific codebases, companies).
- `/Sources`: Raw unprocessed data, articles, book highlights, or web clippings.
- `/Logs`: Chronological append-only entries (daily journals, meeting notes).

## Migration Steps

Follow these sequential steps when migrating an existing vault:

### Step 1: Establish the Root Node
The graph must have a starting point. Ensure there is an `Index.md` in the root (or inside `/MOCs`). 
- Its YAML frontmatter should ideally link to itself (`up: "[[Index]]"`) to satisfy the parser.

### Step 2: Enforce the `up:` Constraint (Bottom-Up Linking)
The gateway strictly enforces that **every markdown file** must contain valid YAML frontmatter with an `up:` field linking to a parent concept.
```yaml
---
up: "[[Parent MOC or Concept]]"
---
```
- **Rule:** No orphans allowed. 
- **Migration Action:** Traverse the vault. If a file lacks this frontmatter, inject it. If you don't know the exact parent, link it up to a broad category MOC (e.g., `up: "[[Software Engineering MOC]]"`).

### Step 3: Resolve Foreign Key Violations
The `obsidian-mcp` server operates like a strict relational database. 
- If you try to write a file with `up: "[[AI Concepts]]"`, but `AI Concepts.md` does not exist in the vault, the server will reject your write with a **Foreign Key Constraint** error.
- **Migration Action:** You must build the graph **bottom-up** or **top-down**. If you are migrating a batch of files that all belong to "AI Concepts", you must *create* the `AI Concepts.md` MOC file *first*, before you can update the child notes to point to it.

### Step 4: Enforce Atomicity (Refactoring Monoliths)
Large files destroy LLM token context limits.
- **Migration Action:** If you encounter a file exceeding 500 words covering multiple disparate ideas, split it.
- Extract the sub-topics into new files inside `/Concepts`.
- Leave a summary in the original file and link out to the new atomic files.
- Ensure the newly created files point their `up:` link back to the original file.

### Step 5: Categorize and Relocate
Move existing files into the appropriate Core Zones. 
- Use the `rename_note` tool to move a file (e.g., `old_path: "Random Idea.md"`, `new_path: "Concepts/Random Idea.md"`). 
- **Note:** The `rename_note` tool automatically updates all backlinks across the entire vault to point to the new path, so you do not need to manually fix broken links.

## Troubleshooting

- **Error:** `ARCHITECTURAL VIOLATION: Foreign Key Constraint failed.`
  - *Fix:* You attempted to add a `[[Link]]` to a file that does not exist. Create the target file first.
- **Error:** `ARCHITECTURAL VIOLATION: The up: field ... cannot be empty.`
  - *Fix:* Every file must point UP the hierarchy.
- **Error:** `cannot delete the auto-generated HOW_TO_NAVIGATE.md`
  - *Fix:* Leave system files alone. Do not modify or delete `HOW_TO_NAVIGATE.md`.
