import React from "react";
import { AssistantDetail } from "@/data/Assistant";
import { DialogStates } from "@/types/forms";
import ConfirmDialog from "@/components/ConfirmDialog";
import EditAssistantDialog from "@/components/config/EditAssistantDialog";
import ShareDialog from "@/components/ShareDialog";
import ImportDialog from "@/components/ImportDialog";

interface AssistantDialogsProps {
    dialogStates: DialogStates;
    shareCode: string;
    currentAssistant: AssistantDetail | null;
    onConfirmDelete: () => void;
    onCancelDelete: () => void;
    onSave: (assistant: AssistantDetail) => Promise<void>;
    onAssistantUpdated: (assistant: AssistantDetail) => void;
    onImportAssistant: (shareCode: string, password?: string, newName?: string) => Promise<void>;
    onCloseUpdateForm: () => void;
    onCloseShare: () => void;
    onCloseImport: () => void;
}

export const AssistantDialogs: React.FC<AssistantDialogsProps> = ({
    dialogStates,
    shareCode,
    currentAssistant,
    onConfirmDelete,
    onCancelDelete,
    onSave,
    onAssistantUpdated,
    onImportAssistant,
    onCloseUpdateForm,
    onCloseShare,
    onCloseImport,
}) => {
    return (
        <>
            {/* 确认删除对话框 */}
            <ConfirmDialog
                title="确认删除"
                confirmText="该操作不可逆，确认执行删除助手操作吗？删除后，配置将会删除，并且该助手的对话将转移到快速使用助手，且不可恢复。"
                onConfirm={onConfirmDelete}
                onCancel={onCancelDelete}
                isOpen={dialogStates.confirmDeleteOpen}
            />

            {/* 编辑助手对话框 */}
            <EditAssistantDialog
                isOpen={dialogStates.updateFormOpen}
                onClose={onCloseUpdateForm}
                currentAssistant={currentAssistant}
                onSave={onSave}
                onAssistantUpdated={onAssistantUpdated}
            />

            {/* 分享对话框 */}
            <ShareDialog
                title="助手配置"
                shareCode={shareCode}
                isOpen={dialogStates.shareOpen}
                onClose={onCloseShare}
            />

            {/* 导入对话框 */}
            <ImportDialog
                title="助手配置"
                isOpen={dialogStates.importOpen}
                requiresPassword={false}
                onClose={onCloseImport}
                onImport={onImportAssistant}
            />
        </>
    );
};