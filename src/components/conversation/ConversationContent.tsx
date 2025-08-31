import React, { useState } from "react";
import MessageList from "./MessageList";
import NewChatComponent from "../NewChatComponent";
import { Message, StreamEvent } from "../../data/Conversation";
import { AssistantListItem } from "../../data/Assistant";
import { SubTaskList, SubTaskDetailDialog } from "../sub-task";
import { SubTaskExecutionSummary } from "../../data/SubTask";

export interface ConversationContentProps {
    conversationId: string;
    // MessageList props
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
    // NewChatComponent props
    selectedText: string;
    selectedAssistant: number;
    assistants: AssistantListItem[];
    setSelectedAssistant: (assistantId: number) => void;
}

const ConversationContent: React.FC<ConversationContentProps> = ({
    conversationId,
    // MessageList props
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
    // NewChatComponent props
    selectedText,
    selectedAssistant,
    assistants,
    setSelectedAssistant,
}) => {
    // State for sub-task detail dialog
    const [selectedSubTask, setSelectedSubTask] = useState<SubTaskExecutionSummary | null>(null);
    const [isDetailDialogOpen, setIsDetailDialogOpen] = useState(false);

    // Handle sub-task detail view
    const handleSubTaskDetailView = (execution: SubTaskExecutionSummary) => {
        setSelectedSubTask(execution);
        setIsDetailDialogOpen(true);
    };

    const handleCloseDetailDialog = () => {
        setIsDetailDialogOpen(false);
        setSelectedSubTask(null);
    };

    const conversationIdNum = parseInt(conversationId);
    const isValidConversationId = !isNaN(conversationIdNum);

    if (conversationId) {
        return (
            <>
                {/* Conversation-level sub-tasks - shown between header and messages */}
                {isValidConversationId && (
                    <SubTaskList
                        conversation_id={conversationIdNum}
                        onTaskDetailView={handleSubTaskDetailView}
                    />
                )}

                <MessageList
                    allDisplayMessages={allDisplayMessages}
                    streamingMessages={streamingMessages}
                    shiningMessageIds={shiningMessageIds}
                    reasoningExpandStates={reasoningExpandStates}
                    mcpToolCallStates={mcpToolCallStates}
                    generationGroups={generationGroups}
                    selectedVersions={selectedVersions}
                    getGenerationGroupControl={getGenerationGroupControl}
                    handleGenerationVersionChange={handleGenerationVersionChange}
                    onCodeRun={onCodeRun}
                    onMessageRegenerate={onMessageRegenerate}
                    onMessageEdit={onMessageEdit}
                    onMessageFork={onMessageFork}
                    onToggleReasoningExpand={onToggleReasoningExpand}
                />

                {/* Sub-task detail dialog */}
                {selectedSubTask && (
                    <SubTaskDetailDialog
                        isOpen={isDetailDialogOpen}
                        onClose={handleCloseDetailDialog}
                        execution={selectedSubTask}
                        // 不再需要传递source_id，使用UI专用的详情接口
                    />
                )}
            </>
        );
    }

    return (
        <NewChatComponent
            selectedText={selectedText}
            selectedAssistant={selectedAssistant}
            assistants={assistants}
            setSelectedAssistant={setSelectedAssistant}
        />
    );
};

export default ConversationContent;