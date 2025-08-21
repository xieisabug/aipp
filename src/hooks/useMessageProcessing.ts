import { useMemo, useState } from "react";
import { Message, Conversation, StreamEvent } from "../data/Conversation";

export interface UseMessageProcessingProps {
    messages: Message[];
    streamingMessages: Map<number, StreamEvent>;
    conversation?: Conversation;
    generationGroups: Map<string, any>;
    groupRootMessageIds: Map<string, number>;
    getMessageVersionInfo: (message: Message) => any;
}

export interface UseMessageProcessingReturn {
    combinedMessagesForGrouping: Message[];
    allDisplayMessages: Message[];
    groupMergeMap: Map<string, string>;
    setGroupMergeMap: React.Dispatch<React.SetStateAction<Map<string, string>>>;
}

export function useMessageProcessing({
    messages,
    streamingMessages,
    conversation,
    generationGroups,
    groupRootMessageIds,
    getMessageVersionInfo,
}: UseMessageProcessingProps): UseMessageProcessingReturn {
    
    // 管理组合并关系：new_group_id -> original_group_id
    const [groupMergeMap, setGroupMergeMap] = useState<Map<string, string>>(
        new Map(),
    );

    // 首先合并常规消息和流式消息（不排序）
    const combinedMessagesForGrouping = useMemo(() => {
        const combinedMessages = [...messages];

        // 找到最后一个用户消息，用于确定当前对话轮次的基准时间
        const lastUserMessage = [...combinedMessages].reverse().find(msg => msg.message_type === "user");
        const baseTimestamp = lastUserMessage ? new Date(lastUserMessage.created_time).getTime() : Date.now();

        // 为了确保同一轮对话中的所有流式消息使用相同的时间戳，我们使用统一的时间戳
        const uniformStreamTimestamp = baseTimestamp + 1000;

        // 将流式消息添加到显示列表中
        streamingMessages.forEach((streamEvent) => {
            // 检查是否已经存在同样ID的消息
            const existingIndex = combinedMessages.findIndex(
                (msg) => msg.id === streamEvent.message_id,
            );
            if (existingIndex === -1) {
                const tempMessage: Message = {
                    id: streamEvent.message_id,
                    conversation_id: conversation?.id || 0,
                    message_type: streamEvent.message_type,
                    content: streamEvent.content,
                    llm_model_id: null,
                    created_time: new Date(uniformStreamTimestamp), // 使用统一时间戳
                    start_time: new Date(uniformStreamTimestamp),
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
                // 存在则更新消息内容，但保持时间戳不变
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

    // 智能排序逻辑：先过滤可见消息，再基于分组基准时间排序
    const allDisplayMessages = useMemo(() => {
        // 首先过滤出可见的消息
        const visibleMessages = combinedMessagesForGrouping.filter((message) => {
            // 跳过系统消息和工具结果
            if (message.message_type === "system" || message.message_type === "tool_result") {
                return false;
            }
            
            // 如果有版本信息检查函数，则检查版本可见性
            if (getMessageVersionInfo) {
                const versionInfo = getMessageVersionInfo(message);
                if (versionInfo && !versionInfo.shouldShow) {
                    return false;
                }
            }
            
            return true;
        });

        // 如果没有分组信息，直接按ID排序返回
        if (generationGroups.size === 0) {
            return visibleMessages.sort((a, b) => a.id - b.id);
        }

        // 创建消息到根组的映射，包括子分组的消息
        const messageToRootGroupMap = new Map<number, string>();
        
        // 首先建立所有 generation_group_id 到根分组的映射
        const generationIdToRootMap = new Map<string, string>();
        combinedMessagesForGrouping.forEach(message => {
            if (message.generation_group_id) {
                // 查找这个 generation_group_id 在哪个根分组中
                generationGroups.forEach((group, rootGroupId) => {
                    group.versions.forEach((version: any) => {
                        if (version.versionId === message.generation_group_id) {
                            generationIdToRootMap.set(message.generation_group_id!, rootGroupId);
                        }
                    });
                });
            }
        });
        
        // 然后为每个消息建立到根分组的映射
        combinedMessagesForGrouping.forEach(message => {
            if (message.generation_group_id) {
                const rootGroupId = generationIdToRootMap.get(message.generation_group_id);
                if (rootGroupId) {
                    messageToRootGroupMap.set(message.id, rootGroupId);
                }
            }
        });

        // 然后对可见消息进行排序
        const sorted = visibleMessages.sort((a, b) => {
            const aGroupId = messageToRootGroupMap.get(a.id);
            const bGroupId = messageToRootGroupMap.get(b.id);

            // 获取排序基准值：分组消息使用分组基准消息ID，非分组消息使用自身ID
            const aBaseValue = aGroupId ? (groupRootMessageIds.get(aGroupId) || a.id) : a.id;
            const bBaseValue = bGroupId ? (groupRootMessageIds.get(bGroupId) || b.id) : b.id;

            if (aBaseValue !== bBaseValue) {
                return aBaseValue - bBaseValue;
            }
            
            // 基准值相同时，按ID排序（同一分组内的消息或ID相同的情况）
            return a.id - b.id;
        });
        
        return sorted;
    }, [combinedMessagesForGrouping, generationGroups, groupRootMessageIds, getMessageVersionInfo]);

    return {
        combinedMessagesForGrouping,
        allDisplayMessages,
        groupMergeMap,
        setGroupMergeMap,
    };
}