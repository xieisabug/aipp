# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AIPP (Tea) is a Tauri 2.0 desktop application that serves as an AI assistant platform with rich features for interacting with various language models.

**Core Technologies:**
- **Backend**: Rust with Tauri 2.0 framework
- **Frontend**: React 18 with TypeScript, built with Vite
- **UI**: shadcn/ui components, Radix UI primitives, Tailwind CSS
- **Database**: SQLite via rusqlite
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
│   ├── db/          # Database operations
│   ├── service/     # Business logic
│   └── utils/       # Helper functions
```

API modules follow domain organization:
- `ai_api.rs`: LLM provider integrations
- `assistant_api.rs`: Assistant management
- `conversation_api.rs`: Chat conversations
- `file_api.rs`: File operations

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

1. **Multi-Model Support**: Integration with various LLM providers through standard APIs
2. **Local Data Storage**: All user data stored locally, no cloud sync
3. **Bang Commands**: Input starting with `!` for quick actions
4. **Content Preview**: Rendering HTML, SVG, React/Vue components
5. **Script Execution**: Running AI-generated code in configured environments
6. **System Tray**: Global shortcuts (Ctrl+Shift+I/O)

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
3. Register in `src-tauri/src/lib.rs` builder
4. Create TypeScript types in `src/data/`
5. Call from frontend using `invoke()`

### Adding a New UI Component
1. Check if shadcn/ui has the component
2. Follow existing component patterns in `src/components/`
3. Use Tailwind classes for styling
4. Add component-specific styles in CSS modules if needed

### Database Schema Changes
1. Update schema in `src-tauri/src/db/[entity].rs`
2. Handle migrations in `src-tauri/src/db/mod.rs`
3. Update corresponding TypeScript types