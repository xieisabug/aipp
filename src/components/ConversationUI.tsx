import { invoke } from "@tauri-apps/api/core";
import {
    useCallback,
    useEffect,
    useMemo,
    useRef,
    useState,
    forwardRef,
    useImperativeHandle,
    useLayoutEffect,
} from "react";

import {
    Conversation,
    Message,
    StreamEvent,
    ConversationWithMessages,
    GroupMergeEvent,
    MCPToolCallUpdateEvent,
} from "../data/Conversation";
import "katex/dist/katex.min.css";
import { listen } from "@tauri-apps/api/event";
import FileDropArea from "./FileDropArea";
import useFileDropHandler from "../hooks/useFileDropHandler";
import InputArea, { InputAreaRef } from "./conversation/InputArea";
import MessageEditDialog from "./MessageEditDialog";
import ConversationTitleEditDialog from "./ConversationTitleEditDialog";
import { useMessageGroups } from "../hooks/useMessageGroups";
import useFileManagement from "@/hooks/useFileManagement";
import { useConversationEvents } from "@/hooks/useConversationEvents";
import { useAssistantListListener } from "@/hooks/useAssistantListListener";
import { AssistantListItem } from "@/data/Assistant";

// 导入新创建的 hooks
import { usePluginManagement } from "@/hooks/usePluginManagement";
import { useScrollManagement } from "@/hooks/useScrollManagement";
import { useTextSelection } from "@/hooks/useTextSelection";
import { useAssistantRuntime } from "@/hooks/useAssistantRuntime";
import { useMessageProcessing } from "@/hooks/useMessageProcessing";
import { useReasoningExpand } from "@/hooks/useReasoningExpand";
import { useConversationOperations } from "@/hooks/useConversationOperations";

// 导入新创建的组件
import ConversationHeader from "./conversation/ConversationHeader";
import ConversationContent from "./conversation/ConversationContent";

// 暴露给外部的方法接口
export interface ConversationUIRef {
    focus: () => void;
}

interface ConversationUIProps {
    conversationId: string;
    onChangeConversationId: (conversationId: string) => void;
    pluginList: any[];
}

