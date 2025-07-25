import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { Sparkles } from "lucide-react";

import { Conversation, FileInfo, Message, StreamEvent, ConversationEvent, MessageUpdateEvent } from "../data/Conversation";
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
    // ============= 插件管理相关状态和逻辑 =============
    
    // 助手类型插件映射表，key为助手类型，value为插件实例
    const [assistantTypePluginMap, setAssistantTypePluginMap] = useState<
        Map<number, TeaAssistantTypePlugin>
    >(new Map());
    
    // 插件函数映射表，用于存储每个消息对应的处理函数
    const [functionMap, setFunctionMap] = useState<
        Map<number, AskAssistantApiFunctions>
    >(new Map());

    // 助手类型API接口，提供给插件使用
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

    // ============= 对话管理相关状态和逻辑 =============
    
    // 当前对话信息和助手列表
    const [conversation, setConversation] = useState<Conversation>();
    const [assistants, setAssistants] = useState<AssistantListItem[]>([]);
    const [selectedAssistant, setSelectedAssistant] = useState(-1);
    
    // 对话加载状态
    const [isLoadingShow, setIsLoadingShow] = useState(false);

    // ============= 消息管理和流式处理相关状态 =============
    
    // 常规消息列表
    const [messages, setMessages] = useState<Array<Message>>([]);
    
    // 流式消息状态管理，存储正在流式传输的消息
    const [streamingMessages, setStreamingMessages] = useState<Map<number, StreamEvent>>(new Map());
    
    // AI响应状态管理
    const [aiIsResponsing, setAiIsResponsing] = useState<boolean>(false);
    const [messageId, setMessageId] = useState<number>(-1);
    
    // 事件监听取消订阅引用
    const unsubscribeRef = useRef<Promise<() => void> | null>(null);

    // ============= UI 状态管理和交互相关逻辑 =============
    
    // 滚动相关状态和逻辑
    const messagesEndRef = useRef<HTMLDivElement | null>(null);
    const scrollContainerRef = useRef<HTMLDivElement | null>(null);
    const isUserScrolledUpRef = useRef(false); // 使用 Ref 来跟踪滚动状态，避免闭包问题
    const isAutoScrolling = useRef(false);
    const resizeObserverRef = useRef<ResizeObserver | null>(null);
    
    // 处理用户滚动事件
    const handleScroll = useCallback(() => {
        // 如果是程序触发的自动滚动，则忽略此次事件
        if (isAutoScrolling.current) {
            return;
        }

        const container = scrollContainerRef.current;
        if (container) {
            const { scrollTop, scrollHeight, clientHeight } = container;
            // 判断是否滚动到了底部，留出 10px 的容差
            const atBottom = scrollHeight - scrollTop - clientHeight < 10;

            // 直接更新 Ref 的值
            isUserScrolledUpRef.current = !atBottom;
        }
    }, []); // 依赖项为空，函数是稳定的

    // 智能滚动函数
    const smartScroll = useCallback(() => {
        // 从 Ref 读取状态，这总是最新的值
        if (isUserScrolledUpRef.current) {
            return;
        }

        const container = scrollContainerRef.current;
        if (!container) return;

        // 清理之前的观察器
        if (resizeObserverRef.current) {
            resizeObserverRef.current.disconnect();
        }

        resizeObserverRef.current = new ResizeObserver(() => {
            // 再次从 Ref 检查，确保万无一失
            if (isUserScrolledUpRef.current || !scrollContainerRef.current) {
                if (resizeObserverRef.current) {
                    resizeObserverRef.current.disconnect();
                }
                return;
            }

            isAutoScrolling.current = true;
            scrollContainerRef.current.scrollTop = scrollContainerRef.current.scrollHeight;

            if (resizeObserverRef.current) {
                resizeObserverRef.current.disconnect();
            }

            setTimeout(() => {
                isAutoScrolling.current = false;
            }, 100);
        });

        const lastMessageElement = container.lastElementChild;
        if (lastMessageElement) {
            resizeObserverRef.current.observe(lastMessageElement);
        }
    }, []); // 依赖项为空，函数是稳定的
    
    // 选中文本相关状态和逻辑
    const [selectedText, setSelectedText] = useState<string>("");
    
    // 文件管理相关状态和逻辑
    const {
        fileInfoList,
        clearFileInfoList,
        handleChooseFile,
        handleDeleteFile,
        handlePaste,
    } = useFileManagement();
    
    // 文件拖拽相关状态
    const { isDragging, setIsDragging, dropRef } = useFileDropHandler(handleChooseFile);
    
    // 输入相关状态
    const [inputText, setInputText] = useState("");

    // 对话标题管理相关状态
    const [formDialogIsOpen, setFormDialogIsOpen] = useState<boolean>(false);
    const [formConversationTitle, setFormConversationTitle] = useState<string>("");
    const [isRegeneratingTitle, setIsRegeneratingTitle] = useState<boolean>(false);

    // ============= 助手运行时API接口实现 =============
    
    // 助手运行时API接口，提供给插件在运行时使用
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
                    }

                    // 处理流式事件的统一函数
                    const handleConversationEvent = (event: any) => {
                        const conversationEvent = event.payload as ConversationEvent;
                        
                        if (conversationEvent.type === 'message_update') {
                            const messageUpdateData = conversationEvent.data as MessageUpdateEvent;
                            
                            const streamEvent: StreamEvent = {
                                message_id: messageUpdateData.message_id,
                                message_type: messageUpdateData.message_type as any,
                                content: messageUpdateData.content,
                                is_done: messageUpdateData.is_done,
                            };
                            
                            // 处理插件兼容性
                            const streamMessageListener = functionMap.get(
                                res.add_message_id,
                            )?.onStreamMessageListener;
                            if (streamMessageListener) {
                                streamMessageListener(
                                    messageUpdateData.content,
                                    res,
                                    setAiIsResponsing,
                                );
                            }
                            
                            if (messageUpdateData.is_done) {
                                if (messageUpdateData.message_type === 'response') {
                                    setAiIsResponsing(false);
                                }
                            } else {
                                setStreamingMessages((prev) => {
                                    const newMap = new Map(prev);
                                    newMap.set(streamEvent.message_id, streamEvent);
                                    return newMap;
                                });
                                smartScroll();
                            }
                        }
                    };
                    
                    unsubscribeRef.current = listen(`conversation_event_${res.conversation_id}`, handleConversationEvent);
                    
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

    // ============= 初始化逻辑 =============
    
    // 初始化助手类型插件
    useEffect(() => {
        pluginList
            .filter((plugin: any) =>
                plugin.pluginType.includes("assistantType"),
            )
            .forEach((plugin: any) => {
                plugin.instance?.onAssistantTypeInit(assistantTypeApi);
            });
    }, [pluginList]);

    // 当消息变化时自动滚动到底部
    useEffect(() => {
        smartScroll();

        // 返回一个清理函数，在组件卸载或依赖变化时，清理最后的观察器
        return () => {
            if (resizeObserverRef.current) {
                resizeObserverRef.current.disconnect();
                resizeObserverRef.current = null;
            }
        };
    }, [messages, streamingMessages, smartScroll]); // smartScroll 是稳定的，但按规则写入依赖

    // 获取选中文本
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

    // 对话加载和管理逻辑
    useEffect(() => {
        if (!conversationId) {
            // 无对话 ID时，清理状态并加载助手列表
            setMessages([]);
            setConversation(undefined);
            setStreamingMessages(new Map());

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
        
        // 加载指定对话的消息和信息
        setIsLoadingShow(true);
        setStreamingMessages(new Map()); // 切换对话时清理流式消息状态
        console.log(`conversationId change : ${conversationId}`);
        
        invoke<Array<any>>("get_conversation_with_messages", {
            conversationId: +conversationId,
        }).then((res: any[]) => {
            setMessages(res[1]);
            setConversation(res[0]);
            setIsLoadingShow(false);

            console.log(res);

            // 取消之前的事件监听
            if (unsubscribeRef.current) {
                console.log("Unsubscribing from previous event listener");
                unsubscribeRef.current.then((f) => f());
            }

            const lastMessageId = res[1][res[1].length - 1].id;
            setMessageId(lastMessageId);
            
            // 为已存在的对话设置事件监听器
            const handleConversationEvent = (event: any) => {
                const conversationEvent = event.payload as ConversationEvent;
                
                if (conversationEvent.type === 'message_update') {
                    const messageUpdateData = conversationEvent.data as MessageUpdateEvent;
                    
                    const streamEvent: StreamEvent = {
                        message_id: messageUpdateData.message_id,
                        message_type: messageUpdateData.message_type as any,
                        content: messageUpdateData.content,
                        is_done: messageUpdateData.is_done,
                    };
                    
                    // 处理插件兼容性
                    const streamMessageListener = functionMap.get(
                        lastMessageId,
                    )?.onStreamMessageListener;
                    if (streamMessageListener) {
                        streamMessageListener(
                            messageUpdateData.content,
                            {
                                conversation_id: +conversationId,
                                add_message_id: lastMessageId,
                                request_prompt_result_with_context: "",
                            },
                            setAiIsResponsing,
                        );
                    }
                    
                    if (messageUpdateData.is_done) {
                        if (messageUpdateData.message_type === 'response') {
                            setAiIsResponsing(false);
                        }
                    } else {
                        setStreamingMessages((prev) => {
                            const newMap = new Map(prev);
                            newMap.set(streamEvent.message_id, streamEvent);
                            return newMap;
                        });
                        smartScroll();
                    }
                }
            };
            
            unsubscribeRef.current = listen(
                `conversation_event_${conversationId}`,
                handleConversationEvent
            );
        });

        return () => {
            if (unsubscribeRef.current) {
                console.log("unsubscribe");
                unsubscribeRef.current.then((f) => f());
            }
        };
    }, [conversationId]);

    // 监听对话标题变化
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

    // 监听标题变化，同步到表单
    useEffect(() => {
        if (formDialogIsOpen && conversation?.name) {
            setFormConversationTitle(conversation.name);
        }
    }, [formDialogIsOpen, conversation?.name]);

    // ============= Reasoning 展开状态管理 =============
    
    // 管理每个 reasoning 消息的展开状态
    const [reasoningExpandStates, setReasoningExpandStates] = useState<Map<number, boolean>>(new Map());
    
    // 切换 reasoning 消息的展开状态
    const toggleReasoningExpand = useCallback((messageId: number) => {
        setReasoningExpandStates(prev => {
            const newMap = new Map(prev);
            newMap.set(messageId, !newMap.get(messageId));
            return newMap;
        });
    }, []);

    // ============= 业务逻辑处理函数 =============
    
    // 对话管理相关操作
    const { deleteConversation } = useConversationManager();
    const handleDeleteConversation = useCallback(() => {
        deleteConversation(conversationId, {
            onSuccess: () => {
                onChangeConversationId("");
            },
        });
    }, [conversationId, deleteConversation, onChangeConversationId]);

    // 代码运行处理
    const handleArtifact = useCallback((lang: string, inputStr: string) => {
        invoke("run_artifacts", { lang, inputStr })
            .then((res) => {
                console.log(res);
            })
            .catch((error) => {
                toast.error("运行失败: " + JSON.stringify(error));
            });
    }, []);

    // 打开表单对话框
    const openFormDialog = useCallback(() => {
        setFormConversationTitle(conversation?.name || "");
        setFormDialogIsOpen(true);
    }, [conversation]);
    
    // 关闭表单对话框
    const closeFormDialog = useCallback(() => {
        setFormDialogIsOpen(false);
    }, []);
    
    // 提交表单处理
    const handleFormSubmit = useCallback(() => {
        invoke("update_conversation", {
            conversationId: conversation?.id,
            name: formConversationTitle,
        }).then(() => {
            closeFormDialog();
        });
    }, [conversation, formConversationTitle]);

    // 重新生成标题处理
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

    // 消息重新生成处理
    const handleMessageRegenerate = useCallback(
        (regenerateMessageId: number) => {
            invoke<AiResponse>("regenerate_ai", {
                messageId: regenerateMessageId,
            }).then((res) => {
                console.log("regenerate ai response", res);
                // 重新生成消息的处理逻辑
                setMessageId(res.add_message_id);
            });
        },
        [],
    );
    
    // 发送消息的主要处理函数，使用节流防止频繁点击
    const handleSend = throttle(() => {
        if (aiIsResponsing) {
            // AI正在响应时，点击取消
            console.log("Cancelling AI");
            console.log(messageId);
            invoke("cancel_ai", { messageId }).then(() => {
                setAiIsResponsing(false);
            });
        } else {
            // 正常发送消息流程
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

            // 检查是否使用插件助手
            const assistantData = assistants.find((a) => a.id === +assistantId);
            if (assistantData?.assistant_type !== 0) {
                // 使用插件助手
                assistantTypePluginMap
                    .get(assistantData?.assistant_type ?? 0)
                    ?.onAssistantTypeRun(assistantRunApi);
            } else {
                // 使用标准AI助手 - 创建用户消息并发送请求
                const userMessage = {
                    id: 0,
                    conversation_id: conversationId ? +conversationId : -1,
                    llm_model_id: -1,
                    content: inputText,
                    token_count: 0,
                    message_type: "user",
                    created_time: new Date(),
                    start_time: null,
                    finish_time: null,
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
                    
                    // 取消之前的事件监听
                    if (unsubscribeRef.current) {
                        console.log("Unsubscribing from previous event listener");
                        unsubscribeRef.current.then((f) => f());
                    }

                    setMessageId(res.add_message_id);

                    // 更新用户消息内容（后端处理后的版本）
                    setMessages((prevMessages) => {
                        const newMessages = [...prevMessages];
                        const index = prevMessages.findIndex(
                            (msg) => msg == userMessage,
                        );
                        if (index !== -1) {
                            newMessages[index] = {
                                ...newMessages[index],
                                content: res.request_prompt_result_with_context,
                            };
                        }
                        return newMessages;
                    });

                    // 如果是新对话，更新对话 ID
                    if (conversationId != res.conversation_id + "") {
                        onChangeConversationId(res.conversation_id + "");
                    }

                    // 处理对话事件
                    const handleConversationEvent = (event: any) => {
                        const conversationEvent = event.payload as ConversationEvent;
                        
                        if (conversationEvent.type === 'message_update') {
                            const messageUpdateData = conversationEvent.data as MessageUpdateEvent;
                            
                            const streamEvent: StreamEvent = {
                                message_id: messageUpdateData.message_id,
                                message_type: messageUpdateData.message_type as any,
                                content: messageUpdateData.content,
                                is_done: messageUpdateData.is_done,
                            };
                            
                            if (messageUpdateData.is_done) {
                                setAiIsResponsing(false);
                            } else {
                                setStreamingMessages((prev) => {
                                    const newMap = new Map(prev);
                                    newMap.set(streamEvent.message_id, streamEvent);
                                    return newMap;
                                });
                                smartScroll();
                            }
                        }
                    };

                    unsubscribeRef.current = listen(`conversation_event_${res.conversation_id}`, handleConversationEvent);
                }).catch((error) => {
                    toast.error("发送消息失败: " + error);
                });
            }

            setInputText("");
            clearFileInfoList();
        }
    }, 200);

    // ============= 数据计算和处理 =============

    // 合并常规消息和流式消息，按时间排序显示
    const allDisplayMessages = useMemo(() => {
        const combinedMessages = [...messages];
        
        // 将流式消息添加到显示列表中
        streamingMessages.forEach((streamEvent) => {
            // 检查是否已经存在同样ID的消息
            const existingIndex = combinedMessages.findIndex(msg => msg.id === streamEvent.message_id);
            if (existingIndex === -1) {
                // 推断合理的时间戳：基于最后一条消息的时间稍微往后一点
                const lastMessage = combinedMessages[combinedMessages.length - 1];
                const baseTime = lastMessage ? new Date(lastMessage.created_time) : new Date();
                const tempMessage: Message = {
                    id: streamEvent.message_id,
                    conversation_id: conversation?.id || 0,
                    message_type: streamEvent.message_type,
                    content: streamEvent.content,
                    llm_model_id: null,
                    created_time: new Date(baseTime.getTime() + 1000), // 基于最后消息时间+1秒
                    start_time: streamEvent.message_type === 'reasoning' ? baseTime : null,
                    finish_time: streamEvent.is_done ? (streamEvent.end_time || new Date()) : null,
                    token_count: 0,
                    regenerate: null,
                };
                combinedMessages.push(tempMessage);
            } else {
                // 存在则更新消息内容
                combinedMessages[existingIndex] = {
                    ...combinedMessages[existingIndex],
                    content: streamEvent.content,
                    message_type: streamEvent.message_type, // 确保消息类型也被更新
                    finish_time: streamEvent.is_done ? (streamEvent.end_time || new Date()) : combinedMessages[existingIndex].finish_time,
                };
            }
        });

        const sorted = combinedMessages.sort((a, b) => new Date(a.created_time).getTime() - new Date(b.created_time).getTime());
        return sorted;
    }, [messages, streamingMessages, conversation?.id]);

    // 过滤系统消息并渲染MessageItem组件
    const filteredMessages = useMemo(
        () =>
            allDisplayMessages
                .filter((m) => m.message_type !== "system")
                .map((message) => {
                    // 查找对应的流式消息信息（如果存在）
                    const streamEvent = streamingMessages.get(message.id);
                    
                    return (
                        <MessageItem
                            key={message.id} // 使用唯一的 id 作为 key，而不是索引
                            message={message}
                            streamEvent={streamEvent} // 传递流式消息信息
                            onCodeRun={handleArtifact}
                            onMessageRegenerate={() =>
                                handleMessageRegenerate(message.id)
                            }
                            // Reasoning 展开状态相关 props
                            isReasoningExpanded={reasoningExpandStates.get(message.id) || false}
                            onToggleReasoningExpand={() => toggleReasoningExpand(message.id)}
                        />
                    );
                }),
        [allDisplayMessages, streamingMessages, reasoningExpandStates, toggleReasoningExpand],
    );

    // ============= 组件渲染 =============
    
    return (
        <div ref={dropRef} className="h-full relative flex flex-col bg-white rounded-xl">
            {conversationId ? (
                <ConversationTitle
                    onEdit={openFormDialog}
                    onDelete={handleDeleteConversation}
                    conversation={conversation}
                />
            ) : null}

            <div 
                ref={scrollContainerRef}
                onScroll={handleScroll}
                className="h-full flex-1 overflow-y-auto flex flex-col p-6 box-border gap-4"
            >
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