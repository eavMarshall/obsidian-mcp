# AI Vault Migration Guide

> **TARGET AUDIENCE:** AI Agents and LLMs.
> **OBJECTIVE:** Convert an unstructured, legacy Obsidian vault into a strict, token-optimized **Bottom-Up Knowledge Graph** compatible with the `obsidian-mcp` gateway.

## The Paradigm Shift
This vault is managed by a strict MCP gateway that enforces referential integrity. You cannot arbitrarily create disconnected notes. Every note must be part of a continuous, traversable graph. 

## Folder Structure
You have the freedom to structure your folders as you see fit.

## Migration Steps

Follow these sequential steps when migrating an existing vault:

### Step 1: Establish the Root Node
The graph must have a starting point. Ensure there is an `_index.md` in the root.
- The file should contain `Part of [[_index]]` in its body to satisfy the parser.

### Step 2: Enforce the Parent Constraint (Bottom-Up Linking)
The gateway strictly enforces that **every markdown file** must have a parent. This is taken from the folder's `_index.md`, or from the body's `Part of [[...]]` line. `up:` in frontmatter is forbidden. Expected frontmatter keys are `title`, `tags`, and `summary`.
- **Rule:** No orphans allowed. 
- **Migration Action:** Traverse the vault. Add `Part of [[Parent MOC or Concept]]` to the body of files without a parent.

### Step 3: Resolve Foreign Key Violations
The `obsidian-mcp` server operates like a strict relational database. 
- If you try to write a file with `Part of [[AI Concepts]]`, but `AI Concepts.md` does not exist in the vault, the server will reject your write with a **Foreign Key Constraint** error.
- **Migration Action:** You must build the graph **bottom-up** or **top-down**. If you are migrating a batch of files that all belong to "AI Concepts", you must *create* the `AI Concepts.md` MOC file *first*, before you can update the child notes to point to it.

### Step 4: No Word Limit
- **Migration Action:** A note is a topic holding many cases. Do not split notes into atomic parts or enforce word limits.



## Troubleshooting

- **Error:** `ARCHITECTURAL VIOLATION: Foreign Key Constraint failed.`
  - *Fix:* You attempted to add a `[[Link]]` to a file that does not exist. Create the target file first.
- **Error:** `ARCHITECTURAL VIOLATION: Parent _index.md not found...`
  - *Fix:* Every file must point UP the hierarchy using `Part of [[...]]` or have a valid `_index.md` in its folder.
- **Error:** `cannot delete the auto-generated HOW_TO_NAVIGATE.md`
  - *Fix:* Leave system files alone. Do not modify or delete `HOW_TO_NAVIGATE.md`.
