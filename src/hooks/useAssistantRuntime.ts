import { invoke } from "@tauri-apps/api/core";
import { Conversation, Message, FileInfo } from "../data/Conversation";

export interface UseAssistantRuntimeProps {
    conversation?: Conversation;
    selectedAssistant: number;
    inputText: string;
    fileInfoList?: FileInfo[];
    setMessages: React.Dispatch<React.SetStateAction<Message[]>>;
    onChangeConversationId: (conversationId: string) => void;
    smartScroll: () => void;
    updateShiningMessages: () => void;
    setAiIsResponsing: (isResponsing: boolean) => void;
}

export interface UseAssistantRuntimeReturn {
    assistantRunApi: AssistantRunApi;
}

export function useAssistantRuntime({
    conversation,
    selectedAssistant,
    inputText,
    fileInfoList: _fileInfoList,
    setMessages,
    onChangeConversationId,
    smartScroll,
    updateShiningMessages,
    setAiIsResponsing,
}: UseAssistantRuntimeProps): UseAssistantRuntimeReturn {
    
    // 助手运行时API接口，提供给插件在运行时使用
    const assistantRunApi: AssistantRunApi = {
        askAI: function (options: AskAiOptions): AskAiResponse {
            const { 
                question, 
                modelId, 
                conversationId, 
                fileInfoList: _fileInfoListParam,
                overrideModelConfig,
                overrideSystemPrompt,
                overrideMcpConfig,
                onMcpToolDetected,
                onMcpToolExecuting,
                onMcpToolResult,
                onCustomUserMessage: _onCustomUserMessage,
                onCustomUserMessageComing: _onCustomUserMessageComing,
                onStreamMessageListener: _onStreamMessageListener
            } = options;
            
            console.log("ask AI", {
                question, 
                modelId, 
                conversationId,
                overrideModelConfig,
                overrideSystemPrompt,
                overrideMcpConfig,
                hasMcpHandlers: !!(onMcpToolDetected || onMcpToolExecuting || onMcpToolResult),
                hasFileInfo: !!_fileInfoListParam,
                hasCallbacks: !!(_onCustomUserMessage || _onCustomUserMessageComing || _onStreamMessageListener)
            });
            
            // TODO: 实现完整的askAI逻辑，包括MCP事件处理器的传递
            // 这里需要调用后端API并传递MCP事件处理器
            
            return {
                answer: "",
            };
        },
        askAssistant: function (options: AskAssistantOptions): Promise<AiResponse> {
            const {
                question,
                assistantId,
                conversationId,
                fileInfoList: fileInfoListParam,
                overrideModelConfig,
                overrideSystemPrompt,
                overrideMcpConfig,
                onMcpToolDetected,
                onMcpToolExecuting,
                onMcpToolResult,
                onCustomUserMessage,
                onCustomUserMessageComing: _onCustomUserMessageComing,
                onStreamMessageListener: _onStreamMessageListener
            } = options;
            
            console.log(
                "ask assistant",
                question,
                assistantId,
                conversationId,
                overrideModelConfig,
                overrideSystemPrompt,
                overrideMcpConfig,
                "hasMcpHandlers:",
                !!(onMcpToolDetected || onMcpToolExecuting || onMcpToolResult)
            );
            let userMessage: any;
            if (onCustomUserMessage) {
                userMessage = onCustomUserMessage(
                    question,
                    assistantId,
                    conversationId,
                );
            } else {
                userMessage = {
                    id: 0,
                    conversation_id: conversationId ? +conversationId : -1,
                    llm_model_id: -1,
                    content: question,
                    token_count: 0,
                    message_type: "user",
                    created_time: new Date(),
                    attachment_list: [],
                    regenerate: null,
                };

                setMessages((prevMessages) => [...prevMessages, userMessage]);
            }

            return invoke<AiResponse>("ask_ai", {
                request: {
                    prompt: question,
                    conversation_id: conversationId,
                    assistant_id: +assistantId,
                    attachment_list: fileInfoListParam?.map((i) => i.id),
                },
                overrideModelConfig: overrideModelConfig,
                overridePrompt: overrideSystemPrompt,
                overrideMcpConfig: overrideMcpConfig,
                // 传递MCP事件处理器到后端
                mcpHandlers: {
                    onMcpToolDetected,
                    onMcpToolExecuting,
                    onMcpToolResult
                }
            })
                .then((res) => {
                    console.log("ask assistant response", res);

                    if (conversationId != res.conversation_id + "") {
                        onChangeConversationId(res.conversation_id + "");
                    }

                    // 事件处理现在由共享的 useConversationEvents hook 处理

                    return res;
                })
                .catch((e) => {
                    console.error("ask assistant error", e);
                    setAiIsResponsing(false);
                    // 使用智能边框控制，而不是直接清空
                    updateShiningMessages();
                    // 错误信息将在对话框中显示
                    throw e;
                });
        },
        getUserInput: function (): string {
            console.log("get user input");
            return inputText;
        },
        getModelId: function (): string {
            console.log("get model id");
            return "";
        },
        getField: async function (
            assistantId: string,
            fieldName: string,
        ): Promise<string> {
            console.log("get field", fieldName);
            return await invoke<string>("get_assistant_field_value", {
                assistantId: +assistantId,
                fieldName,
            });
        },
        appendAiResponse: function (messageId: number, response: string): void {
            console.log("append ai response", messageId, response);
            setMessages((prevMessages) => {
                const newMessages = [...prevMessages];
                const index = newMessages.findIndex(
                    (msg) => msg.id === messageId,
                );
                if (index !== -1) {
                    newMessages[index] = {
                        ...newMessages[index],
                        content: newMessages[index].content + response,
                    };
                    smartScroll();
                }
                return newMessages;
            });
        },
        setAiResponse: function (messageId: number, response: string): void {
            console.log("set ai response", messageId, response);
            setMessages((prevMessages) => {
                const newMessages = [...prevMessages];
                const index = newMessages.findIndex(
                    (msg) => msg.id === messageId,
                );
                if (index !== -1) {
                    newMessages[index] = {
                        ...newMessages[index],
                        content: response,
                    };
                    smartScroll();
                }
                return newMessages;
            });
        },
        getAssistantId: function (): string {
            if (!conversation || !conversation.id) {
                return selectedAssistant + "";
            } else {
                return conversation.assistant_id + "";
            }
        },
        getConversationId: function (): string {
            if (!conversation || !conversation.id) {
                return "";
            } else {
                return conversation.id + "";
            }
        },
        getMcpProvider: async function (providerId: string): Promise<McpProviderInfo | null> {
            console.log("get mcp provider", providerId);
            try {
                const result = await invoke<McpProviderInfo | null>("get_mcp_provider", {
                    providerId,
                });
                return result;
            } catch (error) {
                console.error("Failed to get MCP provider:", error);
                return null;
            }
        },
        buildMcpPrompt: async function (providerIds: string[]): Promise<string> {
            console.log("build mcp prompt", providerIds);
            try {
                const result = await invoke<string>("build_mcp_prompt", {
                    providerIds,
                });
                return result;
            } catch (error) {
                console.error("Failed to build MCP prompt:", error);
                return "Failed to build MCP prompt.";
            }
        },
        createMessage: async function (markdownText: string, conversationId: number): Promise<Message> {
            console.log("create message", markdownText, conversationId);
            try {
                const createdMessage = await invoke<Message>("create_message", {
                    markdownText,
                    conversationId,
                });
                
                // Convert the created message to frontend format
                const newMessage: Message = {
                    id: createdMessage.id,
                    conversation_id: createdMessage.conversation_id,
                    llm_model_id: createdMessage.llm_model_id,
                    content: createdMessage.content,
                    token_count: createdMessage.token_count,
                    message_type: createdMessage.message_type,
                    created_time: new Date(createdMessage.created_time),
                    start_time: createdMessage.start_time ? new Date(createdMessage.start_time) : null,
                    finish_time: createdMessage.finish_time ? new Date(createdMessage.finish_time) : null,
                    generation_group_id: createdMessage.generation_group_id,
                    parent_group_id: createdMessage.parent_group_id,
                    parent_id: createdMessage.parent_id,
                    attachment_list: [],
                    regenerate: null,
                };
                
                setMessages((prevMessages) => [...prevMessages, newMessage]);
                smartScroll();
                
                return newMessage;
            } catch (error) {
                console.error("Failed to create message:", error);
                throw error;
            }
        },
        updateAssistantMessage: async function (messageId: number, markdownText: string): Promise<void> {
            console.log("update assistant message", messageId, markdownText);
            try {
                await invoke<void>("update_assistant_message", {
                    messageId,
                    markdownText,
                });
                
                // Update local state
                setMessages((prevMessages) => {
                    const newMessages = [...prevMessages];
                    const index = newMessages.findIndex((msg) => msg.id === messageId);
                    if (index !== -1) {
                        newMessages[index] = {
                            ...newMessages[index],
                            content: markdownText,
                        };
                        smartScroll();
                    }
                    return newMessages;
                });
            } catch (error) {
                console.error("Failed to update assistant message:", error);
                throw error;
            }
        },
        
        // 保留MCP查询方法
        getMcpToolCalls: async function (conversationId?: number): Promise<McpToolCall[]> {
            const targetConversationId = conversationId || (conversation?.id ? +conversation.id : undefined);
            if (!targetConversationId) {
                console.warn("No conversation ID available for getMcpToolCalls");
                return [];
            }
            
            console.log("Getting MCP tool calls for conversation:", targetConversationId);
            try {
                const result = await invoke<McpToolCall[]>("get_mcp_tool_calls", {
                    conversationId: targetConversationId,
                });
                return result;
            } catch (error) {
                console.error("Failed to get MCP tool calls:", error);
                return [];
            }
        },
        
        getMcpToolCall: async function (callId: number): Promise<McpToolCall | null> {
            console.log("Getting MCP tool call:", callId);
            try {
                const result = await invoke<McpToolCall | null>("get_mcp_tool_call", {
                    callId,
                });
                return result;
            } catch (error) {
                console.error("Failed to get MCP tool call:", error);
                return null;
            }
        },
    };

    return {
        assistantRunApi,
    };
}