import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Sparkles, X } from 'lucide-react';
import { Button } from './ui/button';

interface ConversationTitleEditDialogProps {
    isOpen: boolean;
    conversationId: number;
    initialTitle: string;
    onClose: () => void;
    onSave?: (title: string) => void; // 可选的保存回调
}

const ConversationTitleEditDialog: React.FC<ConversationTitleEditDialogProps> = ({
    isOpen,
    conversationId,
    initialTitle,
    onClose,
    onSave,
}) => {
    const [title, setTitle] = useState<string>("");
    const [isRegeneratingTitle, setIsRegeneratingTitle] = useState<boolean>(false);

    // 当对话框打开时，同步初始标题
    useEffect(() => {
        if (isOpen) {
            setTitle(initialTitle || "");
        }
    }, [isOpen, initialTitle]);

    // 监听标题变化事件，实时更新输入框内容
    useEffect(() => {
        if (!isOpen || !conversationId) return;

        const unsubscribe = listen("title_change", (event) => {
            const [eventConversationId, newTitle] = event.payload as [number, string];
            
            // 只更新当前正在编辑的对话的标题
            if (eventConversationId === conversationId) {
                setTitle(newTitle);
            }
        });

        return () => {
            unsubscribe.then((f) => f());
        };
    }, [isOpen, conversationId]);

    // 重新生成标题处理
    const handleRegenerateTitle = useCallback(async () => {
        if (!conversationId || isRegeneratingTitle) return;

        setIsRegeneratingTitle(true);

        try {
            await invoke("regenerate_conversation_title", {
                conversationId: conversationId,
            });
            toast.success("标题已重新生成");
        } catch (error) {
            console.error("重新生成标题失败:", error);
            
            // 处理特定的错误类型，提供更友好的提示
            if (error === "InsufficientMessages") {
                toast.error("对话内容不足，需要至少包含一条用户消息才能生成标题");
            } else {
                toast.error("重新生成标题失败: " + error);
            }
        } finally {
            setIsRegeneratingTitle(false);
        }
    }, [conversationId, isRegeneratingTitle]);

    // 提交表单处理
    const handleSubmit = useCallback(() => {
        invoke("update_conversation", {
            conversationId: conversationId,
            name: title,
        }).then(() => {
            if (onSave) {
                onSave(title);
            }
            onClose();
            toast.success("标题已更新");
        }).catch((error) => {
            toast.error("更新标题失败: " + error);
        });
    }, [conversationId, title, onSave, onClose]);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* 背景遮罩 */}
            <div
                className="absolute inset-0 bg-black/50 backdrop-blur-sm"
                onClick={onClose}
            />

            {/* 模态框内容 */}
            <div className="relative bg-background rounded-xl shadow-2xl w-full max-w-md transform transition-all duration-200 scale-100">
                {/* 标题栏 */}
                <div className="flex items-center justify-between p-6 border-b border-border">
                    <h2 className="text-xl font-semibold text-foreground truncate pr-4">修改对话标题</h2>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-muted rounded-lg transition-colors duration-200 flex-shrink-0"
                    >
                        <X className="h-5 w-5 text-muted-foreground" />
                    </button>
                </div>

                {/* 内容区域 */}
                <div className="p-6">
                    <div className="space-y-4">
                        <div className="space-y-2">
                            <div className="flex items-center justify-between">
                                <label className="text-sm font-medium leading-none text-foreground">
                                    标题
                                </label>
                                <button
                                    type="button"
                                    onClick={handleRegenerateTitle}
                                    disabled={isRegeneratingTitle}
                                    className="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 hover:bg-accent hover:text-accent-foreground h-8 px-2 py-1"
                                    title="重新生成标题"
                                >
                                    <Sparkles className={`h-4 w-4 ${isRegeneratingTitle ? 'animate-pulse' : ''}`} />
                                </button>
                            </div>
                            <input
                                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 transition-colors"
                                type="text"
                                name="name"
                                value={title}
                                onChange={(e) => setTitle(e.target.value)}
                                placeholder="请输入对话标题"
                                autoFocus
                            />
                        </div>
                    </div>
                </div>

                {/* 按钮区域 */}
                <div className="flex justify-end gap-3 p-6 pt-0">
                    <Button
                        variant="outline"
                        onClick={onClose}
                        className="px-6"
                    >
                        取消
                    </Button>
                    <Button
                        onClick={handleSubmit}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all"
                    >
                        确认
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default ConversationTitleEditDialog;