const ConversationUI = forwardRef<ConversationUIRef, ConversationUIProps>(
    ({ conversationId, onChangeConversationId, pluginList }, ref) => {
        // ============= 基础状态管理 =============

        // 当前对话信息和助手列表
        const [conversation, setConversation] = useState<Conversation>();
        const [assistants, setAssistants] = useState<AssistantListItem[]>([]);
        const [selectedAssistant, setSelectedAssistant] = useState(-1);

        // 对话加载状态
        const [isLoadingShow, setIsLoadingShow] = useState(false);

        // 常规消息列表
        const [messages, setMessages] = useState<Array<Message>>([]);

        // AI响应状态管理
        const [aiIsResponsing, setAiIsResponsing] = useState<boolean>(false);

        // 输入相关状态
        const [inputText, setInputText] = useState("");
        const inputAreaRef = useRef<InputAreaRef>(null);

        // ============= 使用新创建的 hooks =============

        // 插件管理
        const { assistantTypePluginMap, functionMap, setFunctionMapForMessage } = usePluginManagement(pluginList);

        // 文本选择
        const { selectedText } = useTextSelection();

        // 文件管理
        const { fileInfoList, clearFileInfoList, handleChooseFile, handleDeleteFile, handlePaste } =
            useFileManagement();

        // 文件拖拽
        const { isDragging, setIsDragging, dropRef } = useFileDropHandler(handleChooseFile);

        // Reasoning 展开状态
        const { reasoningExpandStates, toggleReasoningExpand } = useReasoningExpand();

        // ============= 事件处理逻辑 =============

        const handleMessageAdd = useCallback(
            (messageAddData: any) => {
                // 设置函数映射
                setFunctionMapForMessage(messageAddData.message_id);

                // 重新获取对话消息，以确保获得完整的消息数据（包括generation_group_id等）
                invoke<ConversationWithMessages>("get_conversation_with_messages", {
                    conversationId: +conversationId,
                })
                    .then((updatedConversation) => {
                        setMessages(updatedConversation.messages);
                    })
                    .catch((error) => {
                        console.error("Failed to reload conversation after message_add:", error);

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

                        setMessages((prevMessages) => [...prevMessages, newMessage]);
                    });
            },
            [conversationId, setFunctionMapForMessage]
        );

        const handleGroupMerge = useCallback((groupMergeData: GroupMergeEvent) => {
            // 设置组合并关系
            setGroupMergeMap((prev) => {
                const newMap = new Map(prev);
                newMap.set(groupMergeData.new_group_id, groupMergeData.original_group_id);
                return newMap;
            });
        }, []);

        const handleAiResponseComplete = useCallback(() => {
            setAiIsResponsing(false);
        }, []);

        const handleError = useCallback((errorMessage: string) => {
            console.error("Stream error from conversation events:", errorMessage);
            // 确保AI响应状态被重置
            setAiIsResponsing(false);
            // 不再显示toast，错误信息将在对话框中显示
        }, []);

        const handleMCPToolCallUpdate = useCallback((mcpUpdateData: MCPToolCallUpdateEvent) => {
            console.log("ConversationUI received MCP update:", mcpUpdateData);
            // MCP状态更新已经在useConversationEvents中处理，这里可以添加额外的逻辑
        }, []);

        // ============= 消息处理逻辑 =============

        // 处理消息完成时的状态更新，确保消息在streamingMessages清理后仍能显示
        const handleMessageCompletion = useCallback(
            (streamEvent: StreamEvent) => {
                // 检查messages中是否已存在该消息
                setMessages((prevMessages) => {
                    const existingIndex = prevMessages.findIndex((msg) => msg.id === streamEvent.message_id);

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
                        const baseTime = lastMessage ? new Date(lastMessage.created_time) : new Date();
                        const newMessage: Message = {
                            id: streamEvent.message_id,
                            conversation_id: conversation?.id || 0,
                            message_type: streamEvent.message_type,
                            content: streamEvent.content,
                            llm_model_id: null,
                            created_time: new Date(baseTime.getTime() + 1000),
                            start_time: streamEvent.message_type === "reasoning" ? baseTime : null,
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
            [conversation?.id]
        );

        // 滚动管理 - 移除依赖项，改为手动调用
        const { messagesEndRef, scrollContainerRef, handleScroll, smartScroll } = useScrollManagement();

        // 使用 useMemo 稳定 options 对象，避免频繁触发 useConversationEvents 内部的 useEffect
        const conversationEventsOptions = useMemo(() => {
            const handleMessageUpdate = (streamEvent: StreamEvent) => {
                // 处理插件兼容性 - 现在从 ref 中获取最新的 functionMap
                // 这里需要从 useConversationEvents 内部处理，所以暂时移除
                // const streamMessageListener = functionMap.get(
                //     streamEvent.message_id,
                // )?.onStreamMessageListener;
                // if (streamMessageListener) {
                //     streamMessageListener(
                //         streamEvent.content,
                //         { conversation_id: +conversationId, request_prompt_result_with_context: "" },
                //         setAiIsResponsing,
                //     );
                // }

                if (streamEvent.is_done) {
                    // 在清理streamingMessages之前，先将消息添加到messages状态
                    handleMessageCompletion(streamEvent);
                }

                // 每次消息更新时手动触发滚动
                setTimeout(() => smartScroll(), 0);
            };

            return {
                conversationId: conversationId,
                onMessageAdd: handleMessageAdd,
                onMessageUpdate: handleMessageUpdate,
                onGroupMerge: handleGroupMerge,
                onMCPToolCallUpdate: handleMCPToolCallUpdate,
                onAiResponseComplete: handleAiResponseComplete,
                onError: handleError,
            };
        }, [
            conversationId,
            handleMessageAdd,
            handleGroupMerge,
            handleMCPToolCallUpdate,
            handleAiResponseComplete,
            handleError,
            handleMessageCompletion,
            smartScroll,
            // 移除 functionMap 依赖，改为在回调内部访问
        ]);

        // 使用共享的消息事件处理 hook
        const {
            streamingMessages,
            shiningMessageIds,
            setShiningMessageIds,
            mcpToolCallStates,
            updateShiningMessages,
            updateFunctionMap,
            clearStreamingMessages,
        } = useConversationEvents(conversationEventsOptions);

        // 当 functionMap 变化时更新事件处理器
        useEffect(() => {
            updateFunctionMap(functionMap);
        }, [functionMap, updateFunctionMap]);

        // 消息处理 - 首先需要获取 groupMergeMap
        const [groupMergeMap, setGroupMergeMap] = useState<Map<string, string>>(new Map());

        // 第一步：消息处理 - 获取合并的消息用于分组
        const { combinedMessagesForGrouping } = useMessageProcessing({
            messages,
            streamingMessages,
            conversation,
            generationGroups: new Map(), // 第一步只需要合并消息用于分组
            groupRootMessageIds: new Map(),
            getMessageVersionInfo: () => ({ shouldShow: true }),
        });

        // 第二步：使用合并后的消息进行分组计算
        const messageGroupsData = useMessageGroups({
            allDisplayMessages: combinedMessagesForGrouping,
            groupMergeMap,
        });

        // 第三步：基于分组信息与选择的版本，计算最终需要展示的消息列表
        const { allDisplayMessages } = useMessageProcessing({
            messages,
            streamingMessages,
            conversation,
            generationGroups: messageGroupsData.generationGroups,
            groupRootMessageIds: messageGroupsData.groupRootMessageIds,
            getMessageVersionInfo: messageGroupsData.getMessageVersionInfo,
        });

        // 助手运行时API
        const { assistantRunApi } = useAssistantRuntime({
            conversation,
            selectedAssistant,
            inputText,
            fileInfoList: fileInfoList || undefined,
            setMessages,
            onChangeConversationId,
            smartScroll,
            updateShiningMessages,
            setAiIsResponsing,
        });

        // 对话操作
        const {
            handleDeleteConversationSuccess,
            handleMessageRegenerate,
            handleMessageEdit,
            handleMessageFork,
            handleEditSave,
            handleEditSaveAndRegenerate,
            handleSend,
            handleArtifact,
            editDialogIsOpen,
            editingMessage,
            closeEditDialog,
            titleEditDialogIsOpen,
            openTitleEditDialog,
            closeTitleEditDialog,
        } = useConversationOperations({
            conversation,
            selectedAssistant,
            assistants,
            setMessages,
            inputText,
            setInputText,
            fileInfoList: fileInfoList || undefined,
            clearFileInfoList,
            aiIsResponsing,
            setAiIsResponsing,
            onChangeConversationId,
            setShiningMessageIds,
            updateShiningMessages,
            assistantTypePluginMap,
            assistantRunApi,
        });

        // ============= 初始化和生命周期逻辑 =============

        // 暴露给外部的方法
        useImperativeHandle(
            ref,
            () => ({
                focus: () => {
                    inputAreaRef.current?.focus();
                },
            }),
            []
        );

        // 智能聚焦逻辑 - 无延迟版本
        useLayoutEffect(() => {
            // 只在 InputArea 存在且不在加载状态时聚焦
            if (inputAreaRef.current && !isLoadingShow) {
                inputAreaRef.current.focus();
            }
        }, [conversationId, isLoadingShow]); // 监听对话ID和加载状态变化

        // 对话加载和管理逻辑
        useEffect(() => {
            if (!conversationId) {
                // 无对话 ID时，清理状态并加载助手列表
                setMessages([]);
                setConversation(undefined);
                // 清理流式消息和闪烁状态
                clearStreamingMessages();

                invoke<Array<AssistantListItem>>("get_assistants").then((assistantList) => {
                    setAssistants(assistantList);
                    if (assistantList.length > 0) {
                        setSelectedAssistant(assistantList[0].id);
                    }
                });
                return;
            }

            // 防止重复请求
            const currentLoadingRef = { cancelled: false };

            // 加载指定对话的消息和信息
            setIsLoadingShow(true);
            
            // 在切换对话时立即清理所有与前一个对话相关的状态
            setGroupMergeMap(new Map()); // 切换对话时清理组合并状态
            clearStreamingMessages(); // 清理流式消息

            console.log(`conversationId change : ${conversationId}`);

            invoke<ConversationWithMessages>("get_conversation_with_messages", {
                conversationId: +conversationId,
            })
                .then((res: ConversationWithMessages) => {
                    // 检查请求是否已被取消
                    if (currentLoadingRef.cancelled) {
                        return;
                    }

                    setMessages(res.messages);
                    setConversation(res.conversation);
                    setIsLoadingShow(false); // 这里会触发 useLayoutEffect 中的聚焦

                    if (res.messages.length === 2) {
                        if (res.messages[0].message_type === "system" && res.messages[1].message_type === "user") {
                            setShiningMessageIds((prev) => new Set([...prev, res.messages[1].id]));
                        }
                    }
                })
                .catch((error) => {
                    if (!currentLoadingRef.cancelled) {
                        console.error("Failed to load conversation:", error);
                        setIsLoadingShow(false);
                    }
                });

            // 清理函数，防止组件卸载时的状态更新
            return () => {
                currentLoadingRef.cancelled = true;
            };
        }, [conversationId, clearStreamingMessages]);

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

        // 监听助手列表变化
        useAssistantListListener({
            onAssistantListChanged: useCallback(
                (assistantList: AssistantListItem[]) => {
                    setAssistants(assistantList);
                    // 如果当前选中的助手不在新列表中，选择第一个助手
                    if (
                        assistantList.length > 0 &&
                        !assistantList.some((assistant) => assistant.id === selectedAssistant)
                    ) {
                        setSelectedAssistant(assistantList[0].id);
                    }
                },
                [selectedAssistant]
            ),
        });

        // 监听错误通知事件
        useEffect(() => {
            const unsubscribe = listen("conversation-window-error-notification", (event) => {
                const errorMessage = event.payload as string;
                console.error("Received error notification:", errorMessage);

                // 重置AI响应状态
                setAiIsResponsing(false);

                // 使用智能边框控制，而不是直接清空
                updateShiningMessages();
            });

            return () => {
                if (unsubscribe) {
                    unsubscribe.then((f) => f());
                }
            };
        }, [updateShiningMessages]);

        // ============= 组件渲染 =============

        return (
            <div ref={dropRef} className="h-full relative flex flex-col bg-background rounded-xl">
                <ConversationHeader
                    conversationId={conversationId}
                    conversation={conversation}
                    onEdit={openTitleEditDialog}
                    onDelete={handleDeleteConversationSuccess}
                />

                <div
                    ref={scrollContainerRef}
                    onScroll={handleScroll}
                    className="h-full flex-1 overflow-y-auto flex flex-col p-6 box-border gap-4"
                >
                    <ConversationContent
                        conversationId={conversationId}
                        // MessageList props
                        allDisplayMessages={allDisplayMessages}
                        streamingMessages={streamingMessages}
                        shiningMessageIds={shiningMessageIds}
                        reasoningExpandStates={reasoningExpandStates}
                        mcpToolCallStates={mcpToolCallStates}
                        generationGroups={messageGroupsData.generationGroups}
                        selectedVersions={messageGroupsData.selectedVersions}
                        getGenerationGroupControl={messageGroupsData.getGenerationGroupControl}
                        handleGenerationVersionChange={messageGroupsData.handleGenerationVersionChange}
                        onCodeRun={handleArtifact}
                        onMessageRegenerate={handleMessageRegenerate}
                        onMessageEdit={handleMessageEdit}
                        onMessageFork={handleMessageFork}
                        onToggleReasoningExpand={toggleReasoningExpand}
                        // NewChatComponent props
                        selectedText={selectedText}
                        selectedAssistant={selectedAssistant}
                        assistants={assistants}
                        setSelectedAssistant={setSelectedAssistant}
                    />
                    <div className="flex-none h-[120px]"></div>
                    <div ref={messagesEndRef} />
                </div>

                {isDragging ? <FileDropArea onDragChange={setIsDragging} onFilesSelect={handleChooseFile} /> : null}

                <InputArea
                    ref={inputAreaRef}
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
                    <div className="bg-background/95 w-full h-full absolute flex items-center justify-center backdrop-blur rounded-xl">
                        <div className="loading-icon"></div>
                        <div className="text-primary text-base font-medium">加载中...</div>
                    </div>
                ) : null}
            </div>
        );
    }
);

export default ConversationUI;
