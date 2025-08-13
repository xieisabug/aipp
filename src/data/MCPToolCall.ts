export interface MCPToolCall {
    id: number;
    conversation_id: number;
    message_id?: number;
    server_id: number;
    server_name: string;
    tool_name: string;
    parameters: string;
    status: 'pending' | 'executing' | 'success' | 'failed';
    result?: string;
    error?: string;
    created_time: string;
    started_time?: string;
    finished_time?: string;
}

export interface CreateMCPToolCallRequest {
    conversation_id: number;
    message_id?: number;
    server_name: string;
    tool_name: string;
    parameters: string;
    [key: string]: unknown; // Index signature for Tauri compatibility
}