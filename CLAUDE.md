## Project Overview

AIPP (AI 助手平台) is a cross-platform desktop application built with Tauri 2.0 that serves as a comprehensive AI assistant platform. The application enables users to interact with multiple large language models, execute scripts, preview components, manage conversations, and extend functionality through MCP (Model Context Protocol).

**Core Technologies:**

-   **Backend**: Rust with Tauri 2.0 framework, SQLite via rusqlite
-   **Frontend**: React 19 with TypeScript, Vite build system
-   **UI Framework**: shadcn/ui components, Radix UI primitives, Tailwind CSS v4
-   **AI Integration**: Custom forked genai client with streaming support
-   **MCP Protocol**: rmcp crate for Model Context Protocol integration
-   **State Management**: React hooks for frontend, Arc<TokioMutex<>> for Rust backend
-   **Content Execution**: Support for HTML, SVG, React, Vue, Python, Bash/PowerShell, AppleScript
-   **Platform Features**: System tray, global shortcuts (Ctrl+Shift+I/O), multi-window architecture
-   **Testing**: Comprehensive test suite with integration tests for AI functionality

## Essential Build Commands

```bash
# Verify frontend changes (includes TypeScript check)
npm run build

# Verify Rust backend changes
cargo build --manifest-path src-tauri/Cargo.toml

# Build complete application
npm run package

# Development mode (not recommended for debugging)
npm run dev

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml
```

**Important**: Frontend debugging should be done through the built application, not through `npm run dev`.

## Architecture Overview

### Window-Based Architecture

The application uses multiple Tauri windows for different features:

-   **Ask Window**: Quick AI query interface
-   **Config Window**: Settings and configuration
-   **ChatUI Window**: Main chat interface
-   **ArtifactPreview Window**: Content preview (HTML, SVG, components)
-   **ArtifactCollections Window**: Manage artifact collections
-   **Plugin Windows**: For plugin management and store

### Frontend Structure

```
src/
├── components/
│   ├── ui/          # shadcn/ui primitives
│   ├── common/      # Shared components (ConfigPageLayout, EmptyState, etc.)
│   ├── config/      # Configuration-related components
│   │   ├── assistant/     # Assistant form rendering
│   │   └── feature/       # Feature-specific forms
│   ├── conversation/      # Chat conversation components
│   ├── message-item/      # Message display components
│   └── magicui/     # Animation components
├── hooks/           # Custom React hooks
│   ├── assistant/   # Assistant management hooks
│   └── feature/     # Feature configuration hooks
├── data/            # TypeScript types and data models
├── lib/             # Utility functions
├── windows/         # Window-specific entry points
└── artifacts/       # React/Vue artifact templates
```

Key patterns:

-   Use `@/` import alias for `./src/`
-   Component-specific CSS modules alongside Tailwind
-   React Hook Form with Zod for form validation
-   Domain-specific hook organization (assistant/, feature/)

### Backend Structure

```
src-tauri/
├── src/
│   ├── api/         # Tauri command handlers
│   │   ├── ai/      # AI functionality (modularized)
│   │   │   ├── chat.rs          # Stream/non-stream chat handling
│   │   │   ├── config.rs        # Model configuration management
│   │   │   ├── conversation.rs  # Message processing utilities
│   │   │   ├── events.rs        # Event definitions
│   │   │   ├── mcp.rs          # MCP integration
│   │   │   ├── title.rs        # Title generation
│   │   │   └── types.rs        # AI request/response types
│   │   ├── builtin_mcp/         # Built-in MCP tools
│   │   │   ├── search/          # Web search functionality
│   │   │   │   ├── engines/     # Search engine implementations
│   │   │   │   ├── fetcher.rs   # Content fetching with fallback strategies
│   │   │   │   ├── browser.rs   # Browser management
│   │   │   │   └── handler.rs   # Main search handler
│   │   │   └── templates.rs     # MCP template management
│   │   ├── tests/               # Comprehensive test suite
│   │   └── [other apis]...
│   ├── artifacts/   # Content rendering (HTML, React, Vue, AppleScript, PowerShell)
│   ├── db/          # Database operations (SQLite)
│   ├── errors.rs    # Error handling
│   ├── state/       # Application state management
│   ├── template_engine/  # Prompt templating with bang commands
│   ├── utils/       # Helper functions (bun_utils, uv_utils, etc.)
│   └── window.rs    # Window management
```

**Key API modules:**

-   `ai_api.rs`: Main AI interaction entry points (ask_ai, regenerate_ai)
-   `ai/chat.rs`: Streaming and non-streaming chat implementation
-   `ai/mcp.rs`: Model Context Protocol integration and tool detection
-   `ai/config.rs`: Configuration merging and chat options building
-   `assistant_api.rs`: Assistant management
-   `conversation_api.rs`: Chat conversations with versioning support
-   `mcp_api.rs`: MCP server management
-   `builtin_mcp_api.rs`: Built-in tools (web search, URL fetching)
-   `artifacts_api.rs` & `artifacts_collection_api.rs`: Artifact management

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

**Frontend State Management:**

```typescript
// Use domain-specific custom hooks
const { deleteConversation, listConversations } = useConversationManager();
const { models, updateModel } = useModels();
const { assistant, saveAssistant } = useAssistantRuntime();

// New hook patterns for feature management
const { formConfig } = useAssistantFormConfig(assistantType);
const { featureConfig, updateConfig } = useFeatureConfig();

// Hook naming convention: use[Domain][Action/Manager]
// Examples: useConversationEvents, useMessageProcessing, useFileManagement
```

**Backend State Management:**

