import React from "react";
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
    onToggleReasoningExpand: (messageId: number) => void;
}

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
    onToggleReasoningExpand,
}) => {
    // 渲染已经过滤和排序的消息
    const filteredMessages = React.useMemo(() => {
        const result = allDisplayMessages.map((message) => {
            // 查找对应的流式消息信息（如果存在）
            const streamEvent = streamingMessages.get(message.id);

            // 检查是否需要显示版本控制
            const groupControl = getGenerationGroupControl(message);

            // 检查是否需要显示shine-border
            const shouldShowShineBorder = shiningMessageIds.has(message.id);

            return (
                <React.Fragment key={message.id}>
                    <MessageItem
                        message={message}
                        streamEvent={streamEvent}
                        onCodeRun={onCodeRun}
                        onMessageRegenerate={() => onMessageRegenerate(message.id)}
                        onMessageEdit={() => onMessageEdit(message)}
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
                    .flatMap((version: any) => version.messages)
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
        getGenerationGroupControl,
        handleGenerationVersionChange,
        reasoningExpandStates,
        onToggleReasoningExpand,
        generationGroups,
        selectedVersions,
        shiningMessageIds,
        mcpToolCallStates,
        onCodeRun,
        onMessageRegenerate,
        onMessageEdit,
    ]);

    return <>{filteredMessages}</>;
};

export default MessageList;