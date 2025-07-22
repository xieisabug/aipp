import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { Sparkles } from "lucide-react";

import { Conversation, FileInfo, Message, StreamEvent } from "../data/Conversation";
import "katex/dist/katex.min.css";
import { listen } from "@tauri-apps/api/event";
import { throttle } from "lodash";
import NewChatComponent from "./NewChatComponent";
import FileDropArea from "./FileDropArea";
import MessageItem from "./MessageItem";
import ConversationTitle from "./conversation/ConversationTitle";
import useFileDropHandler from "../hooks/useFileDropHandler";
import InputArea from "./conversation/InputArea";
import FormDialog from "./FormDialog";
import useConversationManager from "../hooks/useConversationManager";
import useFileManagement from "@/hooks/useFileManagement";

interface AssistantListItem {
    id: number;
    name: string;
    assistant_type: number;
}

interface ConversationUIProps {
    conversationId: string;
    onChangeConversationId: (conversationId: string) => void;
    pluginList: any[];
}

// 用于存储AskAssistantApi中对应的处理函数
interface AskAssistantApiFunctions {
    onCustomUserMessage?: (
        question: string,
        assistantId: string,
        conversationId?: string,
    ) => any;
    onCustomUserMessageComing?: (aiResponse: AiResponse) => void;
    onStreamMessageListener?: (
        payload: string,
        aiResponse: AiResponse,
        responseIsResponsingFunction: (isFinish: boolean) => void,
    ) => void;
}

