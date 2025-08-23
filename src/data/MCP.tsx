export interface MCPServer {
    id: number;
    name: string;
    description: string | null;
    transport_type: string; // 'stdio' | 'sse' | 'http' | 'builtin'
    command: string | null;
    environment_variables: string | null;
    url: string | null;
    timeout: number | null;
    is_long_running: boolean;
    is_enabled: boolean;
    is_builtin: boolean; // 标识是否为内置服务器
    created_time: string;
}

export interface MCPServerTool {
    id: number;
    server_id: number;
    tool_name: string;
    tool_description: string | null;
    is_enabled: boolean;
    is_auto_run: boolean;
    parameters: string | null; // JSON string of tool parameters
}

export interface MCPServerResource {
    id: number;
    server_id: number;
    resource_uri: string;
    resource_name: string;
    resource_type: string;
    resource_description: string | null;
}

export interface MCPServerPrompt {
    id: number;
    server_id: number;
    prompt_name: string;
    prompt_description: string | null;
    is_enabled: boolean;
    arguments: string | null; // JSON string of prompt arguments
}

export interface MCPServerRequest {
    name: string;
    description?: string;
    transport_type: string;
    command?: string;
    environment_variables?: string;
    url?: string;
    timeout?: number;
    is_long_running: boolean;
    is_enabled: boolean;
    is_builtin?: boolean; // 可选字段，用于创建内置服务器
}

export interface MCPToolConfig {
    tool_name: string;
    is_enabled: boolean;
    is_auto_run: boolean;
}

export interface MCPTransportConfig {
    stdio?: {
        command: string[];
        environment_variables?: Record<string, string>;
    };
    sse?: {
        url: string;
        headers?: Record<string, string>;
    };
    http?: {
        url: string;
        headers?: Record<string, string>;
    };
}

export type MCPTransportType = 'stdio' | 'sse' | 'http';

export const MCP_TRANSPORT_TYPES: { value: MCPTransportType; label: string }[] = [
    { value: 'stdio', label: 'Stdio' },
    { value: 'sse', label: 'SSE (Server-Sent Events)' },
    { value: 'http', label: 'HTTP' },
    // { value: 'builtin', label: '内置工具' }, // Removed builtin transport option
];

export interface MCPConnectionStatus {
    server_id: number;
    is_connected: boolean;
    error_message?: string;
    last_test_time?: string;
}