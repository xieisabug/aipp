import React, { useState, useCallback } from "react";
import IconButton from "../IconButton";
import Edit from "../../assets/edit.svg?react";
import Delete from "../../assets/delete.svg?react";
import { Conversation } from "../../data/Conversation";
import ConfirmDialog from "../ConfirmDialog";
import useConversationManager from "../../hooks/useConversationManager";

const ConversationTitle: React.FC<{
    conversation: Conversation | undefined;
    onEdit: () => void;
    onDelete: () => void;
}> = React.memo(({ conversation, onEdit, onDelete }) => {
    const [deleteDialogIsOpen, setDeleteDialogIsOpen] = useState<boolean>(false);
    const { deleteConversation } = useConversationManager();

    const openDeleteDialog = useCallback(() => {
        setDeleteDialogIsOpen(true);
    }, []);

    const closeDeleteDialog = useCallback(() => {
        setDeleteDialogIsOpen(false);
    }, []);

    const handleConfirmDelete = useCallback(() => {
        if (conversation) {
            deleteConversation(conversation.id.toString(), {
                onSuccess: async () => {
                    closeDeleteDialog();
                    onDelete(); // 通知父组件更新
                },
            });
        }
    }, [conversation, deleteConversation, onDelete, closeDeleteDialog]);

    return (
        <>
            <div className="flex justify-between flex-none h-[68px] items-center px-6 box-border border-b border-border bg-background rounded-t-xl">
                <div className="flex-1 overflow-hidden">
                    <div className="text-base font-semibold overflow-hidden text-ellipsis whitespace-nowrap text-foreground cursor-pointer" onClick={onEdit}>{conversation?.name}</div>
                    <div className="text-xs text-muted-foreground overflow-hidden text-ellipsis whitespace-nowrap mt-0.5">{conversation?.assistant_name}</div>
                </div>
                <div className="flex items-center flex-none w-40 justify-end gap-2">
                    <IconButton icon={<Edit className="fill-foreground" />} onClick={onEdit} border />
                    <IconButton icon={<Delete className="fill-foreground" />} onClick={openDeleteDialog} border />
                </div>
            </div>

            <ConfirmDialog
                title="确认删除对话"
                confirmText={`确定要删除对话 "${conversation?.name}" 吗？此操作无法撤销。`}
                onConfirm={handleConfirmDelete}
                onCancel={closeDeleteDialog}
                isOpen={deleteDialogIsOpen}
            />
        </>
    );
});

export default ConversationTitle;