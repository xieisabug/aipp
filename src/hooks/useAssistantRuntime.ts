import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useRef } from "react";
import { Conversation, Message, FileInfo } from "../data/Conversation";
import { AssistantDetail } from "../data/Assistant";

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
    // Cache for the current assistant's model ID
    const [cachedModelId, setCachedModelId] = useState<string>("");

    // Internal conversation state for dynamically created conversations
    const [runtimeConversation, setRuntimeConversation] = useState<Conversation | null>(null);

    // A ref that always points to the most up-to-date effective conversation,
    // avoiding stale-closure issues when calling API methods right after creation.
    const effectiveConversationRef = useRef<Conversation | null>(null);

    // Helper function to get the effective conversation (prefers ref to avoid stale closures)
    const getEffectiveConversation = () =>
        effectiveConversationRef.current || runtimeConversation || conversation || null;

    // Keep the ref in sync when props.conversation changes
    useEffect(() => {
        if (conversation && conversation.id) {
            effectiveConversationRef.current = conversation;
        } else if (!runtimeConversation) {
            // Only clear when we also don't have a runtime conversation
            effectiveConversationRef.current = null;
        }
    }, [conversation, runtimeConversation]);

    // Load assistant model ID when assistant changes
    useEffect(() => {
        const loadAssistantModelId = async () => {
            try {
                const effectiveConversation = getEffectiveConversation();
                const assistantId = effectiveConversation?.assistant_id || selectedAssistant;
                if (!assistantId) {
                    setCachedModelId("");
                    return;
                }

                const assistantDetail = await invoke<AssistantDetail>("get_assistant", {
                    assistantId: +assistantId,
                });

                if (assistantDetail.model.length > 0) {
                    setCachedModelId(assistantDetail.model[0].model_code);
                } else {
                    setCachedModelId("");
                }
            } catch (error) {
                console.error("Failed to load assistant model ID:", error);
                setCachedModelId("");
            }
        };

        loadAssistantModelId();
    }, [conversation?.assistant_id, selectedAssistant, runtimeConversation?.assistant_id]);

    // 助手运行时API接口，提供给插件在运行时使用
    const assistantRunApi: AssistantRunApi = {
        askAssistant: function (options: AskAssistantOptions): Promise<AiResponse> {
            const {
                question,
                assistantId,
                conversationId,
                fileInfoList: fileInfoListParam,
                overrideModelConfig,
                overrideSystemPrompt,
                overrideModelId,
                overrideMcpConfig,
                onCustomUserMessage,
                onCustomUserMessageComing: _onCustomUserMessageComing,
                onStreamMessageListener: _onStreamMessageListener,
            } = options;

            let userMessage: any;
            if (onCustomUserMessage) {
                userMessage = onCustomUserMessage(question, assistantId, conversationId);
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
                    override_model_id: overrideModelId,
                    attachment_list: fileInfoListParam?.map((i) => i.id),
                },
                overrideModelConfig: overrideModelConfig,
                overridePrompt: overrideSystemPrompt,
                overrideMcpConfig: overrideMcpConfig,
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
            return inputText;
        },
        getModelId: function (): string {
            return cachedModelId;
        },
        getField: async function (assistantId: string, fieldName: string): Promise<string> {
            return await invoke<string>("get_assistant_field_value", {
                assistantId: +assistantId,
                fieldName,
            });
        },
        appendAiResponse: function (messageId: number, response: string): void {
            console.log("append ai response", messageId, response);
            setMessages((prevMessages) => {
                const newMessages = [...prevMessages];
                const index = newMessages.findIndex((msg) => msg.id === messageId);
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
                const index = newMessages.findIndex((msg) => msg.id === messageId);
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
            const effectiveConversation = getEffectiveConversation();
            if (!effectiveConversation || !effectiveConversation.id) {
                return selectedAssistant + "";
            } else {
                return effectiveConversation.assistant_id + "";
            }
        },
        getConversationId: function (): string {
            const effectiveConversation = getEffectiveConversation();
            if (!effectiveConversation || !effectiveConversation.id) {
                return "";
            } else {
                return effectiveConversation.id + "";
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
            const effectiveConversation = getEffectiveConversation();
            const targetConversationId =
                conversationId || (effectiveConversation?.id ? +effectiveConversation.id : undefined);
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

        createConversation: async function (
            systemPrompt: string,
            userPrompt: string
        ): Promise<CreateConversationResponse> {
            // Validation: prevent creating conversation when one already exists
            const effectiveConversation = getEffectiveConversation();
            if (effectiveConversation?.id) {
                throw new Error("Cannot create conversation: conversation already exists");
            }

            console.log("Creating conversation with system and user prompts");
            try {
                const assistantId = effectiveConversation?.assistant_id || selectedAssistant;
                if (!assistantId) {
                    throw new Error("No assistant selected");
                }

                const result = await invoke<CreateConversationResponse>("create_conversation_with_messages", {
                    assistantId: +assistantId,
                    systemPrompt: systemPrompt.trim() || undefined,
                    userMessage: userPrompt.trim() || undefined,
                    conversationName: undefined, // 使用默认名称
                });

                console.log("Conversation created:", result);

                // Update runtime conversation state with the newly created conversation
                const newConversation: Conversation = {
                    id: result.conversation_id,
                    name: "", // Will be set by backend with default name
                    assistant_id: +assistantId,
                    assistant_name: "", // Will be populated by backend
                    created_time: new Date(),
                };
                // Update ref immediately to make it available to subsequent API calls
                effectiveConversationRef.current = newConversation;
                setRuntimeConversation(newConversation);

                // Notify parent component of conversation change
                onChangeConversationId(result.conversation_id + "");

                return result;
            } catch (error) {
                console.error("Failed to create conversation:", error);
                throw error;
            }
        },

        runSubTask: async function (code: string, taskPrompt: string): Promise<SubTaskRunResult> {
            console.log("Running sub task:", code, "with prompt:", taskPrompt);
            try {
                const effectiveConversation = getEffectiveConversation();
                const assistantId = effectiveConversation?.assistant_id || selectedAssistant;
                const conversationIdNumber = effectiveConversation?.id ? +effectiveConversation.id : 0;

                if (!assistantId) {
                    throw new Error("No assistant selected for running sub task");
                }

                if (!conversationIdNumber) {
                    throw new Error("No conversation context available for running sub task");
                }

                const result = await invoke<SubTaskRunResult>("run_sub_task_sync", {
                    code,
                    taskPrompt,
                    conversationId: conversationIdNumber,
                    assistantId: +assistantId,
                });

                console.log("Sub task completed:", result);
                return result;
            } catch (error) {
                console.error("Failed to run sub task:", error);
                throw error;
            }
        },

        runSubTaskWithMcpLoop: async function (code: string, taskPrompt: string, options: McpLoopOptions): Promise<SubTaskRunWithMcpResult> {
            console.log("Running sub task with MCP loop:", code, "with prompt:", taskPrompt, "options:", options);
            try {
                const effectiveConversation = getEffectiveConversation();
                const assistantId = effectiveConversation?.assistant_id || selectedAssistant;
                const conversationIdNumber = effectiveConversation?.id ? +effectiveConversation.id : 0;

                if (!assistantId) {
                    throw new Error("No assistant selected for running sub task with MCP loop");
                }

                if (!conversationIdNumber) {
                    throw new Error("No conversation context available for running sub task with MCP loop");
                }

                const result = await invoke<SubTaskRunWithMcpResult>("run_sub_task_with_mcp_loop", {
                    code,
                    taskPrompt,
                    conversationId: conversationIdNumber,
                    assistantId: +assistantId,
                    options,
                });

                console.log("Sub task with MCP loop completed:", result);
                return result;
            } catch (error) {
                console.error("Failed to run sub task with MCP loop:", error);
                throw error;
            }
        },
    };

    return {
        assistantRunApi,
    };
}
