// FormDialog.tsx
import React from 'react';
import { X } from 'lucide-react';
import { Button } from './ui/button';

interface FormDialogProps {
    title: string;
    onSubmit: () => void;
    onClose: () => void;
    isOpen: boolean;
    children: React.ReactNode;
}

const FormDialog: React.FC<FormDialogProps> = ({ title, onSubmit, onClose, isOpen, children }) => {
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
                    <h2 className="text-xl font-semibold text-foreground truncate pr-4">{title}</h2>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-muted rounded-lg transition-colors duration-200 flex-shrink-0"
                    >
                        <X className="h-5 w-5 text-muted-foreground" />
                    </button>
                </div>

                {/* 内容区域 */}
                <div className="p-6">
                    {children}
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
                        onClick={onSubmit}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all"
                    >
                        确认
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default FormDialog;