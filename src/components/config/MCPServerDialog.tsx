import React, { useState, useEffect, useCallback } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { Textarea } from '../ui/textarea';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '../ui/accordion';
import { toast } from 'sonner';
import { MCPServer, MCPServerRequest, MCP_TRANSPORT_TYPES } from '../../data/MCP';
import CustomSelect from '../CustomSelect';

interface MCPServerDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: () => void;
    editingServer?: MCPServer | null;
    initialServerType?: string;
    initialConfig?: Partial<MCPServerRequest>;
}

const MCPServerDialog: React.FC<MCPServerDialogProps> = ({
    isOpen,
    onClose,
    onSubmit,
    editingServer,
    initialServerType,
    initialConfig
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
    const [isSubmitting, setIsSubmitting] = useState(false);

    // 初始化界面数据
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
            // 重置表单，使用可选的初始配置
            const defaultConfig: MCPServerRequest = {
                name: '',
                description: '',
                transport_type: initialServerType || 'stdio',
                command: '',
                environment_variables: '',
                url: '',
                timeout: 30000,
                is_long_running: false,
                is_enabled: true,
            };

            // 合并初始配置
            const finalConfig = initialConfig ? { ...defaultConfig, ...initialConfig } : defaultConfig;
            
            setFormData(finalConfig);
        }
    }, [editingServer, isOpen, initialServerType, initialConfig]);

    // 更新表单字段
    const updateField = useCallback((field: keyof MCPServerRequest, value: any) => {
        setFormData(prev => ({
            ...prev,
            [field]: value
        }));
    }, []);

    // 处理表单提交
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

    // 处理取消
    const handleCancel = useCallback(() => {
        onClose();
    }, [onClose]);

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && handleCancel()}>
            <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                    <DialogTitle>
                        {editingServer ? '编辑MCP服务器' : '新增MCP服务器'}
                    </DialogTitle>
                </DialogHeader>

                <div className="space-y-6 py-4">
                    {/* 基本信息 */}
                    <div className="space-y-4">
                        <div className="space-y-2">
                            <Label htmlFor="name">ID *</Label>
                            <Input
                                id="name"
                                placeholder="例如：fetch-mcp"
                                value={formData.name}
                                onChange={(e) => updateField('name', e.target.value)}
                            />
                        </div>

                        <div className="space-y-2">
                            <Label htmlFor="description">描述</Label>
                            <Textarea
                                id="description"
                                placeholder="MCP功能描述..."
                                rows={2}
                                value={formData.description}
                                onChange={(e) => updateField('description', e.target.value)}
                            />
                        </div>


                        <div className="space-y-2">
                            <label className="text-sm font-medium text-foreground">MCP类型 *</label>
                            <CustomSelect
                                options={MCP_TRANSPORT_TYPES}
                                value={formData.transport_type}
                                onChange={(value) => updateField('transport_type', value)}
                            />
                        </div>


                        {/* Stdio specific fields */}
                        {formData.transport_type === 'stdio' && (
                            <div className="space-y-2">
                                <Label htmlFor="command">命令 *</Label>
                                <Textarea
                                    id="command"
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
                                <Label htmlFor="url">URL *</Label>
                                <Input
                                    id="url"
                                    type="url"
                                    placeholder="例如：http://localhost:3000/mcp"
                                    value={formData.url}
                                    onChange={(e) => updateField('url', e.target.value)}
                                />
                            </div>
                        )}

                        <div className="space-y-2">
                            <Label htmlFor="environment_variables">环境变量</Label>
                            <Textarea
                                id="environment_variables"
                                placeholder="KEY1=value1&#10;KEY2=value2"
                                rows={3}
                                value={formData.environment_variables}
                                onChange={(e) => updateField('environment_variables', e.target.value)}
                            />
                        </div>
                    </div>

                    {/* 高级设置 */}
                    <Accordion type="single" collapsible>
                        <AccordionItem value="advanced">
                            <AccordionTrigger>高级设置</AccordionTrigger>
                            <AccordionContent className="space-y-4">
                                <div className="space-y-2">
                                    <Label htmlFor="timeout">请求超时 (毫秒)</Label>
                                    <Input
                                        id="timeout"
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
                                        <Label>是否长期运行</Label>
                                        <p className="text-sm text-muted-foreground mt-1">长期运行的服务器会保持连接状态</p>
                                    </div>
                                    <Switch
                                        checked={formData.is_long_running}
                                        onCheckedChange={(checked) => updateField('is_long_running', checked)}
                                    />
                                </div>
                            </AccordionContent>
                        </AccordionItem>
                    </Accordion>
                </div>

                <DialogFooter>
                    <Button
                        variant="outline"
                        onClick={handleCancel}
                        disabled={isSubmitting}
                    >
                        取消
                    </Button>
                    <Button
                        onClick={handleSubmit}
                        disabled={isSubmitting}
                    >
                        {isSubmitting ? '保存中...' : '确定'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default MCPServerDialog;