```rust
// Thread-safe state with Arc<TokioMutex<T>>
struct FeatureConfigState {
    configs: Arc<TokioMutex<Vec<FeatureConfig>>>,
    config_feature_map: Arc<TokioMutex<HashMap<String, HashMap<String, FeatureConfig>>>>,
}

// Always use async-aware locks
let config = state.configs.lock().await;
```

### Component Patterns

-   Prefer shadcn/ui components from `@/components/ui`
-   Use Radix UI primitives for complex interactions
-   Follow existing component structure and naming conventions
-   Keep complex logic in Rust, UI logic in React
-   Use domain-specific component organization (config/, conversation/, etc.)

## Critical Features to Maintain

1. **Multi-Model Support**: Integration with various LLM providers through genai client
2. **Local Data Storage**: All user data stored locally via SQLite, no cloud sync
3. **Bang Commands**: Input starting with `!` for quick actions via template engine
4. **Content Preview**: Rendering HTML, SVG, React/Vue components in ArtifactPreview window
5. **Script Execution**: Running AI-generated code in configured environments
6. **System Tray**: Global shortcuts (Ctrl+Shift+I/O)
7. **MCP Integration**: Model Context Protocol for extensible tool calling
8. **Message Versioning**: Support for regenerating responses with parent/child relationships
9. **Built-in Tools**: Web search with multiple engines, URL fetching, fingerprint management
10. **Artifact Collections**: Managing and organizing generated artifacts
11. **Assistant Types**: Different assistant configurations with custom forms

## Development Guidelines

1. **Cross-Platform**: Ensure compatibility across Windows, macOS, and Linux
2. **Performance**: Optimize resource loading, use caching, minimize re-renders
3. **Async Operations**: No blocking operations in Rust, use Tokio runtime
4. **Type Safety**: Maintain TypeScript strict mode and Rust type safety
5. **Error Handling**: Provide meaningful error messages to users
6. **Testing**: Write tests for new functionality, especially AI-related features
7. **Code Organization**: Follow domain-driven structure for both frontend and backend

## Testing Changes

Always verify both frontend and backend changes:

```bash
# Check TypeScript
npm run build

# Check Rust (includes clippy lints)
cargo check --manifest-path src-tauri/Cargo.toml

# Run Rust tests，When running Rust tests, please run them with precise, minimal scope—for example, by method or by file.
cargo test --manifest-path src-tauri/Cargo.toml
```

## Common Development Tasks

### Adding a New API Endpoint

1. Create Tauri command in `src-tauri/src/api/[module].rs`
2. Export command in `src-tauri/src/api/mod.rs`
3. Register in `src-tauri/src/main.rs` builder
4. Create TypeScript types in `src/data/`
5. Call from frontend using `invoke()`
6. Add tests in `src-tauri/src/api/tests/`

### Working with AI Features

-   Core AI logic is in `ai_api.rs` with modular implementations in `ai/` subdirectory
-   Stream processing uses genai client with event emission for real-time UI updates
-   MCP tools are automatically detected and can be called natively or through prompt formatting
-   All AI responses support versioning through `generation_group_id` and `parent_group_id`
-   Built-in tools available through `builtin_mcp/` module

### Adding a New UI Component

1. Check if shadcn/ui has the component
2. Follow existing component patterns in `src/components/`
3. Use domain-specific directories (config/, conversation/, etc.)
4. Use Tailwind classes for styling
5. Add component-specific styles in CSS modules if needed
6. 编写界面的时候，注意样式风格要和现在的界面一致，使用 ShadcnUI 的组件和 tailwind css 的写法，我的主色调是黑白灰，尽量少使用别的颜色

### Adding New Assistant Types

1. Define assistant type in `src/data/Assistant.tsx`
2. Create form configuration in `src/hooks/assistant/useAssistantFormConfig.ts`
3. Add form renderer in `src/components/config/assistant/AssistantFormRenderer.tsx`
4. Handle backend logic in `assistant_api.rs`

### Database Schema Changes

1. Update schema in `src-tauri/src/db/[entity].rs`
2. Handle migrations in `src-tauri/src/db/mod.rs`
3. Update corresponding TypeScript types
4. Key tables: conversations, messages (with versioning), assistants, mcp_servers, llm_models, artifacts

### MCP Integration Guidelines

-   MCP servers are managed through `mcp_api.rs` and stored in SQLite
-   Tool detection happens automatically via `ai/mcp.rs::detect_and_process_mcp_calls`
-   Native tool calls are preferred when `use_native_toolcall` is true
-   Prompt formatting fallback when native calls are disabled
-   Built-in MCP tools available: web search (Google, Bing, DuckDuckGo, Kagi), URL fetching

### Built-in MCP Tools

The application includes built-in MCP tools in `builtin_mcp/`:

-   **Web Search**: Multi-engine search with intelligent fallback (Google → Bing)
-   **Content Fetching**: Playwright-based with headless browser and HTTP fallbacks
-   **Fingerprint Management**: Anti-detection for web scraping
-   **Template Management**: Dynamic MCP server configuration

### Artifact Management

-   Artifacts support HTML, SVG, React, Vue components
-   Collections for organizing related artifacts
-   Preview windows with live rendering
-   Script execution environments (Python, Node.js, etc.)

## Testing Framework

-   Integration tests in `src-tauri/src/api/tests/`
-   AI functionality tests with mocked responses
-   Conversation management tests
-   Regeneration and versioning tests

# important-instruction-reminders

Do what has been asked; nothing more, nothing less.
NEVER create files unless they're absolutely necessary for achieving your goal.
ALWAYS prefer editing an existing file to creating a new one.
NEVER proactively create documentation files (\*.md) or README files. Only create documentation files if explicitly requested by the User.

IMPORTANT: this context may or may not be relevant to your tasks. You should not respond to this context unless it is highly relevant to your task.