function ConversationUI({
    conversationId,
    onChangeConversationId,
    pluginList,
}: ConversationUIProps) {
    // 插件实例
    const [assistantTypePluginMap, setAssistantTypePluginMap] = useState<
        Map<number, TeaAssistantTypePlugin>
    >(new Map());
    const assistantTypeApi: AssistantTypeApi = {
        typeRegist: (
            code: number,
            _: string,
            pluginInstance: TeaAssistantTypePlugin & TeaPlugin,
        ) => {
            setAssistantTypePluginMap((prev) => {
                const newMap = new Map(prev);
                newMap.set(code, pluginInstance);
                return newMap;
            });
        },
        markdownRemarkRegist: (_: any) => {

        },
        changeFieldLabel: (_: string, __: string) => { },
        addField: (
            _: string,
            __: string,
            ___: string,
            ____?: FieldConfig,
        ) => { },
        addFieldTips: (_: string, __: string) => { },
        hideField: (_: string) => { },
        runLogic: (_: (assistantRunApi: AssistantRunApi) => void) => { },
        forceFieldValue: (_: string, __: string) => { },
    };
    const [functionMap, setFunctionMap] = useState<
        Map<number, AskAssistantApiFunctions>
    >(new Map());
    const assistantRunApi: AssistantRunApi = {
        askAI: function (
            question: string,
            modelId: string,
            prompt?: string,
            conversationId?: string,
        ): AskAiResponse {
            console.log("ask AI", question, modelId, prompt, conversationId);
            return {
                answer: "",
            };
        },
        askAssistant: function (
            question: string,
            assistantId: string,
            conversationId?: string,
            fileInfoList?: FileInfo[],
            overrideModelConfig?: Map<string, any>,
            overrideSystemPrompt?: string,
            onCustomUserMessage?: (
                question: string,
                assistantId: string,
                conversationId?: string,
            ) => any,
            onCustomUserMessageComing?: (_: AiResponse) => void,
            onStreamMessageListener?: (
                _: string,
                __: AiResponse,
                responseFinishFunction: (_: boolean) => void,
            ) => void,
        ): Promise<AiResponse> {
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
                    attachment_list: fileInfoList?.map((i) => i.id),
                },
                overrideModelConfig: overrideModelConfig,
                overridePrompt: overrideSystemPrompt,
            })
                .then((res) => {
                    console.log("ask assistant response", res);
                    if (unsubscribeRef.current) {
                        console.log(
                            "Unsubscribing from previous event listener",
                        );
                        unsubscribeRef.current.then((f) => f());
                    }
                    console.log(`init ${res.add_message_id} function map`);
                    setFunctionMap((prev) => {
                        const newMap = new Map(prev);
                        newMap.set(res.add_message_id, {
                            onCustomUserMessage,
                            onCustomUserMessageComing,
                            onStreamMessageListener,
                        });
                        return newMap;
                    });

                    const customUserMessageComing = functionMap.get(
                        res.add_message_id,
                    )?.onCustomUserMessageComing;
                    if (customUserMessageComing) {
                        customUserMessageComing(res);
                    } else {
                        setMessageId(res.add_message_id);

                        setMessages((prevMessages) => {
                            const newMessages = [...prevMessages];
                            const index = prevMessages.findIndex(
                                (msg) => msg == userMessage,
                            );
                            if (index !== -1) {
                                newMessages[index] = {
                                    ...newMessages[index],
                                    content:
                                        res.request_prompt_result_with_context,
                                };
                            }
                            return newMessages;
                        });
                    }

                    if (conversationId != res.conversation_id + "") {
                        onChangeConversationId(res.conversation_id + "");
                    } else {
                        const assistantMessage = {
                            id: res.add_message_id,
                            conversation_id: conversationId
                                ? -1
                                : +conversationId,
                            llm_model_id: -1,
                            content: "",
                            token_count: 0,
                            message_type: "assistant",
                            created_time: new Date(),
                            attachment_list: [],
                            regenerate: null,
                        };

                        setMessages((prevMessages) => [
                            ...prevMessages,
                            assistantMessage,
                        ]);
                    }

                    console.log(
                        "Listening for response",
                        `message_${res.add_message_id}`,
                    );

                    // 使用节流函数降低 setMessages 调用频率
                    const updateAssistantContent = throttle((streamEvent: StreamEvent) => {
                        if (streamEvent.message_type === 'response') {
                            // 现在response消息也通过streamingMessages管理
                            setStreamingMessages((prev) => {
                                const newMap = new Map(prev);
                                newMap.set(streamEvent.message_id, streamEvent);
                                return newMap;
                            });
                            scroll(); // 确保滚动到最新消息
                        } else if (streamEvent.message_type === 'reasoning') {
                            // 更新流式消息状态
                            setStreamingMessages((prev) => {
                                const newMap = new Map(prev);
                                newMap.set(streamEvent.message_id, streamEvent);
                                return newMap;
                            });
                        }
                    }, 50);

                    unsubscribeRef.current = listen(
                        `message_${res.add_message_id}`,
                        (event) => {
                            const streamMessageListener = functionMap.get(
                                res.add_message_id,
                            )?.onStreamMessageListener;
                            if (streamMessageListener) {
                                // 兼容旧版插件API，转换为字符串格式
                                const payload = typeof event.payload === 'object' 
                                    ? (event.payload as StreamEvent).content 
                                    : event.payload as string;
                                streamMessageListener(
                                    payload,
                                    res,
                                    setAiIsResponsing,
                                );
                            } else {
                                if (typeof event.payload === 'object') {
                                    // 新格式：JSON 对象
                                    const streamEvent = event.payload as StreamEvent;
                                    if (streamEvent.is_done) {
                                        // 确保最后一帧内容被渲染
                                        (updateAssistantContent as any).flush?.();
                                        if (streamEvent.message_type === 'response') {
                                            setAiIsResponsing(false);
                                        }
                                    } else {
                                        updateAssistantContent(streamEvent);
                                    }
                                } else {
                                    // 旧格式兼容：字符串
                                    const payload = event.payload as string;
                                    if (payload !== "Tea::Event::MessageFinish") {
                                        // 构造兼容的 StreamEvent 对象
                                        const streamEvent: StreamEvent = {
                                            message_id: res.add_message_id,
                                            message_type: 'response',
                                            content: payload,
                                            is_done: false,
                                        };
                                        updateAssistantContent(streamEvent);
                                    } else {
                                        // 确保最后一帧内容被渲染
                                        (updateAssistantContent as any).flush?.();
                                        setAiIsResponsing(false);
                                    }
                                }
                            }
                        },
                    );
                    return res;
                })
                .catch((e) => {
                    console.error("ask assistant error", e);
                    toast.error("发送消息失败: " + e);
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
                    scroll();
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
                    scroll();
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

    useEffect(() => {
        // 加载助手类型的插件
        pluginList
            .filter((plugin: any) =>
                plugin.pluginType.includes("assistantType"),
            )
            .forEach((plugin: any) => {
                plugin.instance?.onAssistantTypeInit(assistantTypeApi);
            });
    }, [pluginList]);

    // 是否应用滚动，默认是
    const [isUserScrolling, setIsUserScrolling] = useState(true);
    const scroll = throttle(() => {
        if (!isUserScrolling && messagesEndRef.current) {
            messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
        }
    }, 300);
    useEffect(() => {
        let lastScrollTop = 0;

        const handleScroll = () => {
            if (messagesEndRef.current) {
                const { scrollTop, scrollHeight, clientHeight } =
                    messagesEndRef.current.parentElement!;
                const isScrollingUp = scrollTop < lastScrollTop;

                if (isScrollingUp) {
                    setIsUserScrolling(true);
                }

                const isAtBottom = scrollHeight - scrollTop === clientHeight;
                if (isAtBottom) {
                    setIsUserScrolling(false);
                }

                lastScrollTop = scrollTop <= 0 ? 0 : scrollTop; // For Mobile or negative scrolling
            }
        };

        const messagesContainer = messagesEndRef.current?.parentElement;
        messagesContainer?.addEventListener("scroll", handleScroll);

        return () => {
            messagesContainer?.removeEventListener("scroll", handleScroll);
        };
    }, []);

    // 处理选中文字展示的部分
    const [selectedText, setSelectedText] = useState<string>("");
    useEffect(() => {
        invoke<string>("get_selected_text_api").then((text) => {
            console.log("get_selected_text_api", text);
            setSelectedText(text);
        });

        listen<string>("get_selected_text_event", (event) => {
            console.log("get_selected_text_event", event.payload);
            setSelectedText(event.payload);
        });
    }, []);

    const unsubscribeRef = useRef<Promise<() => void> | null>(null);
    const messagesEndRef = useRef<HTMLDivElement | null>(null);

    const [messages, setMessages] = useState<Array<Message>>([]);
    // 存储流式消息的临时状态，按 conversation_id 分组
    const [streamingMessages, setStreamingMessages] = useState<Map<number, StreamEvent>>(new Map());
    const [conversation, setConversation] = useState<Conversation>();
    const [assistants, setAssistants] = useState<AssistantListItem[]>([]);

    const [isLoadingShow, setIsLoadingShow] = useState(false);
    useEffect(() => {
        if (!conversationId) {
            setMessages([]);
            setConversation(undefined);

            invoke<Array<AssistantListItem>>("get_assistants").then(
                (assistantList) => {
                    setAssistants(assistantList);
                    if (assistantList.length > 0) {
                        setSelectedAssistant(assistantList[0].id);
                    }
                },
            );
            return;
        }
        setIsLoadingShow(true);
        console.log(`conversationId change : ${conversationId}`);
        invoke<Array<any>>("get_conversation_with_messages", {
            conversationId: +conversationId,
        }).then((res: any[]) => {
            setMessages(res[1]);
            setConversation(res[0]);
            setIsLoadingShow(false);

            console.log(res);

            if (unsubscribeRef.current) {
                console.log("Unsubscribing from previous event listener");
                unsubscribeRef.current.then((f) => f());
            }

            const lastMessageId = res[1][res[1].length - 1].id;

            setMessageId(lastMessageId);
            unsubscribeRef.current = listen(
                `message_${lastMessageId}`,
                (event) => {
                    const streamMessageListener =
                        functionMap.get(lastMessageId)?.onStreamMessageListener;
                    if (streamMessageListener) {
                        // 兼容旧版插件API
                        const payload = typeof event.payload === 'object' 
                            ? (event.payload as StreamEvent).content 
                            : event.payload as string;
                        streamMessageListener(
                            payload,
                            {
                                conversation_id: +conversationId,
                                add_message_id: lastMessageId,
                                request_prompt_result_with_context: "",
                            },
                            setAiIsResponsing,
                        );
                    } else {
                        if (typeof event.payload === 'object') {
                            // 新格式：JSON 对象
                            const streamEvent = event.payload as StreamEvent;
                            if (streamEvent.is_done) {
                                setAiIsResponsing(false);
                            } else {
                                // 更新流式消息状态 (reasoning和response都通过streamingMessages管理)
                                if (streamEvent.message_type === 'reasoning' || streamEvent.message_type === 'response') {
                                    setStreamingMessages((prev) => {
                                        const newMap = new Map(prev);
                                        newMap.set(streamEvent.message_id, streamEvent);
                                        return newMap;
                                    });
                                    scroll();
                                }
                            }
                        } else {
                            // 旧格式兼容：字符串
                            const payload = event.payload as string;
                            if (payload !== "Tea::Event::MessageFinish") {
                                // 构造兼容的StreamEvent对象
                                const streamEvent: StreamEvent = {
                                    message_id: lastMessageId,
                                    message_type: 'response',
                                    content: payload,
                                    is_done: false,
                                };
                                setStreamingMessages((prev) => {
                                    const newMap = new Map(prev);
                                    newMap.set(streamEvent.message_id, streamEvent);
                                    return newMap;
                                });
                                scroll();
                            } else {
                                setAiIsResponsing(false);
                            }
                        }
                    }
                },
            );
        });

        return () => {
            if (unsubscribeRef.current) {
                console.log("unsubscribe");
                unsubscribeRef.current.then((f) => f());
            }
        };
    }, [conversationId]);

    useEffect(() => {
        const unsubscribe = listen("title_change", (event) => {
            const [conversationId, title] = event.payload as [number, string];

            if (conversation && conversation.id === conversationId) {
                const newConversation = { ...conversation, name: title };
                setConversation(newConversation);
            }
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, [conversation]);

    useEffect(() => {
        scroll();
    }, [messages]);

    const {
        fileInfoList,
        clearFileInfoList,
        handleChooseFile,
        handleDeleteFile,
        handlePaste,
    } = useFileManagement();

    const [inputText, setInputText] = useState("");
    const [aiIsResponsing, setAiIsResponsing] = useState<boolean>(false);
    const [messageId, setMessageId] = useState<number>(-1);
    const handleSend = throttle(() => {
        if (aiIsResponsing) {
            console.log("Cancelling AI");
            console.log(messageId);
            invoke("cancel_ai", { messageId }).then(() => {
                setAiIsResponsing(false);
            });
        } else {
            if (inputText.trim() === "") {
                setInputText("");
                return;
            }
            setAiIsResponsing(true);

            let conversationId = "";
            let assistantId = "";
            if (!conversation || !conversation.id) {
                assistantId = selectedAssistant + "";
            } else {
                conversationId = conversation.id + "";
                assistantId = conversation.assistant_id + "";
            }

            const assistantData = assistants.find((a) => a.id === +assistantId);
            if (assistantData?.assistant_type !== 0) {
                assistantTypePluginMap
                    .get(assistantData?.assistant_type ?? 0)
                    ?.onAssistantTypeRun(assistantRunApi);
            } else {
                try {
                    const userMessage = {
                        id: 0,
                        conversation_id: conversationId ? +conversationId : -1,
                        llm_model_id: -1,
                        content: inputText,
                        token_count: 0,
                        message_type: "user",
                        created_time: new Date(),
                        attachment_list: [],
                        regenerate: null,
                    };

                    setMessages((prevMessages) => [
                        ...prevMessages,
                        userMessage,
                    ]);
                    invoke<AiResponse>("ask_ai", {
                        request: {
                            prompt: inputText,
                            conversation_id: conversationId,
                            assistant_id: +assistantId,
                            attachment_list: fileInfoList?.map((i) => i.id),
                        },
                    }).then((res) => {
                        console.log("ask ai response", res);
                        if (unsubscribeRef.current) {
                            console.log(
                                "Unsubscribing from previous event listener",
                            );
                            unsubscribeRef.current.then((f) => f());
                        }

                        setMessageId(res.add_message_id);

                        setMessages((prevMessages) => {
                            const newMessages = [...prevMessages];
                            const index = prevMessages.findIndex(
                                (msg) => msg == userMessage,
                            );
                            if (index !== -1) {
                                newMessages[index] = {
                                    ...newMessages[index],
                                    content:
                                        res.request_prompt_result_with_context,
                                };
                            }
                            return newMessages;
                        });

                        if (conversationId != res.conversation_id + "") {
                            onChangeConversationId(res.conversation_id + "");
                        }
                        // 不预先创建空的assistant消息，让流式处理动态创建

                        console.log(
                            "Listening for response",
                            `message_${res.add_message_id}`,
                        );

                        unsubscribeRef.current = listen(
                            `message_${res.add_message_id}`,
                            (event) => {
                                if (typeof event.payload === 'object') {
                                    // 新格式：JSON 对象
                                    const streamEvent = event.payload as StreamEvent;
                                    if (streamEvent.is_done) {
                                        setAiIsResponsing(false);
                                    } else {
                                        // reasoning和response消息都通过streamingMessages管理
                                        if (streamEvent.message_type === 'response') {
                                            setStreamingMessages((prev) => {
                                                const newMap = new Map(prev);
                                                newMap.set(streamEvent.message_id, streamEvent);
                                                return newMap;
                                            });
                                            scroll();
                                        } else if (streamEvent.message_type === 'reasoning') {
                                            // 更新流式消息状态
                                            setStreamingMessages((prev) => {
                                                const newMap = new Map(prev);
                                                newMap.set(streamEvent.message_id, streamEvent);
                                                return newMap;
                                            });
                                        }
                                    }
                                } else {
                                    // 旧格式兼容：字符串
                                    const payload = event.payload as string;
                                    if (payload !== "Tea::Event::MessageFinish") {
                                        // 构造兼容的StreamEvent对象
                                        const streamEvent: StreamEvent = {
                                            message_id: res.add_message_id,
                                            message_type: 'response',
                                            content: payload,
                                            is_done: false,
                                        };
                                        setStreamingMessages((prev) => {
                                            const newMap = new Map(prev);
                                            newMap.set(streamEvent.message_id, streamEvent);
                                            return newMap;
                                        });
                                        scroll();
                                    } else {
                                        setAiIsResponsing(false);
                                    }
                                }
                            },
                        );
                    });
                } catch (error) {
                    toast.error("发送消息失败: " + error);
                }
            }

            setInputText("");
            clearFileInfoList();
        }
    }, 200);

    const [selectedAssistant, setSelectedAssistant] = useState(-1);

    const handleArtifact = useCallback((lang: string, inputStr: string) => {
        invoke("run_artifacts", { lang, inputStr })
            .then((res) => {
                console.log(res);
            })
            .catch((error) => {
                toast.error("运行失败: " + JSON.stringify(error));
            });
    }, []);

    // 合并常规消息和流式消息，按时间排序显示
    const allDisplayMessages = useMemo(() => {
        const combinedMessages = [...messages];
        
        // 将流式消息添加到显示列表中
        streamingMessages.forEach((streamEvent) => {
            // 创建一个临时的Message对象用于显示
            const tempMessage: Message = {
                id: streamEvent.message_id,
                conversation_id: conversation?.id || 0,
                message_type: streamEvent.message_type,
                content: streamEvent.content,
                llm_model_id: null,
                created_time: new Date(),
                token_count: 0,
                regenerate: null,
            };
            
            // 检查是否已经存在同样ID的消息
            const existingIndex = combinedMessages.findIndex(msg => msg.id === streamEvent.message_id);
            if (existingIndex === -1) {
                // 不存在则添加新消息
                combinedMessages.push(tempMessage);
            } else {
                // 存在则更新消息内容（这是关键！）
                combinedMessages[existingIndex] = {
                    ...combinedMessages[existingIndex],
                    content: streamEvent.content,
                };
            }
        });

        return combinedMessages.sort((a, b) => new Date(a.created_time).getTime() - new Date(b.created_time).getTime());
    }, [messages, streamingMessages, conversation?.id]);

    const filteredMessages = useMemo(
        () =>
            allDisplayMessages
                .filter((m) => m.message_type !== "system")
                .map((message) => (
                    <MessageItem
                        key={message.id} // 使用唯一的 id 作为 key，而不是索引
                        message={message}
                        onCodeRun={handleArtifact}
                        onMessageRegenerate={() =>
                            handleMessageRegenerate(message.id)
                        }
                    />
                )),
        [allDisplayMessages],
    );

    // 文件拖拽处理
    const { isDragging, setIsDragging, dropRef } =
        useFileDropHandler(handleChooseFile);

    const [formDialogIsOpen, setFormDialogIsOpen] = useState<boolean>(false);
    const openFormDialog = useCallback(() => {
        setFormConversationTitle(conversation?.name || "");
        setFormDialogIsOpen(true);
    }, [conversation]);
    const closeFormDialog = useCallback(() => {
        setFormDialogIsOpen(false);
    }, []);
    const [formConversationTitle, setFormConversationTitle] =
        useState<string>("");
    const [isRegeneratingTitle, setIsRegeneratingTitle] = useState<boolean>(false);

    const handleFormSubmit = useCallback(() => {
        invoke("update_conversation", {
            conversationId: conversation?.id,
            name: formConversationTitle,
        }).then(() => {
            closeFormDialog();
        });
    }, [conversation, formConversationTitle]);

    const handleRegenerateTitle = useCallback(async () => {
        if (!conversation?.id || isRegeneratingTitle) return;

        setIsRegeneratingTitle(true);

        try {
            await invoke("regenerate_conversation_title", {
                conversationId: conversation.id,
            });
            toast.success("标题已重新生成");
        } catch (error) {
            console.error("重新生成标题失败:", error);
            toast.error("重新生成标题失败: " + error);
        } finally {
            setIsRegeneratingTitle(false);
        }
    }, [conversation, isRegeneratingTitle]);

    // 监听标题变化，同步到表单
    useEffect(() => {
        if (formDialogIsOpen && conversation?.name) {
            setFormConversationTitle(conversation.name);
        }
    }, [formDialogIsOpen, conversation?.name]);

    const { deleteConversation } = useConversationManager();
    const handleDeleteConversation = useCallback(() => {
        deleteConversation(conversationId, {
            onSuccess: () => {
                onChangeConversationId("");
            },
        });
    }, [conversationId]);

    const handleMessageRegenerate = useCallback(
        (regenerateMessageId: number) => {
            invoke<AiResponse>("regenerate_ai", {
                messageId: regenerateMessageId,
            }).then((res) => {
                console.log("regenerate ai response", res);

                const assistantMessage = {
                    id: res.add_message_id,
                    conversation_id: conversationId ? -1 : +conversationId,
                    llm_model_id: -1,
                    content: "",
                    token_count: 0,
                    message_type: "assistant",
                    created_time: new Date(),
                    attachment_list: [],
                    regenerate: null,
                };

                setMessages((prevMessages) => {
                    const newMessages = [...prevMessages];
                    const index = newMessages.findIndex(
                        (msg) => msg.id === regenerateMessageId,
                    );
                    if (index !== -1) {
                        if (!newMessages[index].regenerate) {
                            newMessages[index].regenerate = [];
                        }

                        // 检查regenerate里是否存在对应的assistantMessage
                        if (
                            newMessages[index].regenerate.findIndex(
                                (msg) => msg.id === res.add_message_id,
                            ) === -1
                        ) {
                            newMessages[index].regenerate.push(
                                assistantMessage,
                            );
                        }
                    }
                    return newMessages;
                });

                console.log(
                    "Listening for response",
                    `message_${res.add_message_id}`,
                );

                // 再生场景下也使用节流，防止高频 setState
                const updateRegenerateContent = throttle((streamEvent: StreamEvent) => {
                    if (streamEvent.message_type === 'response') {
                        setMessages((prevMessages) => {
                            const newMessages = [...prevMessages];
                            const index = newMessages.findIndex(
                                (msg) => msg.id === regenerateMessageId,
                            );

                            if (index !== -1) {
                                const regenerateIndex =
                                    newMessages[index].regenerate?.findIndex(
                                        (msg) => msg.id === res.add_message_id,
                                    ) ?? -1;

                                if (regenerateIndex !== -1) {
                                    const newRegenerate = [
                                        ...(newMessages[index].regenerate ?? []),
                                    ];
                                    newRegenerate[regenerateIndex] = {
                                        ...newRegenerate[regenerateIndex],
                                        content: streamEvent.content,
                                    };
                                    newMessages[index] = {
                                        ...newMessages[index],
                                        regenerate: newRegenerate,
                                    };
                                }
                            }
                            return newMessages;
                        });
                    } else if (streamEvent.message_type === 'reasoning') {
                        // 更新流式消息状态
                        setStreamingMessages((prev) => {
                            const newMap = new Map(prev);
                            newMap.set(streamEvent.message_id, streamEvent);
                            return newMap;
                        });
                    }
                }, 50);

                unsubscribeRef.current = listen(
                    `message_${res.add_message_id}`,
                    (event) => {
                        if (typeof event.payload === 'object') {
                            // 新格式：JSON 对象
                            const streamEvent = event.payload as StreamEvent;
                            if (streamEvent.is_done) {
                                (updateRegenerateContent as any).flush?.();
                                if (streamEvent.message_type === 'response') {
                                    setAiIsResponsing(false);
                                }
                            } else {
                                updateRegenerateContent(streamEvent);
                            }
                        } else {
                            // 旧格式兼容：字符串
                            const payload = event.payload as string;
                            if (payload !== "Tea::Event::MessageFinish") {
                                const streamEvent: StreamEvent = {
                                    message_id: res.add_message_id,
                                    message_type: 'response',
                                    content: payload,
                                    is_done: false,
                                };
                                updateRegenerateContent(streamEvent);
                            } else {
                                (updateRegenerateContent as any).flush?.();
                                setAiIsResponsing(false);
                            }
                        }
                    },
                );
            });
        },
        [],
    );

    return (
        <div ref={dropRef} className="h-full relative flex flex-col bg-white rounded-xl">
            {conversationId ? (
                <ConversationTitle
                    onEdit={openFormDialog}
                    onDelete={handleDeleteConversation}
                    conversation={conversation}
                />
            ) : null}

            <div className="h-full flex-1 overflow-y-auto flex flex-col p-6 box-border gap-4">
                {conversationId ? (
                    filteredMessages
                ) : (
                    <NewChatComponent
                        selectedText={selectedText}
                        selectedAssistant={selectedAssistant}
                        assistants={assistants}
                        setSelectedAssistant={setSelectedAssistant}
                    />
                )}
                <div className="flex-none h-[120px]"></div>
                <div ref={messagesEndRef} />
            </div>
            {isDragging ? (
                <FileDropArea
                    onDragChange={setIsDragging}
                    onFilesSelect={handleChooseFile}
                />
            ) : null}

            <InputArea
                inputText={inputText}
                setInputText={setInputText}
                fileInfoList={fileInfoList}
                handleChooseFile={handleChooseFile}
                handleDeleteFile={handleDeleteFile}
                handlePaste={handlePaste}
                handleSend={handleSend}
                aiIsResponsing={aiIsResponsing}
                placement="bottom"
            />

            <FormDialog
                title={"修改对话标题"}
                onSubmit={handleFormSubmit}
                onClose={closeFormDialog}
                isOpen={formDialogIsOpen}
            >
                <div className="space-y-4">
                    <div className="space-y-2">
                        <div className="flex items-center justify-between">
                            <label className="text-sm font-medium leading-none text-gray-700">
                                标题
                            </label>
                            <button
                                type="button"
                                onClick={handleRegenerateTitle}
                                disabled={isRegeneratingTitle}
                                className="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-8 px-2 py-1"
                                title="重新生成标题"
                            >
                                <Sparkles className={`h-4 w-4 ${isRegeneratingTitle ? 'animate-pulse' : ''}`} />
                            </button>
                        </div>
                        <input
                            className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 transition-colors"
                            type="text"
                            name="name"
                            value={formConversationTitle}
                            onChange={(e) =>
                                setFormConversationTitle(e.target.value)
                            }
                            placeholder="请输入对话标题"
                            autoFocus
                        />
                    </div>
                </div>
            </FormDialog>

            {isLoadingShow ? (
                <div className="bg-white/95 w-full h-full absolute flex items-center justify-center backdrop-blur rounded-xl">
                    <div className="loading-icon"></div>
                    <div className="text-indigo-500 text-base font-medium">加载中...</div>
                </div>
            ) : null}
        </div>
    );
}

export default ConversationUI;
