import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { X, ChevronDown, ChevronRight } from 'lucide-react';
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { Textarea } from '../ui/textarea';
import CustomSelect from '../CustomSelect';
import { toast } from 'sonner';
import { MCPServer, MCPServerRequest, MCP_TRANSPORT_TYPES } from '../../data/MCP';

interface MCPServerDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: () => void;
    editingServer?: MCPServer | null;
}

const MCPServerDialog: React.FC<MCPServerDialogProps> = ({
    isOpen,
    onClose,
    onSubmit,
    editingServer
}) => {
    // Form state
    const [formData, setFormData] = useState<MCPServerRequest>({
        name: '',
        description: '',
        transport_type: 'stdio',
        command: '',
        environment_variables: '',
        url: '',
        timeout: 30000,
        is_long_running: false,
        is_enabled: true,
    });

    // UI state
    const [advancedOpen, setAdvancedOpen] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    // Initialize form when editing
    useEffect(() => {
        if (editingServer) {
            setFormData({
                name: editingServer.name,
                description: editingServer.description || '',
                transport_type: editingServer.transport_type,
                command: editingServer.command || '',
                environment_variables: editingServer.environment_variables || '',
                url: editingServer.url || '',
                timeout: editingServer.timeout || 30000,
                is_long_running: editingServer.is_long_running,
                is_enabled: editingServer.is_enabled,
            });
        } else {
            // Reset form for new server
            setFormData({
                name: '',
                description: '',
                transport_type: 'stdio',
                command: '',
                environment_variables: '',
                url: '',
                timeout: 30000,
                is_long_running: false,
                is_enabled: true,
            });
        }
        setAdvancedOpen(false);
    }, [editingServer, isOpen]);

    // Update form field
    const updateField = useCallback((field: keyof MCPServerRequest, value: any) => {
        setFormData(prev => ({
            ...prev,
            [field]: value
        }));
    }, []);

    // Handle form submission
    const handleSubmit = useCallback(async () => {
        // Validation
        if (!formData.name.trim()) {
            toast.error('请输入MCP服务器名称');
            return;
        }

        if (formData.transport_type === 'stdio' && !formData.command?.trim()) {
            toast.error('Stdio类型需要提供命令');
            return;
        }

        if ((formData.transport_type === 'sse' || formData.transport_type === 'http') && !formData.url?.trim()) {
            toast.error('SSE/HTTP类型需要提供URL');
            return;
        }

        setIsSubmitting(true);
        
        try {
            let serverId: number;
            
            if (editingServer) {
                await invoke('update_mcp_server', {
                    id: editingServer.id,
                    request: formData
                });
                serverId = editingServer.id;
                toast.success('更新MCP服务器成功');
            } else {
                serverId = await invoke<number>('add_mcp_server', {
                    request: formData
                });
                toast.success('添加MCP服务器成功');
                
                // 新增服务器后自动获取能力
                try {
                    await invoke('refresh_mcp_server_capabilities', { serverId });
                    toast.success('自动获取服务器能力成功');
                } catch (e) {
                    console.warn('自动获取能力失败:', e);
                    toast.warning('服务器添加成功，但获取能力失败，请手动刷新');
                }
            }
            
            onSubmit();
        } catch (e) {
            toast.error(`${editingServer ? '更新' : '添加'}MCP服务器失败: ${e}`);
        } finally {
            setIsSubmitting(false);
        }
    }, [formData, editingServer, onSubmit]);

    // Handle cancel
    const handleCancel = useCallback(() => {
        onClose();
    }, [onClose]);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* 背景遮罩 */}
            <div
                className="absolute inset-0 bg-black/50 backdrop-blur-sm"
                onClick={handleCancel}
            />

            {/* 模态框内容 */}
            <div className="relative bg-white rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden transform transition-all duration-200 scale-100">
                {/* 标题栏 */}
                <div className="flex items-center justify-between p-6 border-b border-gray-200">
                    <h2 className="text-xl font-semibold text-gray-900">
                        {editingServer ? '编辑MCP服务器' : '新增MCP服务器'}
                    </h2>
                    <button
                        onClick={handleCancel}
                        className="p-2 hover:bg-gray-100 rounded-lg transition-colors duration-200 flex-shrink-0"
                    >
                        <X className="h-5 w-5 text-gray-500" />
                    </button>
                </div>

                {/* 内容区域 */}
                <div className="overflow-y-auto max-h-[calc(90vh-140px)]">
                    <div className="p-6 space-y-6">
                        {/* 基本信息 */}
                        <div className="space-y-4">
                            <div className="space-y-2">
                                <label className="text-sm font-medium text-gray-700">名称 *</label>
                                <input
                                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors"
                                    type="text"
                                    placeholder="例如：文件搜索服务"
                                    value={formData.name}
                                    onChange={(e) => updateField('name', e.target.value)}
                                />
                            </div>

                            <div className="space-y-2">
                                <label className="text-sm font-medium text-gray-700">描述</label>
                                <Textarea
                                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors resize-none"
                                    placeholder="服务器功能描述..."
                                    rows={3}
                                    value={formData.description}
                                    onChange={(e) => updateField('description', e.target.value)}
                                />
                            </div>

                            <div className="space-y-2">
                                <label className="text-sm font-medium text-gray-700">MCP类型 *</label>
                                <CustomSelect
                                    options={MCP_TRANSPORT_TYPES}
                                    value={formData.transport_type}
                                    onChange={(value) => updateField('transport_type', value)}
                                />
                            </div>

                            {/* Stdio specific fields */}
                            {formData.transport_type === 'stdio' && (
                                <div className="space-y-2">
                                    <label className="text-sm font-medium text-gray-700">命令 *</label>
                                    <Textarea
                                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors resize-none"
                                        placeholder="例如：npx @modelcontextprotocol/server-filesystem /path/to/directory"
                                        rows={3}
                                        value={formData.command}
                                        onChange={(e) => updateField('command', e.target.value)}
                                    />
                                </div>
                            )}

                            {/* SSE/HTTP specific fields */}
                            {(formData.transport_type === 'sse' || formData.transport_type === 'http') && (
                                <div className="space-y-2">
                                    <label className="text-sm font-medium text-gray-700">URL *</label>
                                    <input
                                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors"
                                        type="url"
                                        placeholder="例如：http://localhost:3000/mcp"
                                        value={formData.url}
                                        onChange={(e) => updateField('url', e.target.value)}
                                    />
                                </div>
                            )}

                            <div className="space-y-2">
                                <label className="text-sm font-medium text-gray-700">环境变量</label>
                                <Textarea
                                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors resize-none"
                                    placeholder="KEY1=value1&#10;KEY2=value2"
                                    rows={4}
                                    value={formData.environment_variables}
                                    onChange={(e) => updateField('environment_variables', e.target.value)}
                                />
                            </div>
                        </div>

                        {/* 高级设置 */}
                        <div>
                            <button
                                type="button"
                                onClick={() => setAdvancedOpen(!advancedOpen)}
                                className="flex items-center gap-2 text-sm font-medium text-gray-700 hover:text-gray-900 transition-colors"
                            >
                                {advancedOpen ? (
                                    <ChevronDown className="h-4 w-4" />
                                ) : (
                                    <ChevronRight className="h-4 w-4" />
                                )}
                                高级设置
                            </button>

                            {advancedOpen && (
                                <div className="mt-4 space-y-4 pl-6 border-l-2 border-gray-200">
                                    <div className="space-y-2">
                                        <label className="text-sm font-medium text-gray-700">请求超时 (毫秒)</label>
                                        <input
                                            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 focus:border-gray-500 transition-colors"
                                            type="number"
                                            min="1000"
                                            max="300000"
                                            step="1000"
                                            value={formData.timeout || 30000}
                                            onChange={(e) => updateField('timeout', parseInt(e.target.value) || 30000)}
                                        />
                                    </div>

                                    <div className="flex items-center justify-between">
                                        <div>
                                            <label className="text-sm font-medium text-gray-700">是否长期运行</label>
                                            <p className="text-xs text-gray-500 mt-1">长期运行的服务器会保持连接状态</p>
                                        </div>
                                        <Switch
                                            checked={formData.is_long_running}
                                            onCheckedChange={(checked) => updateField('is_long_running', checked)}
                                        />
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                </div>

                {/* 按钮区域 */}
                <div className="flex justify-end gap-3 p-6 pt-0 border-t border-gray-200">
                    <Button
                        variant="outline"
                        onClick={handleCancel}
                        disabled={isSubmitting}
                        className="px-6"
                    >
                        取消
                    </Button>
                    <Button
                        onClick={handleSubmit}
                        disabled={isSubmitting}
                        className="px-6 bg-gray-800 hover:bg-gray-900 text-white shadow-md hover:shadow-lg transition-all"
                    >
                        {isSubmitting ? '保存中...' : '确定'}
                    </Button>
                </div>
            </div>
        </div>
    );
};

export default MCPServerDialog;