import React, { useState, useCallback, useRef } from 'react';
import { X, Copy, Check, Share2 } from 'lucide-react';
import { Button } from './ui/button';
import { toast } from 'sonner';

interface ShareDialogProps {
    title: string;
    shareCode: string;
    isOpen: boolean;
    onClose: () => void;
}

const ShareDialog: React.FC<ShareDialogProps> = ({ 
    title, 
    shareCode, 
    isOpen, 
    onClose 
}) => {
    const [copied, setCopied] = useState(false);
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const handleCopy = useCallback(async () => {
        try {
            await navigator.clipboard.writeText(shareCode);
            setCopied(true);
            toast.success('分享码已复制到剪贴板');
            setTimeout(() => setCopied(false), 2000);
        } catch (error) {
            // Fallback for older browsers
            if (textareaRef.current) {
                textareaRef.current.select();
                textareaRef.current.setSelectionRange(0, 99999);
                document.execCommand('copy');
                setCopied(true);
                toast.success('分享码已复制到剪贴板');
                setTimeout(() => setCopied(false), 2000);
            } else {
                toast.error('复制失败，请手动复制');
            }
        }
    }, [shareCode]);

    const handleSelectAll = useCallback(() => {
        if (textareaRef.current) {
            textareaRef.current.select();
            textareaRef.current.setSelectionRange(0, 99999);
        }
    }, []);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* 背景遮罩 */}
            <div
                className="absolute inset-0 bg-black/50 backdrop-blur-sm"
                onClick={onClose}
            />

            {/* 模态框内容 */}
            <div className="relative bg-background rounded-xl shadow-2xl w-full max-w-2xl transform transition-all duration-200 scale-100">
                {/* 标题栏 */}
                <div className="flex items-center justify-between p-6 border-b border-border">
                    <div className="flex items-center gap-3">
                        <Share2 className="h-5 w-5 text-primary" />
                        <h2 className="text-xl font-semibold text-foreground">分享 {title}</h2>
                    </div>
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
                        <div>
                            <label className="text-sm font-medium text-foreground mb-2 block">
                                分享码
                            </label>
                            <div className="relative">
                                <textarea
                                    ref={textareaRef}
                                    value={shareCode}
                                    readOnly
                                    onClick={handleSelectAll}
                                    className="w-full h-32 px-3 py-2 pr-12 border border-input rounded-lg bg-muted/20 text-foreground text-sm font-mono resize-none focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors"
                                    placeholder="生成中..."
                                />
                                <Button
                                    size="sm"
                                    variant="ghost"
                                    onClick={handleCopy}
                                    className="absolute top-2 right-2 h-8 w-8 p-0 hover:bg-background/80"
                                >
                                    {copied ? (
                                        <Check className="h-4 w-4 text-green-500" />
                                    ) : (
                                        <Copy className="h-4 w-4" />
                                    )}
                                </Button>
                            </div>
                            <p className="text-xs text-muted-foreground mt-2">
                                点击文本区域全选，或点击复制按钮复制分享码
                            </p>
                        </div>

                        <div className="bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                            <h4 className="text-sm font-medium text-blue-900 dark:text-blue-100 mb-2">
                                分享说明
                            </h4>
                            <ul className="text-xs text-blue-700 dark:text-blue-300 space-y-1">
                                <li>• 将此分享码发送给其他人，他们可以导入相同的配置</li>
                                <li>• 分享码已经过压缩，请完整复制</li>
                                <li>• 如果是Provider分享，接收方需要输入正确的密码才能导入</li>
                            </ul>
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
                        关闭
                    </Button>
                    <Button
                        onClick={handleCopy}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all"
                    >
                        {copied ? '已复制' : '复制分享码'}
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default ShareDialog;