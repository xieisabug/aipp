import { useCallback, useEffect, useRef, useState, startTransition } from "react";
import { listen } from "@tauri-apps/api/event";
import {
    StreamEvent,
    ConversationEvent,
    MessageUpdateEvent,
    GroupMergeEvent,
    MCPToolCallUpdateEvent,
    ConversationCancelEvent,
} from "../data/Conversation";

export interface UseConversationEventsOptions {
    conversationId: string | number;
    onMessageAdd?: (messageData: any) => void;
    onMessageUpdate?: (streamEvent: StreamEvent) => void;
    onGroupMerge?: (groupMergeData: GroupMergeEvent) => void;
    onMCPToolCallUpdate?: (mcpUpdateData: MCPToolCallUpdateEvent) => void;
    onConversationCancel?: (cancelData: ConversationCancelEvent) => void;
    onAiResponseStart?: () => void;
    onAiResponseComplete?: () => void;
    onError?: (errorMessage: string) => void;
}

export function useConversationEvents(options: UseConversationEventsOptions) {
    // 流式消息状态管理，存储正在流式传输的消息
    const [streamingMessages, setStreamingMessages] = useState<
        Map<number, StreamEvent>
    >(new Map());

    // ShineBorder 动画状态管理
    const [shiningMessageIds, setShiningMessageIds] = useState<Set<number>>(
        new Set(),
    );

    // MCP工具调用状态管理
    const [mcpToolCallStates, setMCPToolCallStates] = useState<
        Map<number, MCPToolCallUpdateEvent>
    >(new Map());

    // 活跃的 MCP 工具调用 ID 集合（正在执行的）
    const [activeMcpCallIds, setActiveMcpCallIds] = useState<Set<number>>(
        new Set(),
    );

    // 正在输出的 assistant 消息 ID 集合
    const [streamingAssistantMessageIds, setStreamingAssistantMessageIds] = useState<Set<number>>(
        new Set(),
    );

    // 等待回复的用户消息 ID（只有一个）
    const [pendingUserMessageId, setPendingUserMessageId] = useState<number | null>(null);

    // 事件监听取消订阅引用
    const unsubscribeRef = useRef<Promise<() => void> | null>(null);

    // 使用 ref 存储最新的回调函数，避免依赖项变化
    const callbacksRef = useRef(options);
    
    // 使用 ref 存储最新的 functionMap，避免频繁变化
    const functionMapRef = useRef<Map<number, any>>(new Map());

    // 更新 ref 中的回调函数
    useEffect(() => {
        callbacksRef.current = options;
    }, [options]);

    // 智能边框控制辅助函数 - 优先级：MCP > Assistant > 等待回复的用户消息
    const updateShiningMessages = useCallback(() => {
        setShiningMessageIds(() => {
            const newShining = new Set<number>();

            // 优先级 1: 如果有活跃的 MCP 调用，不显示任何消息边框（MCP 组件自己控制边框）
            if (activeMcpCallIds.size > 0) {
                return newShining; // 清空所有消息边框
            }

            // 优先级 2: 如果有 Assistant 消息正在输出，只显示 Assistant 边框
            if (streamingAssistantMessageIds.size > 0) {
                streamingAssistantMessageIds.forEach((messageId) => {
                    newShining.add(messageId);
                });
                console.log("✨ [DEBUG] Shining messages:", Array.from(newShining), "- Assistant streaming");
                return newShining; // 只显示 Assistant 消息边框
            }

            // 优先级 3: 如果有等待回复的用户消息，显示用户消息边框
            if (pendingUserMessageId !== null) {
                newShining.add(pendingUserMessageId);
                console.log("✨ [DEBUG] Shining messages:", Array.from(newShining), "- User pending");
                return newShining; // 只显示用户消息边框
            }

            // 优先级 4: 没有任何活跃状态时，清空所有边框
            console.log("🧹 [DEBUG] Shining messages: [] - No active states, clearing all borders");
            return newShining; // 清空所有边框
        });
    }, [activeMcpCallIds, streamingAssistantMessageIds, pendingUserMessageId]);

    // 当状态变化时，更新边框显示
    useEffect(() => {
        updateShiningMessages();
    }, [updateShiningMessages]);

    // 统一的事件处理函数
    const handleConversationEvent = useCallback(
        (event: any) => {
            const conversationEvent = event.payload as ConversationEvent;

            if (conversationEvent.type === "message_add") {
                // 处理消息添加事件
                const messageAddData = conversationEvent.data as any;
                console.log("Received message_add event:", messageAddData);

                // 如果是用户消息，设置为等待回复的消息，而不是直接设置边框
                if (messageAddData.message_type === "user") {
                    setPendingUserMessageId(messageAddData.message_id);
                }

                // 调用外部的消息添加处理函数
                callbacksRef.current.onMessageAdd?.(messageAddData);
            } else if (conversationEvent.type === "message_update") {
                const messageUpdateData =
                    conversationEvent.data as MessageUpdateEvent;

                const streamEvent: StreamEvent = {
                    message_id: messageUpdateData.message_id,
                    message_type: messageUpdateData.message_type as any,
                    content: messageUpdateData.content,
                    is_done: messageUpdateData.is_done,
                };

                // 检查是否是错误消息
                if (messageUpdateData.message_type === "error") {
                    // 对于错误消息，立即触发错误处理和状态清理
                    console.error("Received error message:", messageUpdateData.content);
                    
                    // 清理所有边框相关状态
                    setPendingUserMessageId(null);
                    setStreamingAssistantMessageIds(new Set());
                    
                    // 调用错误处理回调
                    callbacksRef.current.onError?.(messageUpdateData.content);
                    callbacksRef.current.onAiResponseComplete?.(); // 错误也算作响应完成
                    
                    // 对于错误消息，处理完成状态并延长显示时间
                    if (messageUpdateData.is_done) {
                        setStreamingMessages((prev) => {
                            const newMap = new Map(prev);
                            const completedEvent = {
                                ...streamEvent,
                                is_done: true,
                            };
                            newMap.set(streamEvent.message_id, completedEvent);
                            return newMap;
                        });

                        // 错误消息保留更长时间，让用户能看到完整的错误信息
                        setTimeout(() => {
                            setStreamingMessages((prev) => {
                                const newMap = new Map(prev);
                                newMap.delete(streamEvent.message_id);
                                return newMap;
                            });
                        }, 8000); // 8秒后清理错误消息，给用户更多时间阅读
                    }
                } else {
                    // 正常消息处理逻辑
                    
                    // 处理 assistant 消息的流式输出边框
                    if (messageUpdateData.message_type === "response" || messageUpdateData.message_type === "assistant") {
                        if (messageUpdateData.is_done) {
                            // Assistant 消息完成，从流式消息集合中移除
                            console.log("✅ [DEBUG] Assistant message COMPLETED:", messageUpdateData.message_id);
                            setStreamingAssistantMessageIds((prev) => {
                                const newSet = new Set(prev);
                                newSet.delete(messageUpdateData.message_id);
                                return newSet;
                            });
                        } else if (messageUpdateData.content) {
                            // Assistant 消息开始输出，清除等待回复的用户消息，添加到流式消息集合
                            console.log("🚀 [DEBUG] Assistant message STARTING:", messageUpdateData.message_id);
                            setPendingUserMessageId(null); // 清除等待回复的用户消息
                            setStreamingAssistantMessageIds((prev) => {
                                const newSet = new Set(prev);
                                newSet.add(messageUpdateData.message_id);
                                return newSet;
                            });
                        }
                    }

                    // 当开始收到新的AI响应时（不是is_done时），清除用户消息的shine-border
                    if (
                        !messageUpdateData.is_done &&
                        messageUpdateData.content
                    ) {
                        if (messageUpdateData.message_type !== "user") {
                            // 不直接清空，而是移除用户消息的边框，通过 updateShiningMessages 来智能控制
                            callbacksRef.current.onAiResponseStart?.();
                        }
                    }

                    if (messageUpdateData.is_done) {
                        if (messageUpdateData.message_type === "response") {
                            callbacksRef.current.onAiResponseComplete?.();
                        }

                        // 标记流式消息为完成状态，但不立即删除，让消息能正常显示
                        setStreamingMessages((prev) => {
                            const newMap = new Map(prev);
                            const completedEvent = {
                                ...streamEvent,
                                is_done: true,
                            };
                            newMap.set(streamEvent.message_id, completedEvent);
                            return newMap;
                        });

                        // 延迟清理已完成的流式消息，给足够时间让消息保存到 messages 中
                        setTimeout(() => {
                            setStreamingMessages((prev) => {
                                const newMap = new Map(prev);
                                newMap.delete(streamEvent.message_id);
                                return newMap;
                            });
                        }, 1000); // 1秒后清理
                    } else {
                        // 使用 startTransition 将流式消息更新标记为低优先级，保持界面响应性
                        startTransition(() => {
                            setStreamingMessages((prev) => {
                                const newMap = new Map(prev);
                                newMap.set(streamEvent.message_id, streamEvent);
                                return newMap;
                            });
                        });
                    }
                }

                // 处理插件兼容性
                const functionMap = functionMapRef.current;
                const streamMessageListener = functionMap.get(
                    streamEvent.message_id,
                )?.onStreamMessageListener;
                if (streamMessageListener) {
                    streamMessageListener(
                        streamEvent.content,
                        { conversation_id: +callbacksRef.current.conversationId, request_prompt_result_with_context: "" },
                        () => {}, // 空的 setAiIsResponsing 函数，实际应该从外部传入
                    );
                }

                // 调用外部的消息更新处理函数
                callbacksRef.current.onMessageUpdate?.(streamEvent);
            } else if (conversationEvent.type === "group_merge") {
                // 处理组合并事件
                const groupMergeData =
                    conversationEvent.data as GroupMergeEvent;
                console.log("Received group merge event:", groupMergeData);

                // 调用外部的组合并处理函数
                callbacksRef.current.onGroupMerge?.(groupMergeData);
            } else if (conversationEvent.type === "mcp_tool_call_update") {
                // 处理MCP工具调用状态更新事件
                const mcpUpdateData = conversationEvent.data as MCPToolCallUpdateEvent;
                console.log("Received mcp_tool_call_update event:", mcpUpdateData);

                // 更新MCP工具调用状态
                setMCPToolCallStates((prev) => {
                    const newMap = new Map(prev);
                    newMap.set(mcpUpdateData.call_id, mcpUpdateData);
                    return newMap;
                });

                // 更新活跃的 MCP 调用状态
                setActiveMcpCallIds((prev) => {
                    const newSet = new Set(prev);
                    
                    if (mcpUpdateData.status === "executing" || mcpUpdateData.status === "pending") {
                        // MCP 开始执行，添加到活跃集合
                        newSet.add(mcpUpdateData.call_id);
                    } else if (mcpUpdateData.status === "success" || mcpUpdateData.status === "failed") {
                        // MCP 执行完成，从活跃集合中移除
                        newSet.delete(mcpUpdateData.call_id);
                    }
                    
                    return newSet;
                });

                // 调用外部的MCP状态更新处理函数
                callbacksRef.current.onMCPToolCallUpdate?.(mcpUpdateData);
            } else if (conversationEvent.type === "conversation_cancel") {
                // 处理对话取消事件
                const cancelData = conversationEvent.data as ConversationCancelEvent;
                console.log("Received conversation_cancel event:", cancelData);

                // 立即清理所有流式状态，停止显示闪亮边框
                setPendingUserMessageId(null);
                setStreamingAssistantMessageIds(new Set());
                setActiveMcpCallIds(new Set());

                // 调用 AI 响应完成回调，确保状态重置
                callbacksRef.current.onAiResponseComplete?.();
                
                // 调用外部的取消处理函数
                callbacksRef.current.onConversationCancel?.(cancelData);
            }
        },
        [], // 不再依赖 options，因为我们使用 callbacksRef
    );

    // 设置和清理事件监听
    useEffect(() => {
        if (!callbacksRef.current.conversationId) {
            // 清理状态
            setStreamingMessages(new Map());
            setShiningMessageIds(new Set());
            setMCPToolCallStates(new Map());
            setActiveMcpCallIds(new Set());
            setStreamingAssistantMessageIds(new Set());
            setPendingUserMessageId(null);
            return;
        }

        console.log(
            `Setting up conversation event listener for: conversation_event_${callbacksRef.current.conversationId}`,
        );

        // 取消之前的事件监听
        if (unsubscribeRef.current) {
            console.log("Unsubscribing from previous event listener");
            unsubscribeRef.current.then((f) => f());
        }

        // 设置新的事件监听
        unsubscribeRef.current = listen(
            `conversation_event_${callbacksRef.current.conversationId}`,
            handleConversationEvent,
        );

        return () => {
            if (unsubscribeRef.current) {
                console.log("unsubscribe conversation events");
                unsubscribeRef.current.then((f) => f());
            }
        };
    }, [options.conversationId]); // 只依赖 conversationId

    // 清理函数
    const clearStreamingMessages = useCallback(() => {
        setStreamingMessages(new Map());
    }, []);

    const clearShiningMessages = useCallback(() => {
        setShiningMessageIds(new Set());
        setStreamingAssistantMessageIds(new Set());
        setPendingUserMessageId(null);
    }, []);

    const handleError = useCallback((errorMessage: string) => {
        console.error("Global error handler called:", errorMessage);
        
        // 清理所有流式消息状态
        setStreamingMessages(new Map());
        setShiningMessageIds(new Set());
        setMCPToolCallStates(new Map());
        setActiveMcpCallIds(new Set());
        setStreamingAssistantMessageIds(new Set());
        setPendingUserMessageId(null); // 清理等待回复的用户消息
        
        // 调用外部错误处理，确保状态重置
        callbacksRef.current.onError?.(errorMessage);
        callbacksRef.current.onAiResponseComplete?.();
    }, []);

    // 提供稳定的 functionMap 更新接口
    const updateFunctionMap = useCallback((functionMap: Map<number, any>) => {
        functionMapRef.current = functionMap;
    }, []);

    return {
        streamingMessages,
        shiningMessageIds,
        setShiningMessageIds,
        mcpToolCallStates,
        activeMcpCallIds, // 导出活跃的 MCP 调用状态
        streamingAssistantMessageIds, // 导出正在流式输出的 assistant 消息状态
        clearStreamingMessages,
        clearShiningMessages,
        handleError,
        updateShiningMessages, // 导出智能边框更新函数
        updateFunctionMap, // 导出 functionMap 更新函数
    };
}