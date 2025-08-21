import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { throttle } from "lodash";
import { Message, Conversation, FileInfo } from "../data/Conversation";
import { AssistantListItem } from "../data/Assistant";

// 从 plugin.d.ts 导入的接口类型
interface AiResponse {
    conversation_id: number;
    request_prompt_result_with_context: string;
}

export interface UseConversationOperationsProps {
    conversation?: Conversation;
    selectedAssistant: number;
    assistants: AssistantListItem[];
    setMessages: React.Dispatch<React.SetStateAction<Message[]>>;
    inputText: string;
    setInputText: React.Dispatch<React.SetStateAction<string>>;
    fileInfoList?: FileInfo[];
    clearFileInfoList: () => void;
    aiIsResponsing: boolean;
    setAiIsResponsing: (isResponsing: boolean) => void;
    onChangeConversationId: (conversationId: string) => void;
    setShiningMessageIds: React.Dispatch<React.SetStateAction<Set<number>>>;
    updateShiningMessages: () => void;
    assistantTypePluginMap: Map<number, any>;
    assistantRunApi: any;
}

export interface UseConversationOperationsReturn {
    // 对话管理
    handleDeleteConversationSuccess: () => void;
    
    // 消息操作
    handleMessageRegenerate: (regenerateMessageId: number) => void;
    handleMessageEdit: (message: Message) => void;
    handleEditSave: (content: string) => void;
    handleEditSaveAndRegenerate: (content: string) => void;
    handleSend: () => void;
    handleArtifact: (lang: string, inputStr: string) => void;
    
    // 编辑对话框状态
    editDialogIsOpen: boolean;
    editingMessage: Message | null;
    closeEditDialog: () => void;
    
    // 标题编辑
    titleEditDialogIsOpen: boolean;
    openTitleEditDialog: () => void;
    closeTitleEditDialog: () => void;
}

