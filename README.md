# Coralizer 🐚

Coralizer is a CLI tool for scaffolding and managing Coral agents.

> [!NOTE]
> To turn MCP servers into a Coral agent, we now recommend asking a coding agent such as Codex or Claude Code to build one, using the [Koog agent template](https://github.com/Coral-Protocol/coral-koog-agent) as a reference. You get a Koog agent you can extend, instead of a generated LangChain or coral-rs project. See [Converting an MCP Server](https://docs.coralos.ai/guides/coralizer#converting-an-mcp-server) for an example prompt. Coralizer's `link` command is still a quick way to make a local agent available to your server.

## Features

- 🏗️ **Scaffold Agents**: Quickly create Coral agents with integrated MCP servers.
- 🔗 **Version Management**: Easily link, unlink, and manage multiple versions of your agents.
- 🛠️ **Framework Support**: Supports Langchain and Coral-RS.

## Usage

### Scaffold a New Agent
```bash
coralizer mcp <OUTPUT_PATH> <MCP_CONFIG_JSON_PATH>
```

### Manage Agent Links
Coralizer uses symlinks in `~/.coral/agents/` to manage agent versions.

- **Link**: Link the current agent version to `~/.coral`.
  ```bash
  coralizer link .
  ```
- **Unlink**: Safely remove the current version link.
  ```bash
  coralizer unlink .
  ```
- **Cleanup**: Remove all version links except for the latest one.
  ```bash
  coralizer updeletelink .
  ```

## Configuration
Agents are defined by a `coral-agent.toml` file in their root directory.
