interface SystemApi {}

enum PluginType {
    AssistantType = 1,
    InterfaceType = 2,
    ApplicationType = 3,
}

interface Message {
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
    parent_group_id?: string | null;
    parent_id?: number | null;
    regenerate: Array<Message> | null;
    attachment_list?: Array<any>;
}

interface AddFieldOptions {
    fieldName: string;
    label: string;
    type:
        | "select"
        | "textarea"
        | "input"
        | "password"
        | "checkbox"
        | "radio"
        | "static"
        | "custom"
        | "button"
        | "switch"
        | "model-select"
        | "mcp-select";
    fieldConfig?: FieldConfig;
}

interface AskAssistantOptions {
    question: string;
    assistantId: string;
    conversationId?: string;
    fileInfoList?: FileInfo[];
    overrideModelConfig?: Map<string, any>;
    overrideSystemPrompt?: string;
    overrideModelId?: string;
    overrideMcpConfig?: McpOverrideConfig;
    
    // MCP生命周期事件处理器
    onMcpToolDetected?: McpDetectedHandler;
    onMcpToolExecuting?: McpExecutingHandler;
    onMcpToolResult?: McpResultHandler;
    
    // 消息处理回调
    onCustomUserMessage?: (question: string, assistantId: string, conversationId?: string) => any;
    onCustomUserMessageComing?: (aiResponse: AiResponse) => void;
    onStreamMessageListener?: (
        payload: string,
        aiResponse: AiResponse,
        responseIsResponsingFunction: (isFinish: boolean) => void
    ) => void;
}

interface AssistantTypeApi {
    typeRegist(pluginType: PluginType, code: number, label: string, plugin: AippAssistantTypePlugin): void;
    markdownRemarkRegist(component: any): void;
    changeFieldLabel(fieldName: string, label: string): void;
    addField(options: AddFieldOptions): void;
    hideField(fieldName: string): void;
    forceFieldValue(fieldName: string, value: string): void;
    addFieldTips(fieldName: string, tips: string): void;
    runLogic(callback: (assistantRunApi: AssistantRunApi) => void): void;
}

interface AssistantConfigApi {
    clearFieldValue(fieldName: string): void;
    changeFieldValue(fieldName: string, value: string | boolean, valueType: string): void;
}

interface FieldConfig {
    // default none
    position?: "query" | "body" | "header" | "none";
    // default false
    required?: boolean;
    // default false
    hidden?: boolean;
    options?: { value: string; label: string; tooltip?: string }[];
    tips?: string;
    disabled?: boolean;
    onClick?: () => void;
}

interface AssistantRunApi {
    askAssistant(options: AskAssistantOptions): Promise<AiResponse>;
    getUserInput(): string;
    getModelId(): string;
    getAssistantId(): string;
    getConversationId(): string;
    getField(assistantId: string, fieldName: string): Promise<string>;
    appendAiResponse(messageId: number, response: string): void;
    setAiResponse(messageId: number, response: string): void;
    getMcpProvider(providerId: string): Promise<McpProviderInfo | null>;
    buildMcpPrompt(providerIds: string[]): Promise<string>;
    createMessage(markdownText: string, conversationId: number): Promise<Message>;
    updateAssistantMessage(messageId: number, markdownText: string): Promise<void>;
    
    // 保留MCP查询方法
    getMcpToolCalls(conversationId?: number): Promise<McpToolCall[]>;
    getMcpToolCall(callId: number): Promise<McpToolCall | null>;
}

interface AiResponse {
    conversation_id: number;
    request_prompt_result_with_context: string;
}

interface McpToolInfo {
    name: string;
    description: string;
    parameters: string;
    isEnabled: boolean;
    isAutoRun: boolean;
}

interface McpProviderInfo {
    id: string;
    name: string;
    description?: string;
    transportType: string;
    isEnabled: boolean;
    tools: McpToolInfo[];
}

// MCP工具调用生命周期阶段
enum McpLifecycleStage {
    DETECTED = "detected",
    EXECUTING = "executing", 
    RESULT = "result"
}

// DETECTED阶段的操作
enum McpDetectedAction {
    DEFAULT = "default",   // 按照原有MCP默认逻辑处理
    EXECUTE = "execute",   // 执行工具调用（可修改参数）
    SKIP = "skip",        // 跳过此工具调用
    ABORT = "abort"       // 中止整个流程
}

// EXECUTING阶段的操作
enum McpExecutingAction {
    DEFAULT = "default",  // 按照原有MCP默认逻辑处理
    ABORT = "abort"       // 中止执行
}

// RESULT阶段的操作
enum McpResultAction {
    DEFAULT = "default",  // 按照原有MCP默认逻辑处理
    CONTINUE = "continue", // 自动添加结果到对话并继续下一轮
    SKIP = "skip",        // 跳过自动添加，插件自己处理结果
    ABORT = "abort"       // 中止对话流程
}

// MCP生命周期控制接口
interface McpDetectedControl {
    action: McpDetectedAction;
    modifiedParameters?: Record<string, any>;
    reason?: string;
}

interface McpExecutingControl {
    action: McpExecutingAction;
    reason?: string;
}

interface McpResultControl {
    action: McpResultAction;
    customMessage?: string;
    reason?: string;
}

// MCP生命周期事件处理器
type McpDetectedHandler = (
    serverId: string,
    toolName: string,
    parameters: Record<string, any>,
    conversationId: number
) => McpDetectedControl | Promise<McpDetectedControl>;

type McpExecutingHandler = (
    callId: number,
    serverId: string,
    toolName: string,
    status: "running" | "pending"
) => McpExecutingControl | Promise<McpExecutingControl>;

type McpResultHandler = (
    callId: number,
    serverId: string,
    toolName: string,
    result: string,
    error?: string
) => McpResultControl | Promise<McpResultControl>;

// MCP配置覆盖
interface McpOverrideConfig {
    // 覆盖特定工具的自动运行配置
    toolAutoRun?: Record<string, boolean>;  // "serverId/toolName" -> autoRun
    // 覆盖整个服务器的启用状态
    serverEnabled?: Record<string, boolean>;  // "serverId" -> enabled
    // 覆盖是否使用原生工具调用
    useNativeToolcall?: boolean;
    // 自定义MCP工具调用超时时间
    toolCallTimeout?: number;
}

// 保留原有的McpToolCall接口用于查询
interface McpToolCall {
    id: number;
    conversation_id: number;
    message_id?: number;
    server_name: string;
    tool_name: string;
    parameters: string;
    status: "pending" | "running" | "success" | "failed";
    result?: string;
    error?: string;
    created_time: Date;
    started_time?: Date;
    finished_time?: Date;
}

declare class Config {
    name: string;
    type: string[];
}

declare class AippPlugin {
    onPluginLoad(systemApi: SystemApi): void;
    renderComponent?(): React.ReactNode;
    config(): Config;
}

declare class AippAssistantTypePlugin {
    onAssistantTypeInit(assistantTypeApi: AssistantTypeApi): void;
    onAssistantTypeSelect(assistantTypeApi: AssistantTypeApi): void;
    onAssistantTypeRun(assistantRunApi: AssistantRunApi): void;
}
