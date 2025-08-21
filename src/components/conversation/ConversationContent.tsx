import React from "react";
import MessageList from "./MessageList";
import NewChatComponent from "../NewChatComponent";
import { Message, StreamEvent } from "../../data/Conversation";
import { AssistantListItem } from "../../data/Assistant";

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
    if (conversationId) {
        return (
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