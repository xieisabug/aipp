export interface Conversation {
    id: number;
    name: string;
    assistant_id: number | null;
    assistant_name: string;
    created_time: Date;
}

export interface Message {
    id: number;
    conversation_id: number;
    message_type: string;
    content: string;
    llm_model_id: number | null;
    created_time: Date;
    token_count: number;
    regenerate: Array<Message> | null;
}

// 流式事件数据类型
export interface StreamEvent {
    message_id: number;
    message_type: 'reasoning' | 'response' | 'error';
    content: string;
    is_done: boolean;
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