export function useConversationOperations({
    conversation,
    selectedAssistant,
    assistants,
    setMessages,
    inputText,
    setInputText,
    fileInfoList,
    clearFileInfoList,
    aiIsResponsing,
    setAiIsResponsing,
    onChangeConversationId,
    setShiningMessageIds,
    updateShiningMessages,
    assistantTypePluginMap,
    assistantRunApi,
}: UseConversationOperationsProps): UseConversationOperationsReturn {
    
    // 对话标题管理相关状态
    const [titleEditDialogIsOpen, setTitleEditDialogIsOpen] = useState<boolean>(false);
    
    // 消息编辑相关状态
    const [editDialogIsOpen, setEditDialogIsOpen] = useState<boolean>(false);
    const [editingMessage, setEditingMessage] = useState<Message | null>(null);

    // 对话管理相关操作
    const handleDeleteConversationSuccess = useCallback(() => {
        // 删除成功后清空会话ID，返回新建对话界面
        onChangeConversationId("");
    }, [onChangeConversationId]);

    // 消息重新生成处理
    const handleMessageRegenerate = useCallback(
        (regenerateMessageId: number) => {
            // 设置AI响应状态
            setAiIsResponsing(true);

            // 使用函数式更新设置被点击的消息显示shine-border
            setShiningMessageIds(() => new Set([regenerateMessageId]));

            invoke<AiResponse>("regenerate_ai", {
                messageId: regenerateMessageId,
            })
                .then((res) => {
                    console.log("regenerate ai response", res);
                    // 重新生成消息的处理逻辑
                    // setMessageId(res.add_message_id);
                })
                .catch((error) => {
                    console.error("Regenerate error:", error);
                    setAiIsResponsing(false);
                    // 使用智能边框控制，而不是直接清空
                    updateShiningMessages();
                    // 错误信息将在对话框中显示
                });
        },
        [setAiIsResponsing, updateShiningMessages],
    );

    // 消息编辑相关处理函数
    const handleMessageEdit = useCallback((message: Message) => {
        setEditingMessage(message);
        setEditDialogIsOpen(true);
    }, []);

    const closeEditDialog = useCallback(() => {
        setEditDialogIsOpen(false);
        setEditingMessage(null);
    }, []);

    const handleEditSave = useCallback(
        (content: string) => {
            if (!editingMessage) return;

            invoke("update_message_content", {
                messageId: editingMessage.id,
                content: content,
            })
                .then(() => {
                    // 更新本地消息状态
                    setMessages((prevMessages) =>
                        prevMessages.map((msg) =>
                            msg.id === editingMessage.id
                                ? { ...msg, content: content }
                                : msg,
                        ),
                    );
                    toast.success("消息已更新");
                })
                .catch((error) => {
                    toast.error("更新消息失败: " + error);
                });
        },
        [editingMessage, setMessages],
    );

    const handleEditSaveAndRegenerate = useCallback(
        (content: string) => {
            if (!editingMessage) return;

            // 先更新消息内容
            invoke("update_message_content", {
                messageId: editingMessage.id,
                content: content,
            })
                .then(() => {
                    // 更新本地消息状态
                    setMessages((prevMessages) =>
                        prevMessages.map((msg) =>
                            msg.id === editingMessage.id
                                ? { ...msg, content: content }
                                : msg,
                        ),
                    );

                    // 然后触发重新生成
                    handleMessageRegenerate(editingMessage.id);

                    toast.success("消息已更新并开始重新生成");
                })
                .catch((error) => {
                    toast.error("更新消息失败: " + error);
                });
        },
        [editingMessage, handleMessageRegenerate, setMessages],
    );

    // 代码运行处理
    const handleArtifact = useCallback((lang: string, inputStr: string) => {
        invoke("run_artifacts", { lang, inputStr })
            .then((res) => {
                console.log(res);
            })
            .catch((error) => {
                toast.error("运行失败: " + JSON.stringify(error));
            });
    }, []);

    // 打开标题编辑对话框
    const openTitleEditDialog = useCallback(() => {
        setTitleEditDialogIsOpen(true);
    }, []);

    // 关闭标题编辑对话框
    const closeTitleEditDialog = useCallback(() => {
        setTitleEditDialogIsOpen(false);
    }, []);

    // 发送消息的主要处理函数，使用节流防止频繁点击
    const handleSend = throttle(() => {
        if (aiIsResponsing) {
            // AI正在响应时，点击取消
            console.log("Cancelling AI");
            console.log(conversation?.id);
            invoke("cancel_ai", { conversationId: +(conversation?.id || 0) }).then(() => {
                setAiIsResponsing(false);
                // 使用智能边框控制
                updateShiningMessages();
            });
        } else {
            // 正常发送消息流程
            if (inputText.trim() === "") {
                setInputText("");
                return;
            }
            setAiIsResponsing(true);

            let conversationId = "";
            let assistantId = "";
            if (!conversation || !conversation.id) {
                assistantId = selectedAssistant + "";
            } else {
                conversationId = conversation.id + "";
                assistantId = conversation.assistant_id + "";
            }

            // 检查是否使用插件助手
            const assistantData = assistants.find((a) => a.id === +assistantId);
            if (assistantData?.assistant_type !== 0) {
                // 使用插件助手
                assistantTypePluginMap
                    .get(assistantData?.assistant_type ?? 0)
                    ?.onAssistantTypeRun(assistantRunApi);
            } else {
                invoke<AiResponse>("ask_ai", {
                    request: {
                        prompt: inputText,
                        conversation_id: conversationId,
                        assistant_id: +assistantId,
                        attachment_list: fileInfoList?.map((i) => i.id),
                    },
                })
                    .then((res) => {
                        console.log("ask ai response", res);

                        // 如果是新对话，更新对话 ID
                        if (conversationId != res.conversation_id + "") {
                            onChangeConversationId(res.conversation_id + "");
                        }
                    })
                    .catch((error) => {
                        console.error("Send message error:", error);
                        setAiIsResponsing(false);
                        // 使用智能边框控制，而不是直接清空
                        updateShiningMessages();
                        // 错误信息将在对话框中显示
                    });
            }

            setInputText("");
            clearFileInfoList();
        }
    }, 200);

    return {
        // 对话管理
        handleDeleteConversationSuccess,
        
        // 消息操作
        handleMessageRegenerate,
        handleMessageEdit,
        handleEditSave,
        handleEditSaveAndRegenerate,
        handleSend,
        handleArtifact,
        
        // 编辑对话框状态
        editDialogIsOpen,
        editingMessage,
        closeEditDialog,
        
        // 标题编辑
        titleEditDialogIsOpen,
        openTitleEditDialog,
        closeTitleEditDialog,
    };
}