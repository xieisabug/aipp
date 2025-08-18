import React, { useState, useCallback } from 'react';
import { X, Download, AlertCircle, Eye, EyeOff } from 'lucide-react';
import { Button } from './ui/button';
import { toast } from 'sonner';

interface ImportDialogProps {
    title: string;
    isOpen: boolean;
    requiresPassword?: boolean;
    onClose: () => void;
    onImport: (shareCode: string, password?: string, newName?: string) => Promise<void>;
}

const ImportDialog: React.FC<ImportDialogProps> = ({ 
    title, 
    isOpen, 
    requiresPassword = false,
    onClose, 
    onImport 
}) => {
    const [shareCode, setShareCode] = useState('');
    const [password, setPassword] = useState('');
    const [newName, setNewName] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [loading, setLoading] = useState(false);

    const handleSubmit = useCallback(async () => {
        if (!shareCode.trim()) {
            toast.error('请输入分享码');
            return;
        }

        if (requiresPassword && !password.trim()) {
            toast.error('请输入密码');
            return;
        }

        setLoading(true);
        try {
            await onImport(
                shareCode.trim(), 
                password.trim() || undefined, 
                newName.trim() || undefined
            );
            
            // 清空表单
            setShareCode('');
            setPassword('');
            setNewName('');
            onClose();
            toast.success('导入成功');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : '导入失败');
        } finally {
            setLoading(false);
        }
    }, [shareCode, password, newName, requiresPassword, onImport, onClose]);

    const handleClose = useCallback(() => {
        if (loading) return;
        setShareCode('');
        setPassword('');
        setNewName('');
        onClose();
    }, [loading, onClose]);

    const togglePasswordVisibility = useCallback(() => {
        setShowPassword(prev => !prev);
    }, []);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* 背景遮罩 */}
            <div
                className="absolute inset-0 bg-black/50 backdrop-blur-sm"
                onClick={handleClose}
            />

            {/* 模态框内容 */}
            <div className="relative bg-background rounded-xl shadow-2xl w-full max-w-md transform transition-all duration-200 scale-100">
                {/* 标题栏 */}
                <div className="flex items-center justify-between p-6 border-b border-border">
                    <div className="flex items-center gap-3">
                        <Download className="h-5 w-5 text-primary" />
                        <h2 className="text-xl font-semibold text-foreground">导入 {title}</h2>
                    </div>
                    <button
                        onClick={handleClose}
                        disabled={loading}
                        className="p-2 hover:bg-muted rounded-lg transition-colors duration-200 flex-shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <X className="h-5 w-5 text-muted-foreground" />
                    </button>
                </div>

                {/* 内容区域 */}
                <div className="p-6">
                    <div className="space-y-5">
                        {/* 分享码输入 */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-foreground">
                                分享码 *
                            </label>
                            <textarea
                                value={shareCode}
                                onChange={(e) => setShareCode(e.target.value)}
                                placeholder="请粘贴分享码..."
                                disabled={loading}
                                className="w-full h-24 px-3 py-2 border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors bg-background text-foreground resize-none disabled:opacity-50 disabled:cursor-not-allowed"
                            />
                        </div>

                        {/* 密码输入（如果需要） */}
                        {requiresPassword && (
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-foreground">
                                    密码 *
                                </label>
                                <div className="relative">
                                    <input
                                        type={showPassword ? "text" : "password"}
                                        value={password}
                                        onChange={(e) => setPassword(e.target.value)}
                                        placeholder="请输入解密密码"
                                        disabled={loading}
                                        className="w-full px-3 py-2 pr-10 border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors bg-background text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
                                    />
                                    <button
                                        type="button"
                                        onClick={togglePasswordVisibility}
                                        disabled={loading}
                                        className="absolute inset-y-0 right-0 pr-3 flex items-center disabled:opacity-50 disabled:cursor-not-allowed"
                                    >
                                        {showPassword ? (
                                            <EyeOff className="h-4 w-4 text-muted-foreground" />
                                        ) : (
                                            <Eye className="h-4 w-4 text-muted-foreground" />
                                        )}
                                    </button>
                                </div>
                            </div>
                        )}

                        {/* 自定义名称 */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-foreground">
                                自定义名称（可选）
                            </label>
                            <input
                                type="text"
                                value={newName}
                                onChange={(e) => setNewName(e.target.value)}
                                placeholder="留空则使用默认名称"
                                disabled={loading}
                                className="w-full px-3 py-2 border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors bg-background text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
                            />
                        </div>

                        {/* 提示信息 */}
                        <div className="bg-yellow-50 dark:bg-yellow-950/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-4">
                            <div className="flex items-start gap-3">
                                <AlertCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-400 mt-0.5 flex-shrink-0" />
                                <div className="text-xs text-yellow-700 dark:text-yellow-300">
                                    <p className="font-medium mb-1">导入说明：</p>
                                    <ul className="space-y-1">
                                        <li>• 请确保分享码完整且正确</li>
                                        {requiresPassword && <li>• Provider配置需要输入正确的密码</li>}
                                        <li>• 导入的配置会作为新项目添加，不会覆盖现有配置</li>
                                    </ul>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                {/* 按钮区域 */}
                <div className="flex justify-end gap-3 p-6 pt-0">
                    <Button
                        variant="outline"
                        onClick={handleClose}
                        disabled={loading}
                        className="px-6"
                    >
                        取消
                    </Button>
                    <Button
                        onClick={handleSubmit}
                        disabled={loading || !shareCode.trim()}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {loading ? '导入中...' : '确认导入'}
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default ImportDialog;