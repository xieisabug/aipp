import React from 'react';
import { AlertDialog, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from './ui/alert-dialog';
import { Button } from './ui/button';

interface EnvironmentInstallDialogProps {
    tool: string;
    message: string;
    isOpen: boolean;
    onConfirm: () => void;
    onCancel: () => void;
}

const EnvironmentInstallDialog: React.FC<EnvironmentInstallDialogProps> = ({ 
    tool, 
    message, 
    isOpen, 
    onConfirm, 
    onCancel 
}) => {
    if (!isOpen) return null;

    const getToolDisplayName = (tool: string) => {
        switch (tool) {
            case 'bun':
                return 'Bun';
            case 'uv':
                return 'uv';
            default:
                return tool;
        }
    };

    const getToolDescription = (tool: string) => {
        switch (tool) {
            case 'bun':
                return 'Bun 是一个快速的 JavaScript 运行时和包管理器，用于运行 React 和 Vue 组件预览。';
            case 'uv':
                return 'uv 是一个极快的 Python 包管理器，用于管理 Python 项目依赖。';
            default:
                return `${tool} 是预览功能所需的环境工具。`;
        }
    };

    return (
        <AlertDialog open={isOpen} onOpenChange={onCancel}>
            <AlertDialogContent className="max-w-md">
                <AlertDialogHeader>
                    <AlertDialogTitle className="flex items-center gap-2">
                        <span className="text-2xl">📦</span>
                        需要安装 {getToolDisplayName(tool)}
                    </AlertDialogTitle>
                    <AlertDialogDescription className="space-y-3">
                        <p className="text-sm text-gray-600">
                            {message}
                        </p>
                        <div className="bg-gray-50 p-3 rounded-md">
                            <p className="text-xs text-gray-700">
                                {getToolDescription(tool)}
                            </p>
                        </div>
                        <p className="text-xs text-gray-500">
                            安装过程可能需要几分钟时间，请耐心等待。
                        </p>
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <Button 
                        onClick={onCancel} 
                        variant="outline"
                        className="flex-1"
                    >
                        取消预览
                    </Button>
                    <Button 
                        onClick={onConfirm} 
                        className="flex-1 bg-gray-800 hover:bg-gray-900"
                    >
                        自动安装
                    </Button>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
};

export default EnvironmentInstallDialog;