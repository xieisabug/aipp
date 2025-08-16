import React, { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import { Button } from './ui/button';
import { Textarea } from './ui/textarea';

interface MessageEditDialogProps {
    isOpen: boolean;
    initialContent: string;
    messageType: string;
    onClose: () => void;
    onSave: (content: string) => void;
    onSaveAndRegenerate: (content: string) => void;
}

const MessageEditDialog: React.FC<MessageEditDialogProps> = ({
    isOpen,
    initialContent,
    messageType,
    onClose,
    onSave,
    onSaveAndRegenerate,
}) => {
    const [content, setContent] = useState(initialContent);

    useEffect(() => {
        if (isOpen) {
            setContent(initialContent);
        }
    }, [isOpen, initialContent]);

    if (!isOpen) return null;

    const handleSave = () => {
        onSave(content);
        onClose();
    };

    const handleSaveAndRegenerate = () => {
        onSaveAndRegenerate(content);
        onClose();
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* 背景遮罩 */}
            <div
                className="absolute inset-0 bg-black/50 backdrop-blur-sm"
                onClick={onClose}
            />

            {/* 模态框内容 */}
            <div className="relative bg-background rounded-xl shadow-2xl w-[50%] min-w-[32rem] h-[70%] min-h-[28rem] max-w-[90%] max-h-[90%] transform transition-all duration-200 scale-100 flex flex-col">
                {/* 标题栏 */}
                <div className="flex items-center justify-between p-6 border-b border-border">
                    <h2 className="text-xl font-semibold text-foreground">编辑消息</h2>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-muted rounded-lg transition-colors duration-200 flex-shrink-0"
                    >
                        <X className="h-5 w-5 text-muted-foreground" />
                    </button>
                </div>

                {/* 内容区域 */}
                <div className="flex-1 p-6 flex flex-col min-h-0">
                    <Textarea
                        value={content}
                        onChange={(e) => setContent(e.target.value)}
                        className="w-full flex-1 min-h-64 resize-none"
                        placeholder="请输入消息内容..."
                        autoFocus
                    />
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
                        onClick={handleSave}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all"
                    >
                        修改
                    </Button>
                    {messageType === "user" && (
                        <Button
                            onClick={handleSaveAndRegenerate}
                            className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all"
                        >
                            修改并重新生成
                        </Button>
                    )}
                </div>
            </div>
        </div>
    );
};

export default MessageEditDialog;