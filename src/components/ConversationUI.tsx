import { invoke } from "@tauri-apps/api/core";
import React, {
    useCallback,
    useEffect,
    useMemo,
    useRef,
    useState,
} from "react";
import { toast } from "sonner";

import {
    Conversation,
    FileInfo,
    Message,
    StreamEvent,
    ConversationWithMessages,
    GroupMergeEvent,
} from "../data/Conversation";
import "katex/dist/katex.min.css";
import { listen } from "@tauri-apps/api/event";
import { throttle } from "lodash";
import NewChatComponent from "./NewChatComponent";
import FileDropArea from "./FileDropArea";
import MessageItem from "./MessageItem";
import VersionPagination from "./VersionPagination";
import ConversationTitle from "./conversation/ConversationTitle";
import useFileDropHandler from "../hooks/useFileDropHandler";
import InputArea from "./conversation/InputArea";
import MessageEditDialog from "./MessageEditDialog";
import ConversationTitleEditDialog from "./ConversationTitleEditDialog";
import useConversationManager from "../hooks/useConversationManager";
import { useMessageGroups } from "../hooks/useMessageGroups";
import useFileManagement from "@/hooks/useFileManagement";
import { useConversationEvents } from "@/hooks/useConversationEvents";

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
        markdownRemarkRegist: (_: any) => { },
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

    // AI响应状态管理
    const [aiIsResponsing, setAiIsResponsing] = useState<boolean>(false);

    // 使用 useCallback 确保回调函数稳定
    const handleMessageAdd = useCallback((messageAddData: any) => {
        // 设置函数映射
        setFunctionMap((prev) => {
            const newMap = new Map(prev);
            newMap.set(messageAddData.message_id, {
                onCustomUserMessage: undefined,
                onCustomUserMessageComing: undefined,
                onStreamMessageListener: undefined,
            });
            return newMap;
        });

        // 重新获取对话消息，以确保获得完整的消息数据（包括generation_group_id等）
        invoke<ConversationWithMessages>(
            "get_conversation_with_messages",
            {
                conversationId: +conversationId,
            },
        )
            .then((updatedConversation) => {
                setMessages(updatedConversation.messages);
                console.log(
                    "Updated messages after message_add:",
                    updatedConversation.messages,
                );
            })
            .catch((error) => {
                console.error(
                    "Failed to reload conversation after message_add:",
                    error,
                );

                // 降级处理：仍然添加基本的消息信息
                const newMessage: Message = {
                    id: messageAddData.message_id,
                    conversation_id: +conversationId,
                    message_type: messageAddData.message_type,
                    content: "", // 初始内容为空，会通过后续的message_update事件更新
                    llm_model_id: null,
                    created_time: new Date(),
                    start_time: new Date(),
                    finish_time: null,
                    token_count: 0,
                    generation_group_id: null, // 这些字段会在数据库查询时填充
                    parent_group_id: null,
                    regenerate: null,
                };

                setMessages((prevMessages) => [
                    ...prevMessages,
                    newMessage,
                ]);
            });
    }, [conversationId]);

    // handleMessageUpdate 将在其依赖的函数之后定义

    const handleGroupMerge = useCallback((groupMergeData: GroupMergeEvent) => {
        // 设置组合并关系
        setGroupMergeMap((prev) => {
            const newMap = new Map(prev);
            newMap.set(
                groupMergeData.new_group_id,
                groupMergeData.original_group_id,
            );
            console.log(
                "Updated groupMergeMap:",
                Array.from(newMap.entries()),
            );
            return newMap;
        });
    }, []);

    const handleAiResponseComplete = useCallback(() => {
        setAiIsResponsing(false);
    }, []);

    const handleError = useCallback((errorMessage: string) => {
        console.error("Stream error from conversation events:", errorMessage);
        // 错误处理在useConversationEvents中的错误处理逻辑和直接的error notification listener中都会处理
        // 这里主要是为了确保响应状态被正确重置
        setAiIsResponsing(false);
    }, []);

    // handleMessageUpdate 将在后面定义，这里先声明一个空的引用
    let handleMessageUpdateRef: ((streamEvent: StreamEvent) => void) | undefined;

    // 使用 useMemo 稳定 options 对象，避免频繁触发 useConversationEvents 内部的 useEffect
    const conversationEventsOptions = useMemo(
        () => ({
            conversationId: conversationId,
            onMessageAdd: handleMessageAdd,
            onMessageUpdate: (streamEvent: StreamEvent) => handleMessageUpdateRef?.(streamEvent),
            onGroupMerge: handleGroupMerge,
            onAiResponseComplete: handleAiResponseComplete,
            onError: handleError,
        }),
        [conversationId, handleMessageAdd, handleGroupMerge, handleAiResponseComplete, handleError]
    );

    // 使用共享的消息事件处理 hook
    const {
        streamingMessages,
        shiningMessageIds,
        setShiningMessageIds,
        clearShiningMessages,
    } = useConversationEvents(conversationEventsOptions);

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
            scrollContainerRef.current.scrollTop =
                scrollContainerRef.current.scrollHeight;

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
    const { isDragging, setIsDragging, dropRef } =
        useFileDropHandler(handleChooseFile);

    // 输入相关状态
    const [inputText, setInputText] = useState("");

    // 对话标题管理相关状态
    const [titleEditDialogIsOpen, setTitleEditDialogIsOpen] =
        useState<boolean>(false);

    // 消息编辑相关状态
    const [editDialogIsOpen, setEditDialogIsOpen] = useState<boolean>(false);
    const [editingMessage, setEditingMessage] = useState<Message | null>(null);


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
            _onCustomUserMessageComing?: (_: AiResponse) => void,
            _onStreamMessageListener?: (
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

                    if (conversationId != res.conversation_id + "") {
                        onChangeConversationId(res.conversation_id + "");
                    }

                    // 事件处理现在由共享的 useConversationEvents hook 处理

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
        setGroupMergeMap(new Map()); // 切换对话时清理组合并状态

        console.log(`conversationId change : ${conversationId}`);

        invoke<ConversationWithMessages>("get_conversation_with_messages", {
            conversationId: +conversationId,
        }).then((res: ConversationWithMessages) => {
            setMessages(res.messages);
            setConversation(res.conversation);
            setIsLoadingShow(false);

            if (res.messages.length === 2) {
                if (res.messages[0].message_type === "system" && res.messages[1].message_type === "user") {
                    setShiningMessageIds(new Set([...shiningMessageIds, res.messages[1].id]));
                }
            }
        });
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

    // 监听错误通知事件
    useEffect(() => {
        const unsubscribe = listen("conversation-window-error-notification", (event) => {
            const errorMessage = event.payload as string;
            console.error("Received error notification:", errorMessage);
            
            // 显示错误通知
            toast.error(`AI请求失败: ${errorMessage}`);
            
            // 重置AI响应状态
            setAiIsResponsing(false);
            
            // 清除闪烁状态
            clearShiningMessages();
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, [clearShiningMessages]);

    // ============= 数据计算和处理 =============

    // 首先合并常规消息和流式消息（不排序）
    const combinedMessagesForGrouping = useMemo(() => {
        const combinedMessages = [...messages];

        // 将流式消息添加到显示列表中
        streamingMessages.forEach((streamEvent) => {
            // 检查是否已经存在同样ID的消息
            const existingIndex = combinedMessages.findIndex(
                (msg) => msg.id === streamEvent.message_id,
            );
            if (existingIndex === -1) {
                // 推断合理的时间戳：基于最后一条消息的时间稍微往后一点
                const lastMessage =
                    combinedMessages[combinedMessages.length - 1];
                const baseTime = lastMessage
                    ? new Date(lastMessage.created_time)
                    : new Date();
                const tempMessage: Message = {
                    id: streamEvent.message_id,
                    conversation_id: conversation?.id || 0,
                    message_type: streamEvent.message_type,
                    content: streamEvent.content,
                    llm_model_id: null,
                    created_time: new Date(baseTime.getTime() + 1000), // 基于最后消息时间+1秒
                    start_time:
                        streamEvent.message_type === "reasoning"
                            ? baseTime
                            : null,
                    finish_time: streamEvent.is_done
                        ? streamEvent.end_time || new Date()
                        : null,
                    token_count: 0,
                    generation_group_id: null, // 流式消息暂时不设置generation_group_id
                    parent_group_id: null, // 流式消息暂时不设置parent_group_id
                    regenerate: null,
                };
                combinedMessages.push(tempMessage);
            } else {
                // 存在则更新消息内容
                combinedMessages[existingIndex] = {
                    ...combinedMessages[existingIndex],
                    content: streamEvent.content,
                    message_type: streamEvent.message_type, // 确保消息类型也被更新
                    finish_time: streamEvent.is_done
                        ? streamEvent.end_time || new Date()
                        : combinedMessages[existingIndex].finish_time,
                };
            }
        });

        return combinedMessages;
    }, [messages, streamingMessages, conversation?.id]);

    // ============= Generation Group 版本管理 =============

    // 管理组合并关系：new_group_id -> original_group_id
    const [groupMergeMap, setGroupMergeMap] = useState<Map<string, string>>(
        new Map(),
    );

    // 使用自定义钩子获取所有分组相关的数据和逻辑
    const {
        generationGroups,
        selectedVersions,
        messageIdToGroupRootTimestamp,
        handleGenerationVersionChange,
        getMessageVersionInfo,
        getGenerationGroupControl,
    } = useMessageGroups({ allDisplayMessages: combinedMessagesForGrouping, groupMergeMap });

    // 最后进行排序，使用从 useMessageGroups 获取的时间戳映射
    const allDisplayMessages = useMemo(() => {
        // 计算每个消息的排序基准时间：使用 O(1) Map 查找替代 O(N) 遍历
        const getMessageSortTime = (message: Message): number => {
            // 尝试从 messageIdToGroupRootTimestamp 中获取根时间戳
            const rootTimestamp = messageIdToGroupRootTimestamp.get(message.id);
            if (rootTimestamp !== undefined) {
                return rootTimestamp;
            }
            
            // 对于没有分组的消息，使用自身创建时间
            return new Date(message.created_time).getTime();
        };

        const sorted = [...combinedMessagesForGrouping].sort(
            (a, b) => getMessageSortTime(a) - getMessageSortTime(b)
        );
        return sorted;
    }, [combinedMessagesForGrouping, messageIdToGroupRootTimestamp]);

    // ============= Reasoning 展开状态管理 =============

    // 管理每个 reasoning 消息的展开状态
    const [reasoningExpandStates, setReasoningExpandStates] = useState<
        Map<number, boolean>
    >(new Map());

    // 切换 reasoning 消息的展开状态
    const toggleReasoningExpand = useCallback((messageId: number) => {
        setReasoningExpandStates((prev) => {
            const newMap = new Map(prev);
            newMap.set(messageId, !newMap.get(messageId));
            return newMap;
        });
    }, []);

    // ============= 消息状态管理辅助函数 =============

    // 处理消息完成时的状态更新，确保消息在streamingMessages清理后仍能显示
    const handleMessageCompletion = useCallback(
        (streamEvent: StreamEvent) => {
            // 检查messages中是否已存在该消息
            setMessages((prevMessages) => {
                const existingIndex = prevMessages.findIndex(
                    (msg) => msg.id === streamEvent.message_id,
                );

                if (existingIndex !== -1) {
                    // 消息已存在，更新其内容和完成状态
                    const updatedMessages = [...prevMessages];
                    updatedMessages[existingIndex] = {
                        ...updatedMessages[existingIndex],
                        content: streamEvent.content,
                        message_type: streamEvent.message_type,
                        finish_time: new Date(), // 标记为完成
                    };
                    return updatedMessages;
                } else {
                    // 消息不存在，添加新消息
                    const lastMessage = prevMessages[prevMessages.length - 1];
                    const baseTime = lastMessage
                        ? new Date(lastMessage.created_time)
                        : new Date();
                    const newMessage: Message = {
                        id: streamEvent.message_id,
                        conversation_id: conversation?.id || 0,
                        message_type: streamEvent.message_type,
                        content: streamEvent.content,
                        llm_model_id: null,
                        created_time: new Date(baseTime.getTime() + 1000),
                        start_time:
                            streamEvent.message_type === "reasoning"
                                ? baseTime
                                : null,
                        finish_time: new Date(), // 标记为完成
                        token_count: 0,
                        generation_group_id: null, // 流式消息暂时不设置generation_group_id
                        parent_group_id: null, // 流式消息暂时不设置parent_group_id
                        regenerate: null,
                    };
                    return [...prevMessages, newMessage];
                }
            });
        },
        [conversation?.id],
    );

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

    // 消息更新处理回调函数
    const handleMessageUpdate = useCallback((streamEvent: StreamEvent) => {
        // 处理插件兼容性
        const streamMessageListener = functionMap.get(
            streamEvent.message_id,
        )?.onStreamMessageListener;
        if (streamMessageListener) {
            streamMessageListener(
                streamEvent.content,
                { conversation_id: +conversationId, request_prompt_result_with_context: "" },
                setAiIsResponsing,
            );
        }

        if (streamEvent.is_done) {
            // 在清理streamingMessages之前，先将消息添加到messages状态
            handleMessageCompletion(streamEvent);
        }

        smartScroll();
    }, [conversationId, functionMap, handleMessageCompletion, smartScroll]);

    // 设置引用
    handleMessageUpdateRef = handleMessageUpdate;

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

    // 打开标题编辑对话框
    const openTitleEditDialog = useCallback(() => {
        setTitleEditDialogIsOpen(true);
    }, []);

    // 关闭标题编辑对话框
    const closeTitleEditDialog = useCallback(() => {
        setTitleEditDialogIsOpen(false);
    }, []);

    // 消息重新生成处理
    const handleMessageRegenerate = useCallback(
        (regenerateMessageId: number) => {
            // 设置AI响应状态
            setAiIsResponsing(true);

            // 设置被点击的消息显示shine-border
            setShiningMessageIds(new Set([regenerateMessageId]));

            invoke<AiResponse>("regenerate_ai", {
                messageId: regenerateMessageId,
            })
                .then((res) => {
                    console.log("regenerate ai response", res);
                    // 重新生成消息的处理逻辑
                    // setMessageId(res.add_message_id);
                })
                .catch((error) => {
                    console.error("Regenerate error:", error);
                    setAiIsResponsing(false);
                    // 错误时清除shine-border
                    clearShiningMessages();
                    toast.error("重新生成失败: " + error);
                });
        },
        [],
    );

    // 消息编辑相关处理函数
    const handleMessageEdit = useCallback((message: Message) => {
        setEditingMessage(message);
        setEditDialogIsOpen(true);
    }, []);

    const closeEditDialog = useCallback(() => {
        setEditDialogIsOpen(false);
        setEditingMessage(null);
    }, []);

    const handleEditSave = useCallback(
        (content: string) => {
            if (!editingMessage) return;

            invoke("update_message_content", {
                messageId: editingMessage.id,
                content: content,
            })
                .then(() => {
                    // 更新本地消息状态
                    setMessages((prevMessages) =>
                        prevMessages.map((msg) =>
                            msg.id === editingMessage.id
                                ? { ...msg, content: content }
                                : msg,
                        ),
                    );
                    toast.success("消息已更新");
                })
                .catch((error) => {
                    toast.error("更新消息失败: " + error);
                });
        },
        [editingMessage],
    );

    const handleEditSaveAndRegenerate = useCallback(
        (content: string) => {
            if (!editingMessage) return;

            // 先更新消息内容
            invoke("update_message_content", {
                messageId: editingMessage.id,
                content: content,
            })
                .then(() => {
                    // 更新本地消息状态
                    setMessages((prevMessages) =>
                        prevMessages.map((msg) =>
                            msg.id === editingMessage.id
                                ? { ...msg, content: content }
                                : msg,
                        ),
                    );

                    // 然后触发重新生成
                    handleMessageRegenerate(editingMessage.id);

                    toast.success("消息已更新并开始重新生成");
                })
                .catch((error) => {
                    toast.error("更新消息失败: " + error);
                });
        },
        [editingMessage, handleMessageRegenerate],
    );

    // 发送消息的主要处理函数，使用节流防止频繁点击
    const handleSend = throttle(() => {
        if (aiIsResponsing) {
            // AI正在响应时，点击取消
            console.log("Cancelling AI");
            console.log(conversationId);
            invoke("cancel_ai", { conversationId: +conversationId }).then(() => {
                setAiIsResponsing(false);
                // shine-border 状态现在由 useConversationEvents hook 管理
                clearShiningMessages();
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
                invoke<AiResponse>("ask_ai", {
                    request: {
                        prompt: inputText,
                        conversation_id: conversationId,
                        assistant_id: +assistantId,
                        attachment_list: fileInfoList?.map((i) => i.id),
                    },
                })
                    .then((res) => {
                        console.log("ask ai response", res);

                        // 如果是新对话，更新对话 ID
                        if (conversationId != res.conversation_id + "") {
                            onChangeConversationId(res.conversation_id + "");
                        }
                    })
                    .catch((error) => {
                        setAiIsResponsing(false);
                        // 发送消息失败时清除shine-border
                        clearShiningMessages();
                        toast.error("发送消息失败: " + error);
                    });
            }

            setInputText("");
            clearFileInfoList();
        }
    }, 200);

    // 过滤系统消息并渲染MessageItem组件
    const filteredMessages = useMemo(() => {
        const result = allDisplayMessages
            .filter((m) => m.message_type !== "system")
            .map((message) => {
                // 查找对应的流式消息信息（如果存在）
                const streamEvent = streamingMessages.get(message.id);

                // 检查是否需要根据版本管理隐藏消息
                const versionInfo = getMessageVersionInfo(message);
                if (versionInfo && !versionInfo.shouldShow) {
                    return null; // 不显示非当前版本的消息
                }

                // 检查是否需要显示版本控制
                const groupControl = getGenerationGroupControl(message);

                // 检查是否需要显示shine-border
                const shouldShowShineBorder = shiningMessageIds.has(message.id);

                return (
                    <React.Fragment key={message.id}>
                        <MessageItem
                            message={message}
                            streamEvent={streamEvent}
                            onCodeRun={handleArtifact}
                            onMessageRegenerate={() =>
                                handleMessageRegenerate(message.id)
                            }
                            onMessageEdit={() => handleMessageEdit(message)}
                            // Reasoning 展开状态相关 props
                            isReasoningExpanded={
                                reasoningExpandStates.get(message.id) || false
                            }
                            onToggleReasoningExpand={() =>
                                toggleReasoningExpand(message.id)
                            }
                            // ShineBorder 动画状态
                            shouldShowShineBorder={shouldShowShineBorder}
                            // MCP 工具调用需要的上下文信息
                            conversationId={message.conversation_id}
                        />
                        {/* 在 generation group 的最后一个消息下方显示版本控制 */}
                        {groupControl && (
                            <div className="flex justify-start mt-2">
                                <VersionPagination
                                    currentVersion={groupControl.currentVersion}
                                    totalVersions={groupControl.totalVersions}
                                    onVersionChange={(versionIndex) =>
                                        handleGenerationVersionChange(
                                            groupControl.groupId,
                                            versionIndex,
                                        )
                                    }
                                />
                            </div>
                        )}
                    </React.Fragment>
                );
            })
            .filter(Boolean); // 过滤掉 null 值

        // 添加占位符消息渲染
        generationGroups.forEach((group, groupId) => {
            const selectedVersionIndex =
                selectedVersions.get(groupId) ??
                (group.versions.length > 0 ? group.versions.length - 1 : 0);
            const selectedVersionData = group.versions[selectedVersionIndex];

            // 如果选中的是占位符版本，添加占位符消息
            if (selectedVersionData?.isPlaceholder) {
                // 找到这个组的最后一个消息的位置，在其后添加占位符
                const groupMessages = group.versions
                    .flatMap((version) => version.messages)
                    .filter((msg: any) => msg.message_type !== "system");
                if (groupMessages.length > 0) {
                    const lastMessage = groupMessages[groupMessages.length - 1];
                    const lastMessageIndex = result.findIndex(
                        (item) => item?.key === lastMessage.id.toString(),
                    );

                    if (lastMessageIndex !== -1) {
                        // 在最后一个消息后添加占位符
                        const placeholderElement = (
                            <React.Fragment key={`placeholder_${groupId}`}>
                                <div className="flex justify-start mb-4">
                                    <div className="bg-gray-100 rounded-lg p-4 max-w-3xl">
                                        <div className="flex items-center space-x-2">
                                            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-900"></div>
                                            <span className="text-sm text-gray-600">
                                                正在重新生成...
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <div className="flex justify-start mt-2">
                                    <VersionPagination
                                        currentVersion={
                                            selectedVersionIndex + 1
                                        }
                                        totalVersions={group.versions.length}
                                        onVersionChange={(versionIndex) =>
                                            handleGenerationVersionChange(
                                                groupId,
                                                versionIndex,
                                            )
                                        }
                                    />
                                </div>
                            </React.Fragment>
                        );
                        result.splice(
                            lastMessageIndex + 1,
                            0,
                            placeholderElement,
                        );
                    }
                }
            }
        });

        return result;
    }, [
        allDisplayMessages,
        streamingMessages,
        getMessageVersionInfo,
        getGenerationGroupControl,
        handleGenerationVersionChange,
        reasoningExpandStates,
        toggleReasoningExpand,
        generationGroups,
        selectedVersions,
        shiningMessageIds,
    ]);

    // ============= 组件渲染 =============

    return (
        <div
            ref={dropRef}
            className="h-full relative flex flex-col bg-white rounded-xl"
        >
            {conversationId ? (
                <ConversationTitle
                    onEdit={openTitleEditDialog}
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

            <ConversationTitleEditDialog
                isOpen={titleEditDialogIsOpen}
                conversationId={conversation?.id || 0}
                initialTitle={conversation?.name || ""}
                onClose={closeTitleEditDialog}
            />

            <MessageEditDialog
                isOpen={editDialogIsOpen}
                initialContent={editingMessage?.content || ""}
                messageType={editingMessage?.message_type || ""}
                onClose={closeEditDialog}
                onSave={handleEditSave}
                onSaveAndRegenerate={handleEditSaveAndRegenerate}
            />

            {isLoadingShow ? (
                <div className="bg-white/95 w-full h-full absolute flex items-center justify-center backdrop-blur rounded-xl">
                    <div className="loading-icon"></div>
                    <div className="text-indigo-500 text-base font-medium">
                        加载中...
                    </div>
                </div>
            ) : null}
        </div>
    );
}

export default ConversationUI;
