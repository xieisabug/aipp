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

interface AskAiOptions {
    question: string;
    modelId: string;
    prompt?: string;
    conversationId?: string;
}

interface AskAssistantOptions {
    question: string;
    assistantId: string;
    conversationId?: string;
    fileInfoList?: FileInfo[];
    overrideModelConfig?: Map<string, any>;
    overrideSystemPrompt?: string;
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
    askAI(options: AskAiOptions): AskAiResponse;
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

declare class AskAiResponse {
    answer: string;
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
