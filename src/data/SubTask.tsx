// Sub Task 相关的 TypeScript 类型定义

export interface SubTaskDefinition {
    id: number;
    name: string;
    code: string;
    description: string;
    system_prompt: string;
    plugin_source: "mcp" | "plugin";
    source_id: number;
    is_enabled: boolean;
    created_time: Date;
    updated_time: Date;
}

export interface SubTaskExecutionSummary {
    id: number;
    task_code: string;
    task_name: string;
    task_prompt: string;
    status: "pending" | "running" | "success" | "failed" | "cancelled";
    created_time: Date;
    token_count: number;
}

export interface SubTaskExecutionDetail extends SubTaskExecutionSummary {
    result_content?: string;
    error_message?: string;
    llm_model_name?: string;
    input_token_count: number;
    output_token_count: number;
    started_time?: Date;
    finished_time?: Date;
}

export interface CreateSubTaskRequest {
    task_code: string;
    task_prompt: string;
    parent_conversation_id: number;
    parent_message_id?: number;
    source_id: number;
    ai_params?: SubTaskExecutionParams;
}

export interface SubTaskExecutionParams {
    temperature?: number;
    top_p?: number;
    max_tokens?: number;
    custom_model_id?: number;
}

export interface RegisterSubTaskDefinitionRequest {
    name: string;
    code: string;
    description: string;
    system_prompt: string;
    plugin_source: "mcp" | "plugin";
    source_id: number;
}

export interface UpdateSubTaskDefinitionRequest {
    id: number;
    name?: string;
    description?: string;
    system_prompt?: string;
    is_enabled?: boolean;
    source_id: number; // 用于鉴权
}

export interface SubTaskStatusUpdateEvent {
    execution_id: number;
    task_code: string;
    task_name: string;
    parent_conversation_id: number;
    parent_message_id?: number;
    status: "pending" | "running" | "success" | "failed" | "cancelled";
    result_content?: string;
    error_message?: string;
    token_count?: number;
    started_time?: Date;
    finished_time?: Date;
}

export interface ListSubTaskDefinitionsParams {
    plugin_source?: "mcp" | "plugin";
    source_id?: number;
    is_enabled?: boolean;
}

export interface ListSubTaskExecutionsParams {
    parent_conversation_id: number;
    parent_message_id?: number;
    status?: "pending" | "running" | "success" | "failed" | "cancelled";
    source_id?: number;
    page?: number;
    page_size?: number;
}

// Hook 和组件相关的类型
export interface UseSubTaskManagerOptions {
    conversation_id: number;
    message_id?: number;
    // source_id 在UI层面不需要，只在MCP/plugin开发时需要
}

export interface UseSubTaskEventsOptions {
    conversation_id: number;
    onStatusUpdate?: (event: SubTaskStatusUpdateEvent) => void;
    onTaskCompleted?: (execution: SubTaskExecutionDetail) => void;
    onTaskFailed?: (execution: SubTaskExecutionDetail) => void;
}

export interface SubTaskListProps {
    conversation_id: number;
    message_id?: number;
    source_id?: number;
    className?: string;
}

export interface SubTaskItemProps {
    execution: SubTaskExecutionSummary;
    onViewDetail?: (execution: SubTaskExecutionSummary) => void;
    onCancel?: (execution_id: number) => void;
}

export interface CreateSubTaskDialogProps {
    isOpen: boolean;
    onClose: () => void;
    conversation_id: number;
    message_id?: number;
    source_id: number;
    availableDefinitions: SubTaskDefinition[];
    onTaskCreated?: (execution_id: number) => void;
}

export interface SubTaskDetailDialogProps {
    isOpen: boolean;
    onClose: () => void;
    execution_id: number;
    source_id: number;
}

export interface SubTaskStatusIndicatorProps {
    status: "pending" | "running" | "success" | "failed" | "cancelled";
    size?: "sm" | "md" | "lg";
}

// 服务层接口
export interface SubTaskService {
    // 任务定义管理
    registerDefinition: (request: RegisterSubTaskDefinitionRequest) => Promise<number>;
    listDefinitions: (params?: ListSubTaskDefinitionsParams) => Promise<SubTaskDefinition[]>;
    getDefinition: (code: string, source_id: number) => Promise<SubTaskDefinition | null>;
    updateDefinition: (request: UpdateSubTaskDefinitionRequest) => Promise<void>;
    deleteDefinition: (id: number, source_id: number) => Promise<void>;

    // 任务执行管理
    createExecution: (request: CreateSubTaskRequest) => Promise<number>;
    listExecutions: (params: ListSubTaskExecutionsParams) => Promise<SubTaskExecutionSummary[]>;
    getExecutionDetail: (execution_id: number, source_id: number) => Promise<SubTaskExecutionDetail | null>;
    cancelExecution: (execution_id: number, source_id: number) => Promise<void>;
}
