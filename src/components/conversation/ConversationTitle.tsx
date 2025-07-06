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
    <div className="flex justify-between flex-none h-[68px] items-center px-6 box-border border-b border-gray-200 bg-white rounded-t-xl">
        <div className="flex-1 overflow-hidden">
            <div className="text-base font-semibold overflow-hidden text-ellipsis whitespace-nowrap text-gray-800 cursor-pointer" onClick={onEdit}>{conversation?.name}</div>
            <div className="text-xs text-gray-500 overflow-hidden text-ellipsis whitespace-nowrap mt-0.5">{conversation?.assistant_name}</div>
        </div>
        <div className="flex items-center flex-none w-40 justify-end gap-2">
            <IconButton icon={<Edit fill="black" />} onClick={onEdit} border />
            <IconButton icon={<Delete fill="black" />} onClick={onDelete} border />
        </div>
    </div>
));

export default ConversationTitle;