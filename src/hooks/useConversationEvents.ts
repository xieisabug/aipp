import { useCallback, useEffect, useRef, useState, startTransition } from "react";
import { listen } from "@tauri-apps/api/event";
import {
    StreamEvent,
    ConversationEvent,
    MessageUpdateEvent,
    GroupMergeEvent,
} from "../data/Conversation";

export interface UseConversationEventsOptions {
    conversationId: string | number;
    onMessageAdd?: (messageData: any) => void;
    onMessageUpdate?: (streamEvent: StreamEvent) => void;
    onGroupMerge?: (groupMergeData: GroupMergeEvent) => void;
    onAiResponseStart?: () => void;
    onAiResponseComplete?: () => void;
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

    // 事件监听取消订阅引用
    const unsubscribeRef = useRef<Promise<() => void> | null>(null);

    // 使用 ref 存储最新的回调函数，避免依赖项变化
    const callbacksRef = useRef(options);

    // 更新 ref 中的回调函数
    useEffect(() => {
        callbacksRef.current = options;
    }, [options]);

    // 统一的事件处理函数
    const handleConversationEvent = useCallback(
        (event: any) => {
            const conversationEvent = event.payload as ConversationEvent;

            if (conversationEvent.type === "message_add") {
                // 处理消息添加事件
                const messageAddData = conversationEvent.data as any;
                console.log("Received message_add event:", messageAddData);

                // 如果是用户消息，设置shine border
                if (messageAddData.message_type === "user") {
                    setShiningMessageIds(
                        new Set([messageAddData.message_id]),
                    );
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

                // 当开始收到新的AI响应时（不是is_done时），清除所有shine-border
                if (
                    !messageUpdateData.is_done &&
                    messageUpdateData.content
                ) {
                    if (messageUpdateData.message_type !== "user") {
                        setShiningMessageIds(new Set());
                    }
                    callbacksRef.current.onAiResponseStart?.();
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

                // 调用外部的消息更新处理函数
                callbacksRef.current.onMessageUpdate?.(streamEvent);
            } else if (conversationEvent.type === "group_merge") {
                // 处理组合并事件
                const groupMergeData =
                    conversationEvent.data as GroupMergeEvent;
                console.log("Received group merge event:", groupMergeData);

                // 调用外部的组合并处理函数
                callbacksRef.current.onGroupMerge?.(groupMergeData);
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
    }, []);

    return {
        streamingMessages,
        shiningMessageIds,
        setShiningMessageIds,
        clearStreamingMessages,
        clearShiningMessages,
    };
}