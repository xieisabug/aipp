import { useState, useCallback } from "react";
import { DialogStates } from "@/types/forms";

export const useDialogStates = () => {
    const [dialogStates, setDialogStates] = useState<DialogStates>({
        confirmDeleteOpen: false,
        updateFormOpen: false,
        shareOpen: false,
        importOpen: false,
    });

    const [shareCode, setShareCode] = useState('');

    // 打开确认删除对话框
    const openConfirmDeleteDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            confirmDeleteOpen: true,
        }));
    }, []);

    // 关闭确认删除对话框
    const closeConfirmDeleteDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            confirmDeleteOpen: false,
        }));
    }, []);

    // 打开更新表单对话框
    const openUpdateFormDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            updateFormOpen: true,
        }));
    }, []);

    // 关闭更新表单对话框
    const closeUpdateFormDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            updateFormOpen: false,
        }));
    }, []);

    // 打开分享对话框
    const openShareDialog = useCallback((code: string) => {
        setShareCode(code);
        setDialogStates(prev => ({
            ...prev,
            shareOpen: true,
        }));
    }, []);

    // 关闭分享对话框
    const closeShareDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            shareOpen: false,
        }));
        setShareCode('');
    }, []);

    // 打开导入对话框
    const openImportDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            importOpen: true,
        }));
    }, []);

    // 关闭导入对话框
    const closeImportDialog = useCallback(() => {
        setDialogStates(prev => ({
            ...prev,
            importOpen: false,
        }));
    }, []);

    return {
        dialogStates,
        shareCode,
        openConfirmDeleteDialog,
        closeConfirmDeleteDialog,
        openUpdateFormDialog,
        closeUpdateFormDialog,
        openShareDialog,
        closeShareDialog,
        openImportDialog,
        closeImportDialog,
    };
};