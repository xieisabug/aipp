import React from "react";
import ConversationTitle from "./ConversationTitle";
import { Conversation } from "../../data/Conversation";

export interface ConversationHeaderProps {
    conversationId: string;
    conversation?: Conversation;
    onEdit: () => void;
    onDelete: () => void;
}

const ConversationHeader: React.FC<ConversationHeaderProps> = ({
    conversationId,
    conversation,
    onEdit,
    onDelete,
}) => {
    if (!conversationId) {
        return null;
    }

    return (
        <ConversationTitle
            onEdit={onEdit}
            onDelete={onDelete}
            conversation={conversation}
        />
    );
};

export default ConversationHeader;