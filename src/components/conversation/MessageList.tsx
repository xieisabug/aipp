import React, { useMemo } from "react";
import MessageItem from "../MessageItem";
import VersionPagination from "../VersionPagination";
import { Message, StreamEvent } from "../../data/Conversation";

export interface MessageListProps {
    allDisplayMessages: Message[];
    streamingMessages: Map<number, StreamEvent>;
    shiningMessageIds: Set<number>;
    reasoningExpandStates: Map<number, boolean>;
    mcpToolCallStates: Map<number, any>;
    generationGroups: Map<string, any>;
    selectedVersions: Map<string, number>;
    getGenerationGroupControl: (message: Message) => any;
    handleGenerationVersionChange: (groupId: string, versionIndex: number) => void;
    onCodeRun: (lang: string, inputStr: string) => void;
    onMessageRegenerate: (messageId: number) => void;
    onMessageEdit: (message: Message) => void;
    onMessageFork: (messageId: number) => void;
    onToggleReasoningExpand: (messageId: number) => void;
}

// 使用 React.memo 优化 MessageItem 渲染
const MemoizedMessageItem = React.memo(MessageItem);

const MessageList: React.FC<MessageListProps> = ({
    allDisplayMessages,
    streamingMessages,
    shiningMessageIds,
    reasoningExpandStates,
    mcpToolCallStates,
    generationGroups,
    selectedVersions,
    getGenerationGroupControl,
    handleGenerationVersionChange,
    onCodeRun,
    onMessageRegenerate,
    onMessageEdit,
    onMessageFork,
    onToggleReasoningExpand,
}) => {
    // 将消息渲染逻辑拆分为更小的部分
    const messageElements = useMemo(() => {
        return allDisplayMessages.map((message) => {
            // 查找对应的流式消息信息（如果存在）
            const streamEvent = streamingMessages.get(message.id);

            // 检查是否需要显示版本控制
            const groupControl = getGenerationGroupControl(message);

            // 检查是否需要显示shine-border
            const shouldShowShineBorder = shiningMessageIds.has(message.id);

            return {
                messageId: message.id,
                messageElement: (
                    <MemoizedMessageItem
                        key={`message-${message.id}`}
                        message={message}
                        streamEvent={streamEvent}
                        onCodeRun={onCodeRun}
                        onMessageRegenerate={() => onMessageRegenerate(message.id)}
                        onMessageEdit={() => onMessageEdit(message)}
                        onMessageFork={() => onMessageFork(message.id)}
                        // Reasoning 展开状态相关 props
                        isReasoningExpanded={
                            reasoningExpandStates.get(message.id) || false
                        }
                        onToggleReasoningExpand={() =>
                            onToggleReasoningExpand(message.id)
                        }
                        // ShineBorder 动画状态
                        shouldShowShineBorder={shouldShowShineBorder}
                        // MCP 工具调用需要的上下文信息
                        conversationId={message.conversation_id}
                        // 传递 MCP 工具调用状态
                        mcpToolCallStates={mcpToolCallStates}
                    />
                ),
                groupControl,
            };
        });
    }, [
        allDisplayMessages,
        streamingMessages,
        shiningMessageIds,
        reasoningExpandStates,
        mcpToolCallStates,
        getGenerationGroupControl,
        onCodeRun,
        onMessageRegenerate,
        onMessageEdit,
        onToggleReasoningExpand,
    ]);

    // 优化版本控制组件的渲染
    const versionControlElements = useMemo(() => {
        return messageElements
            .filter(({ groupControl }) => groupControl)
            .map(({ messageId, groupControl }) => (
                <div key={`version-${messageId}`} className="flex justify-start mt-2">
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
            ));
    }, [messageElements, handleGenerationVersionChange]);

    // 优化占位符消息的渲染
    const placeholderElements = useMemo(() => {
        const placeholders: React.ReactElement[] = [];
        
        generationGroups.forEach((group, groupId) => {
            const selectedVersionIndex =
                selectedVersions.get(groupId) ??
                (group.versions.length > 0 ? group.versions.length - 1 : 0);
            const selectedVersionData = group.versions[selectedVersionIndex];

            // 如果选中的是占位符版本，添加占位符消息
            if (selectedVersionData?.isPlaceholder) {
                placeholders.push(
                    <React.Fragment key={`placeholder_${groupId}`}>
                        <div className="flex justify-start mb-4">
                            <div className="bg-muted rounded-lg p-4 max-w-3xl">
                                <div className="flex items-center space-x-2">
                                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-foreground"></div>
                                    <span className="text-sm text-muted-foreground">
                                        正在重新生成...
                                    </span>
                                </div>
                            </div>
                        </div>
                        <div className="flex justify-start mt-2">
                            <VersionPagination
                                currentVersion={selectedVersionIndex + 1}
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
            }
        });

        return placeholders;
    }, [generationGroups, selectedVersions, handleGenerationVersionChange]);

    // 组合所有元素
    const allElements = useMemo(() => {
        const elements: React.ReactElement[] = [];
        
        // 添加消息元素
        messageElements.forEach(({ messageElement, groupControl }, index) => {
            elements.push(messageElement);
            
            // 如果有版本控制，添加对应的版本控制元素
            if (groupControl) {
                const versionElement = versionControlElements.find(
                    (element) => element.key === `version-${messageElements[index].messageId}`
                );
                if (versionElement) {
                    elements.push(versionElement);
                }
            }
        });
        
        // 添加占位符元素
        elements.push(...placeholderElements);
        
        return elements;
    }, [messageElements, versionControlElements, placeholderElements]);

    return <>{allElements}</>;
};

export default React.memo(MessageList);