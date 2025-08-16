import React from "react";
import IconButton from "../IconButton";
import Edit from "../../assets/edit.svg?react";
import Delete from "../../assets/delete.svg?react";
import { Conversation } from "../../data/Conversation";

const ConversationTitle: React.FC<{
    conversation: Conversation | undefined;
    onEdit: () => void;
    onDelete: () => void;
}> = React.memo(({ conversation, onEdit, onDelete }) => (
    <div className="flex justify-between flex-none h-[68px] items-center px-6 box-border border-b border-border bg-background rounded-t-xl">
        <div className="flex-1 overflow-hidden">
            <div className="text-base font-semibold overflow-hidden text-ellipsis whitespace-nowrap text-foreground cursor-pointer" onClick={onEdit}>{conversation?.name}</div>
            <div className="text-xs text-muted-foreground overflow-hidden text-ellipsis whitespace-nowrap mt-0.5">{conversation?.assistant_name}</div>
        </div>
        <div className="flex items-center flex-none w-40 justify-end gap-2">
            <IconButton icon={<Edit className="fill-foreground" />} onClick={onEdit} border />
            <IconButton icon={<Delete className="fill-foreground" />} onClick={onDelete} border />
        </div>
    </div>
));

export default ConversationTitle;