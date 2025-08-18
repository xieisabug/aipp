import React, { useState, useCallback } from 'react';
import { X, Lock, Eye, EyeOff, AlertTriangle } from 'lucide-react';
import { Button } from './ui/button';
import { toast } from 'sonner';

interface PasswordDialogProps {
    title: string;
    isOpen: boolean;
    onClose: () => void;
    onConfirm: (password: string) => Promise<void>;
}

const PasswordDialog: React.FC<PasswordDialogProps> = ({ 
    title, 
    isOpen, 
    onClose, 
    onConfirm 
}) => {
    const [password, setPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [showConfirmPassword, setShowConfirmPassword] = useState(false);
    const [loading, setLoading] = useState(false);

    const isPasswordValid = password.length >= 6;
    const passwordsMatch = password === confirmPassword;
    const canSubmit = isPasswordValid && passwordsMatch && !loading;

    const handleSubmit = useCallback(async () => {
        if (!canSubmit) return;

        setLoading(true);
        try {
            await onConfirm(password);
            
            // 清空表单
            setPassword('');
            setConfirmPassword('');
            onClose();
            toast.success('导出成功');
        } catch (error) {
            toast.error(error instanceof Error ? error.message : '导出失败');
        } finally {
            setLoading(false);
        }
    }, [password, canSubmit, onConfirm, onClose]);

    const handleClose = useCallback(() => {
        if (loading) return;
        setPassword('');
        setConfirmPassword('');
        onClose();
    }, [loading, onClose]);

    const togglePasswordVisibility = useCallback(() => {
        setShowPassword(prev => !prev);
    }, []);

    const toggleConfirmPasswordVisibility = useCallback(() => {
        setShowConfirmPassword(prev => !prev);
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
                        <Lock className="h-5 w-5 text-primary" />
                        <h2 className="text-xl font-semibold text-foreground">设置导出密码</h2>
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
                        <div className="text-sm text-muted-foreground">
                            为了保护 {title} 的安全信息（如API密钥），请设置一个用于加密的密码。
                        </div>

                        {/* 密码输入 */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-foreground">
                                设置密码 *
                            </label>
                            <div className="relative">
                                <input
                                    type={showPassword ? "text" : "password"}
                                    value={password}
                                    onChange={(e) => setPassword(e.target.value)}
                                    placeholder="至少6个字符"
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
                            {password && !isPasswordValid && (
                                <p className="text-xs text-red-500">密码至少需要6个字符</p>
                            )}
                        </div>

                        {/* 确认密码 */}
                        <div className="space-y-2">
                            <label className="text-sm font-medium text-foreground">
                                确认密码 *
                            </label>
                            <div className="relative">
                                <input
                                    type={showConfirmPassword ? "text" : "password"}
                                    value={confirmPassword}
                                    onChange={(e) => setConfirmPassword(e.target.value)}
                                    placeholder="再次输入密码"
                                    disabled={loading}
                                    className="w-full px-3 py-2 pr-10 border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring transition-colors bg-background text-foreground disabled:opacity-50 disabled:cursor-not-allowed"
                                />
                                <button
                                    type="button"
                                    onClick={toggleConfirmPasswordVisibility}
                                    disabled={loading}
                                    className="absolute inset-y-0 right-0 pr-3 flex items-center disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    {showConfirmPassword ? (
                                        <EyeOff className="h-4 w-4 text-muted-foreground" />
                                    ) : (
                                        <Eye className="h-4 w-4 text-muted-foreground" />
                                    )}
                                </button>
                            </div>
                            {confirmPassword && !passwordsMatch && (
                                <p className="text-xs text-red-500">两次输入的密码不一致</p>
                            )}
                        </div>

                        {/* 安全提示 */}
                        <div className="bg-orange-50 dark:bg-orange-950/20 border border-orange-200 dark:border-orange-800 rounded-lg p-4">
                            <div className="flex items-start gap-3">
                                <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400 mt-0.5 flex-shrink-0" />
                                <div className="text-xs text-orange-700 dark:text-orange-300">
                                    <p className="font-medium mb-1">安全提示：</p>
                                    <ul className="space-y-1">
                                        <li>• 请牢记此密码，丢失后无法恢复</li>
                                        <li>• 建议使用强密码，包含字母、数字和特殊字符</li>
                                        <li>• 导入时需要输入相同的密码才能解密</li>
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
                        disabled={!canSubmit}
                        className="px-6 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {loading ? '导出中...' : '确认导出'}
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default PasswordDialog;