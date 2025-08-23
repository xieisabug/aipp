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
            const { question, modelId, prompt, conversationId } = options;
            console.log("ask AI", question, modelId, prompt, conversationId);
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
    };

    return {
        assistantRunApi,
    };
}