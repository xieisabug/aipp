import React from "react";
import { Edit2, GitBranch } from "lucide-react";
import IconButton from "../IconButton";
import Copy from "../../assets/copy.svg?react";
import Ok from "../../assets/ok.svg?react";
import Refresh from "../../assets/refresh.svg?react";

interface MessageActionButtonsProps {
    messageType: string;
    isUserMessage: boolean;
    copyIconState: "copy" | "ok";
    onCopy: () => void;
    onEdit?: () => void;
    onRegenerate?: () => void;
    onFork?: () => void;
}

const MessageActionButtons: React.FC<MessageActionButtonsProps> = ({
    messageType,
    isUserMessage,
    copyIconState,
    onCopy,
    onEdit,
    onRegenerate,
    onFork,
}) => {
    const showEditRegenerate = messageType === "assistant" || messageType === "response" || messageType === "user";

    return (
        <div
            className={`hidden z-10 group-hover:flex items-center absolute -bottom-9 py-3 px-4 box-border h-10 rounded-[21px] border border-border bg-background ${
                isUserMessage ? "right-0" : "left-0"
            }`}
        >
            {showEditRegenerate && onEdit && (
                <IconButton icon={<Edit2 size={16} className="stroke-foreground" />} onClick={onEdit} />
            )}
            {showEditRegenerate && onRegenerate && (
                <IconButton icon={<Refresh className="fill-foreground" />} onClick={onRegenerate} />
            )}
            {messageType === "response" && onFork && (
                <IconButton icon={<GitBranch size={16} className="stroke-foreground" />} onClick={onFork} />
            )}
            <IconButton
                icon={
                    copyIconState === "copy" ? <Copy className="fill-foreground" /> : <Ok className="fill-foreground" />
                }
                onClick={onCopy}
            />
        </div>
    );
};

export default MessageActionButtons;
