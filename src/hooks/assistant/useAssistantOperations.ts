import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { AssistantDetail, AssistantListItem } from "@/data/Assistant";

export const useAssistantOperations = () => {
    const [assistants, setAssistants] = useState<AssistantListItem[]>([]);
    const [currentAssistant, setCurrentAssistant] = useState<AssistantDetail | null>(null);

    // 保存助手
    const saveAssistant = useCallback((assistant: AssistantDetail) => {
        return invoke<void>("save_assistant", { assistantDetail: assistant });
    }, []);

    // 复制助手
    const copyAssistant = useCallback(() => {
        if (!currentAssistant) return Promise.reject("No current assistant");

        return invoke<AssistantDetail>("copy_assistant", {
            assistantId: currentAssistant.assistant.id,
        })
            .then((assistantDetail: AssistantDetail) => {
                setAssistants((prev) => [
                    ...prev,
                    {
                        id: assistantDetail.assistant.id,
                        name: assistantDetail.assistant.name,
                        assistant_type: assistantDetail.assistant.assistant_type,
                    },
                ]);
                setCurrentAssistant(assistantDetail);
                toast.success("复制助手成功");
                return assistantDetail;
            })
            .catch((error) => {
                toast.error("复制助手失败: " + error);
                throw error;
            });
    }, [currentAssistant]);

    // 删除助手
    const deleteAssistant = useCallback(() => {
        if (!currentAssistant) return Promise.reject("No current assistant");

        return invoke("delete_assistant", {
            assistantId: currentAssistant.assistant.id,
        })
            .then(() => {
                const newAssistants = assistants.filter(
                    (assistant) => assistant.id !== currentAssistant.assistant.id,
                );
                setAssistants(newAssistants);
                if (newAssistants.length > 0) {
                    // 选择第一个助手，这里需要外部处理
                    return { assistants: newAssistants, shouldSelectFirst: true };
                } else {
                    setCurrentAssistant(null);
                    return { assistants: newAssistants, shouldSelectFirst: false };
                }
            })
            .then((result) => {
                toast.success("删除助手成功");
                return result;
            })
            .catch((error) => {
                toast.error("删除助手失败: " + error);
                throw error;
            });
    }, [currentAssistant, assistants]);

    // 获取助手列表
    const loadAssistants = useCallback(() => {
        return invoke<Array<AssistantListItem>>("get_assistants")
            .then((assistantList) => {
                setAssistants(assistantList);
                return assistantList;
            })
            .catch((error) => {
                toast.error("获取助手列表失败: " + error);
                throw error;
            });
    }, []);

    // 获取助手详情
    const loadAssistantDetail = useCallback((assistantId: number) => {
        return invoke<AssistantDetail>("get_assistant", { assistantId })
            .then((assistant) => {
                setCurrentAssistant(assistant);
                return assistant;
            })
            .catch((error) => {
                toast.error("获取助手信息失败: " + error);
                throw error;
            });
    }, []);

    // 分享助手
    const shareAssistant = useCallback(() => {
        if (!currentAssistant) return Promise.reject("No current assistant");

        return invoke<string>('export_assistant', { 
            assistantId: currentAssistant.assistant.id 
        }).catch((error) => {
            toast.error('分享失败: ' + error);
            throw error;
        });
    }, [currentAssistant]);

    // 导入助手
    const importAssistant = useCallback(async (
        shareCode: string, 
        _password?: string, 
        newName?: string
    ): Promise<void> => {
        await invoke('import_assistant', {
            shareCode,
            newName
        });
    }, []);

    // 更新助手信息
    const updateAssistantInfo = useCallback(
        (updatedAssistant: AssistantDetail) => {
            setCurrentAssistant(updatedAssistant);
            const index = assistants.findIndex(
                (assistant) => assistant.id === updatedAssistant.assistant.id,
            );
            if (index >= 0) {
                const newAssistants = [...assistants];
                newAssistants[index] = {
                    id: updatedAssistant.assistant.id,
                    name: updatedAssistant.assistant.name,
                    assistant_type: updatedAssistant.assistant.assistant_type,
                };
                setAssistants(newAssistants);
            }
        },
        [assistants],
    );

    // 添加新助手
    const addAssistant = useCallback((assistantDetail: AssistantDetail) => {
        setAssistants((prev) => [
            ...prev,
            {
                id: assistantDetail.assistant.id,
                name: assistantDetail.assistant.name,
                assistant_type: assistantDetail.assistant.assistant_type,
            },
        ]);
        setCurrentAssistant(assistantDetail);
    }, []);

    return {
        assistants,
        currentAssistant,
        setAssistants,
        setCurrentAssistant,
        saveAssistant,
        copyAssistant,
        deleteAssistant,
        loadAssistants,
        loadAssistantDetail,
        shareAssistant,
        importAssistant,
        updateAssistantInfo,
        addAssistant,
    };
};