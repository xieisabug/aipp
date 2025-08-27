export interface Conversation {
    id: number;
    name: string;
    assistant_id: number | null;
    assistant_name: string;
    created_time: Date;
}

// 新增：用于 get_conversation_with_messages API 的响应结构
export interface ConversationWithMessages {
    conversation: Conversation;
    messages: Array<Message>;
}

export interface Message {
    id: number;
    conversation_id: number;
    message_type: string;
    content: string;
    llm_model_id: number | null;
    created_time: Date;
    start_time: Date | null;
    finish_time: Date | null;
    token_count: number;
    generation_group_id?: string | null;
    parent_group_id?: string | null; // 添加 parent_group_id 字段
    parent_id?: number | null; // 添加 parent_id 字段
    regenerate: Array<Message> | null;
    attachment_list?: Array<any>; // 添加附件列表字段
}

// 流式事件数据类型
export interface StreamEvent {
    message_id: number;
    message_type: 'reasoning' | 'response' | 'error';
    content: string;
    is_done: boolean;
    duration_ms?: number; // 后端提供的持续时间
    end_time?: Date; // 后端提供的结束时间
}

// 新增：Conversation 事件类型
export interface ConversationEvent {
    type: string;
    data: any;
}

export interface MessageAddEvent {
    message_id: number;
    message_type: string;
    temp_message_id: number; // 用于取消操作的临时ID
}

export interface MessageUpdateEvent {
    message_id: number;
    message_type: string;
    content: string;
    is_done: boolean;
}

export interface MessageTypeEndEvent {
    message_id: number;
    message_type: string;
    duration_ms: number;
    end_time: Date;
}

export interface GroupMergeEvent {
    original_group_id: string;
    new_group_id: string;
    is_regeneration: boolean;
    first_message_id?: number;
    conversation_id?: number;
}

export interface MCPToolCallUpdateEvent {
    call_id: number;
    conversation_id: number;
    status: 'pending' | 'executing' | 'success' | 'failed';
    result?: string;
    error?: string;
    started_time?: Date;
    finished_time?: Date;
}

// MCP Event Handler Types (for frontend-backend communication)
export interface McpDetectedEventRequest {
    server_id: string;
    tool_name: string;
    parameters: Record<string, any>;
    conversation_id: number;
}

export interface McpExecutingEventRequest {
    call_id: number;
    server_id: string;
    tool_name: string;
    status: 'running' | 'pending';
}

export interface McpResultEventRequest {
    call_id: number;
    server_id: string;
    tool_name: string;
    result: string;
    error?: string;
}

// MCP Control Response Types
export interface McpDetectedControl {
    action: 'Default' | 'Execute' | 'Skip' | 'Abort';
    modified_parameters?: Record<string, any>;
    reason?: string;
}

export interface McpExecutingControl {
    action: 'Default' | 'Abort';
    reason?: string;
}

export interface McpResultControl {
    action: 'Default' | 'Continue' | 'Skip' | 'Abort';
    custom_message?: string;
    reason?: string;
}

// MCP Event Data Types (wrapper for events with response IDs)
export interface McpDetectedEventData {
    request: McpDetectedEventRequest;
    response_id: string;
}

export interface McpExecutingEventData {
    request: McpExecutingEventRequest;
    response_id: string;
}

export interface McpResultEventData {
    request: McpResultEventRequest;
    response_id: string;
}

// 消息类型枚举
export type MessageType = 'system' | 'user' | 'assistant' | 'reasoning' | 'response' | 'error';

export interface AddAttachmentResponse {
    attachment_id: number;
}

export interface FileInfo {
    id: number;
    name: string;
    path: string;
    type: AttachmentType;
    thumbnail?: string;
}

export enum AttachmentType { // 添加AttachmentType枚举
    Image = 1,
    Text = 2,
    PDF = 3,
    Word = 4,
    PowerPoint = 5,
    Excel = 6,
}