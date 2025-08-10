# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIPP is a Tauri 2.0 desktop application that serves as an AI assistant platform with rich features for interacting with various language models. The application supports multiple LLM providers, local data storage, content preview, script execution, and extensible tooling through MCP (Model Context Protocol).

**Core Technologies:**
- **Backend**: Rust with Tauri 2.0 framework
- **Frontend**: React 19 with TypeScript, built with Vite  
- **UI**: shadcn/ui components, Radix UI primitives, Tailwind CSS
- **Database**: SQLite via rusqlite
- **AI Integration**: Custom genai client with streaming support
- **MCP Support**: rmcp crate for Model Context Protocol
- **State Management**: React hooks for frontend, Arc<TokioMutex<>> for Rust backend

## Essential Build Commands

```bash
# Verify frontend changes (includes TypeScript check)
npm run build

# Verify Rust backend changes
cargo check --manifest-path src-tauri/Cargo.toml

# Build complete application
npm run package

# Development mode (not recommended for debugging)
npm run dev
```

**Important**: Frontend debugging should be done through the built application, not through `npm run dev`.

## Architecture Overview

### Window-Based Architecture

The application uses multiple Tauri windows for different features:
- **Ask Window**: Quick AI query interface
- **Config Window**: Settings and configuration
- **ChatUI Window**: Main chat interface
- **ArtifactPreview Window**: Content preview (HTML, SVG, components)
- **Plugin Windows**: For future plugin system

### Frontend Structure

```
src/
├── components/
│   ├── ui/          # shadcn/ui primitives
│   ├── common/      # Shared components
│   └── [feature]/   # Feature-specific components
├── hooks/           # Custom React hooks (useConversationManager, etc.)
├── data/            # TypeScript types and data models
├── lib/             # Utility functions
└── windows/         # Window-specific entry points
```

Key patterns:
- Use `@/` import alias for `./src/`
- Component-specific CSS modules alongside Tailwind
- React Hook Form with Zod for form validation

### Backend Structure

```
src-tauri/
├── src/
│   ├── api/         # Tauri command handlers
│   │   ├── ai/      # Refactored AI functionality
│   │   │   ├── chat.rs          # Stream/non-stream chat handling
│   │   │   ├── config.rs        # Model configuration management
│   │   │   ├── conversation.rs  # Message processing utilities
│   │   │   ├── events.rs        # Event definitions
│   │   │   ├── mcp.rs          # MCP integration
│   │   │   ├── title.rs        # Title generation
│   │   │   └── types.rs        # AI request/response types
│   │   └── [other apis]...
│   ├── artifacts/   # Content rendering (HTML, React, Vue)
│   ├── db/          # Database operations
│   ├── errors.rs    # Error handling
│   ├── state/       # Application state management
│   ├── template_engine/ # Prompt templating
│   └── utils/       # Helper functions
```

**Key API modules:**
- `ai_api.rs`: Main AI interaction entry points (ask_ai, regenerate_ai)
- `ai/chat.rs`: Streaming and non-streaming chat implementation
- `ai/mcp.rs`: Model Context Protocol integration and tool detection
- `ai/config.rs`: Configuration merging and chat options building
- `assistant_api.rs`: Assistant management
- `conversation_api.rs`: Chat conversations
- `mcp_api.rs`: MCP server management

## Key Development Patterns

### Frontend-Backend Communication

```rust
// Backend: Define Tauri command
#[tauri::command]
async fn get_conversation(id: String) -> Result<Conversation, String> {
    // Implementation
}

// Frontend: Call command
import { invoke } from '@tauri-apps/api/core';
const conversation = await invoke('get_conversation', { id: conversationId });
```

### State Management

Frontend uses custom hooks for state:
```typescript
const { conversations, createConversation } = useConversationManager();
```

Backend uses thread-safe state:
```rust
let app_state = Arc::new(TokioMutex::new(AppState::default()));
```

### Component Patterns

- Prefer shadcn/ui components from `@/components/ui`
- Use Radix UI primitives for complex interactions
- Follow existing component structure and naming conventions
- Keep complex logic in Rust, UI logic in React

## Critical Features to Maintain

1. **Multi-Model Support**: Integration with various LLM providers through genai client
2. **Local Data Storage**: All user data stored locally via SQLite, no cloud sync
3. **Bang Commands**: Input starting with `!` for quick actions
4. **Content Preview**: Rendering HTML, SVG, React/Vue components in ArtifactPreview window
5. **Script Execution**: Running AI-generated code in configured environments
6. **System Tray**: Global shortcuts (Ctrl+Shift+I/O)
7. **MCP Integration**: Model Context Protocol for extensible tool calling
8. **Message Versioning**: Support for regenerating responses with parent/child relationships

## Development Guidelines

1. **Cross-Platform**: Ensure compatibility across Windows, macOS, and Linux
2. **Performance**: Optimize resource loading, use caching, minimize re-renders
3. **Async Operations**: No blocking operations in Rust, use Tokio runtime
4. **Type Safety**: Maintain TypeScript strict mode and Rust type safety
5. **Error Handling**: Provide meaningful error messages to users

## Testing Changes

Always verify both frontend and backend changes:
```bash
# Check TypeScript
npm run build

# Check Rust
cargo check --manifest-path src-tauri/Cargo.toml
```

## Common Development Tasks

### Adding a New API Endpoint
1. Create Tauri command in `src-tauri/src/api/[module].rs`
2. Export command in `src-tauri/src/api/mod.rs`
3. Register in `src-tauri/src/main.rs` builder
4. Create TypeScript types in `src/data/`
5. Call from frontend using `invoke()`

### Working with AI Features
- Core AI logic is in `ai_api.rs` with modular implementations in `ai/` subdirectory
- Stream processing uses genai client with event emission for real-time UI updates
- MCP tools are automatically detected and can be called natively or through prompt formatting
- All AI responses support versioning through `generation_group_id` and `parent_group_id`

### Adding a New UI Component
1. Check if shadcn/ui has the component
2. Follow existing component patterns in `src/components/`
3. Use Tailwind classes for styling
4. Add component-specific styles in CSS modules if needed
5. 编写界面的时候，注意样式风格要和现在的界面一致，使用ShadcnUI的组件和tailwind css的写法，我的主色调是黑白灰，尽量少使用别的颜色

### Database Schema Changes
1. Update schema in `src-tauri/src/db/[entity].rs`
2. Handle migrations in `src-tauri/src/db/mod.rs`  
3. Update corresponding TypeScript types
4. Key tables: conversations, messages (with versioning), assistants, mcp_servers, llm_models

### MCP Integration Guidelines
- MCP servers are managed through `mcp_api.rs` and stored in SQLite
- Tool detection happens automatically via `ai/mcp.rs::detect_and_process_mcp_calls`
- Native tool calls are preferred when `use_native_toolcall` is true
- Prompt formatting fallback when native calls are disabled