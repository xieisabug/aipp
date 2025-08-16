import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../ui/collapsible";
import { Server, Wrench, MoreHorizontal, Play, Pause, ChevronDown, ChevronRight, Settings2, Info } from "lucide-react";
import { toast } from 'sonner';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from "../ui/tooltip";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "../ui/dialog";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "../ui/dropdown-menu";

interface AssistantMCPConfigDialogProps {
    assistantId: number;
    isOpen: boolean;
    onClose: () => void;
    onConfigChange?: () => void;
}

interface MCPServerInfo {
    id: number;
    name: string;
    is_enabled: boolean;
}

interface MCPToolInfo {
    id: number;
    name: string;
    is_enabled: boolean;
    is_auto_run: boolean;
}

const AssistantMCPConfigDialog: React.FC<AssistantMCPConfigDialogProps> = ({
    assistantId,
    isOpen,
    onClose,
    onConfigChange
}) => {
    const [availableServers, setAvailableServers] = useState<MCPServerInfo[]>([]);
    const [expandedServers, setExpandedServers] = useState<Set<number>>(new Set());
    const [serverTools, setServerTools] = useState<Map<number, MCPToolInfo[]>>(new Map());
    const [useNativeToolCall, setUseNativeToolCall] = useState<boolean>(false);
    // loadingTools 不再需要，因为工具数据在初始化时一次性加载

    // 获取可用的MCP服务器列表
    const fetchAvailableServers = useCallback(async () => {
        if (!isOpen) return;

        try {
            const serversWithTools = await invoke<(MCPServerInfo & { tools: MCPToolInfo[] })[]>(
                'get_assistant_mcp_servers_with_tools',
                { assistantId }
            );

            // 获取原生ToolCall配置
            try {
                const nativeToolCallValue = await invoke<string>('get_assistant_field_value', {
                    assistantId,
                    fieldName: 'use_native_toolcall'
                });
                setUseNativeToolCall(nativeToolCallValue === 'true');
            } catch (error) {
                // 如果没有找到配置，默认为false
                setUseNativeToolCall(false);
            }

            // 提取服务器信息
            const servers = serversWithTools.map(server => ({
                id: server.id,
                name: server.name,
                is_enabled: server.is_enabled
            }));
            setAvailableServers(servers);

            // 设置所有服务器的工具映射
            const toolsMap = new Map<number, MCPToolInfo[]>();
            serversWithTools.forEach(server => {
                toolsMap.set(server.id, server.tools);
            });
            setServerTools(toolsMap);
        } catch (error) {
            console.error('Failed to fetch available servers:', error);
            toast.error('获取MCP服务器列表失败: ' + error);
        }
    }, [assistantId, isOpen]);

    // fetchServerTools 不再需要，因为工具数据在 fetchAvailableServers 中一次性加载

    // 更新服务器启用状态
    const handleServerToggle = useCallback(async (serverId: number, isEnabled: boolean) => {
        try {
            await invoke('update_assistant_mcp_config', {
                assistantId,
                mcpServerId: serverId,
                isEnabled
            });

            setAvailableServers(prev =>
                prev.map(server =>
                    server.id === serverId ? { ...server, is_enabled: isEnabled } : server
                )
            );

            toast.success(`服务器已${isEnabled ? '启用' : '禁用'}`);
            onConfigChange?.();
        } catch (error) {
            console.error('Failed to update server config:', error);
            toast.error('更新服务器配置失败: ' + error);
        }
    }, [assistantId, onConfigChange]);

    // 更新工具配置
    const handleToolConfigChange = useCallback(async (
        toolId: number,
        isEnabled: boolean,
        isAutoRun: boolean,
        serverId: number
    ) => {
        try {
            await invoke('update_assistant_mcp_tool_config', {
                assistantId,
                mcpToolId: toolId,
                isEnabled,
                isAutoRun
            });

            setServerTools(prev => {
                const newMap = new Map(prev);
                const tools = newMap.get(serverId) || [];
                const updatedTools = tools.map(tool =>
                    tool.id === toolId ? { ...tool, is_enabled: isEnabled, is_auto_run: isAutoRun } : tool
                );
                newMap.set(serverId, updatedTools);
                
                // 检查是否所有工具都被禁用了，如果是则自动禁用服务器
                const hasEnabledTools = updatedTools.some(tool => tool.is_enabled);
                if (!hasEnabledTools) {
                    const server = availableServers.find(s => s.id === serverId);
                    if (server && server.is_enabled) {
                        // 自动禁用服务器
                        handleServerToggle(serverId, false).then(() => {
                            toast.success('工具全部禁用，服务器已自动禁用');
                        });
                    }
                }
                
                return newMap;
            });

            onConfigChange?.();
        } catch (error) {
            console.error('Failed to update tool config:', error);
            toast.error('更新工具配置失败: ' + error);
        }
    }, [assistantId, availableServers, handleServerToggle, onConfigChange]);

    // 更新原生ToolCall配置
    const handleNativeToolCallToggle = useCallback(async (checked: boolean) => {
        try {
            await invoke('update_assistant_model_config_value', {
                assistantId,
                configName: 'use_native_toolcall',
                configValue: checked.toString(),
                valueType: 'boolean'
            });

            setUseNativeToolCall(checked);
            toast.success(`原生ToolCall已${checked ? '启用' : '禁用'}`);
            onConfigChange?.();
        } catch (error) {
            console.error('Failed to update native toolcall config:', error);
            toast.error('更新原生ToolCall配置失败: ' + error);
        }
    }, [assistantId, onConfigChange]);

    // 批量更新工具
    const handleBulkUpdateTools = useCallback(async (
        serverId: number,
        isEnabled: boolean,
        isAutoRun?: boolean
    ) => {
        try {
            await invoke('bulk_update_assistant_mcp_tools', {
                assistantId,
                mcpServerId: serverId,
                isEnabled,
                isAutoRun: isAutoRun
            });

            // 重新获取所有数据
            await fetchAvailableServers();
            toast.success(`批量${isEnabled ? '启用' : '禁用'}工具成功`);
            onConfigChange?.();
        } catch (error) {
            console.error('Failed to bulk update tools:', error);
            toast.error('批量更新工具失败: ' + error);
        }
    }, [assistantId, fetchAvailableServers, onConfigChange]);

    // 处理服务器展开/折叠
    const handleServerExpand = useCallback((serverId: number) => {
        const isExpanded = expandedServers.has(serverId);

        if (isExpanded) {
            setExpandedServers(prev => {
                const newSet = new Set(prev);
                newSet.delete(serverId);
                return newSet;
            });
        } else {
            setExpandedServers(prev => new Set(prev).add(serverId));
            // 在新的实现中，工具已经在初始化时加载了，不需要额外加载
        }
    }, [expandedServers, serverTools]);

    useEffect(() => {
        fetchAvailableServers();
    }, [fetchAvailableServers]);

    const enabledServers = availableServers.filter(server => server.is_enabled);
    
    // 统计有效启用的工具数量：只有服务器启用时，其工具才算有效启用
    const totalEnabledTools = Array.from(serverTools.entries())
        .filter(([serverId]) => availableServers.find(s => s.id === serverId)?.is_enabled)
        .flatMap(([, tools]) => tools)
        .filter(tool => tool.is_enabled).length;

    return (
        <Dialog open={isOpen} onOpenChange={onClose}>
            <DialogContent className="max-w-4xl max-h-[80vh] overflow-hidden flex flex-col">
                <DialogHeader>
                    <div className="flex items-center justify-between">
                        <div>
                            <DialogTitle className="flex items-center gap-2">
                                <Settings2 className="h-5 w-5" />
                                MCP工具配置
                            </DialogTitle>
                            <DialogDescription>
                                为该助手配置可用的MCP服务器和工具 ({enabledServers.length}个服务器，{totalEnabledTools}个工具已启用)
                            </DialogDescription>
                        </div>
                    </div>
                </DialogHeader>

                {/* 原生ToolCall设置 */}
                <div className="border-b pb-4 mb-4">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                            <span className="text-sm font-medium text-foreground">使用原生ToolCall</span>
                            <TooltipProvider>
                                <Tooltip>
                                    <TooltipTrigger asChild>
                                        <Info className="h-4 w-4 text-muted-foreground cursor-help" />
                                    </TooltipTrigger>
                                    <TooltipContent>
                                        <p className="max-w-xs text-xs">
                                            如果模型支持并且模型能力够强，推荐使用原生Toolcall调用工具更加准确
                                        </p>
                                    </TooltipContent>
                                </Tooltip>
                            </TooltipProvider>
                        </div>
                        <Switch
                            checked={useNativeToolCall}
                            onCheckedChange={handleNativeToolCallToggle}
                        />
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                        {useNativeToolCall ? '已启用原生ToolCall调用' : '使用传统prompt方式调用工具'}
                    </p>
                </div>

                <div className="flex-1 overflow-auto">
                    {availableServers.length === 0 ? (
                        <div className="text-center py-8">
                            <Server className="h-12 w-12 text-muted-foreground mx-auto mb-4" />
                            <p className="text-sm text-muted-foreground mb-2">暂无可用的MCP服务器</p>
                            <p className="text-xs text-muted-foreground">请先在MCP配置中添加服务器</p>
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {availableServers.map(server => {
                                const isExpanded = expandedServers.has(server.id);
                                const serverToolsList = serverTools.get(server.id) || [];
                                const isLoadingTools = false; // 不再需要加载状态，因为数据已预加载
                                const enabledToolsCount = serverToolsList.filter(t => t.is_enabled).length;

                                return (
                                    <Collapsible key={server.id} open={isExpanded} onOpenChange={() => handleServerExpand(server.id)}>
                                        <div className={`border rounded-lg transition-colors ${server.is_enabled
                                            ? 'border-border bg-background'
                                            : 'border-border bg-muted'
                                            }`}>
                                            <CollapsibleTrigger asChild>
                                                <div className="flex items-center justify-between p-4 cursor-pointer hover:bg-muted/50">
                                                    <div className="flex items-center gap-3">
                                                        <div className="flex items-center gap-2">
                                                            {isExpanded ? (
                                                                <ChevronDown className="h-4 w-4 text-muted-foreground" />
                                                            ) : (
                                                                <ChevronRight className="h-4 w-4 text-muted-foreground" />
                                                            )}
                                                            {server.is_enabled ? (
                                                                <Play className="h-5 w-5 text-foreground" />
                                                            ) : (
                                                                <Pause className="h-5 w-5 text-muted-foreground" />
                                                            )}
                                                        </div>
                                                        <div>
                                                            <div className="font-medium text-foreground">{server.name}</div>
                                                            <div className="text-sm text-muted-foreground">
                                                                {server.is_enabled ? '已启用' : '已禁用'}
                                                                {serverToolsList.length > 0 && ` • ${enabledToolsCount}/${serverToolsList.length} 工具已启用`}
                                                            </div>
                                                        </div>
                                                    </div>
                                                    <div className="flex items-center gap-3">
                                                        {server.is_enabled && (
                                                            <DropdownMenu>
                                                                <DropdownMenuTrigger asChild>
                                                                    <Button
                                                                        variant="ghost"
                                                                        size="sm"
                                                                        onClick={(e) => e.stopPropagation()}
                                                                    >
                                                                        <MoreHorizontal className="h-4 w-4" />
                                                                    </Button>
                                                                </DropdownMenuTrigger>
                                                                <DropdownMenuContent>
                                                                    <DropdownMenuItem
                                                                        onClick={() => handleBulkUpdateTools(server.id, true)}
                                                                    >
                                                                        启用所有工具
                                                                    </DropdownMenuItem>
                                                                    <DropdownMenuItem
                                                                        onClick={() => handleBulkUpdateTools(server.id, false)}
                                                                    >
                                                                        禁用所有工具
                                                                    </DropdownMenuItem>
                                                                </DropdownMenuContent>
                                                            </DropdownMenu>
                                                        )}
                                                        <Switch
                                                            checked={server.is_enabled}
                                                            onCheckedChange={(checked) => handleServerToggle(server.id, checked)}
                                                            onClick={(e) => e.stopPropagation()}
                                                        />
                                                    </div>
                                                </div>
                                            </CollapsibleTrigger>

                                            <CollapsibleContent>
                                                <div className="px-4 pb-4 border-t border-border">
                                                    {isLoadingTools ? (
                                                        <div className="text-center py-6">
                                                            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-foreground mx-auto"></div>
                                                            <p className="text-sm text-muted-foreground mt-2">加载工具列表...</p>
                                                        </div>
                                                    ) : serverToolsList.length === 0 ? (
                                                        <div className="text-center py-6">
                                                            <Wrench className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
                                                            <p className="text-sm text-muted-foreground">该服务器暂无可用工具</p>
                                                        </div>
                                                    ) : (
                                                        <div className="space-y-3 mt-3">
                                                            {serverToolsList.map(tool => (
                                                                <div key={tool.id} className="flex items-center justify-between p-3 bg-background rounded border border-border">
                                                                    <div className="flex items-center gap-3">
                                                                        <Wrench className="h-4 w-4 text-muted-foreground" />
                                                                        <div>
                                                                            <div className="font-medium text-foreground">{tool.name}</div>
                                                                            <div className="text-sm text-muted-foreground">
                                                                                {tool.is_enabled ? (
                                                                                    <span className="text-foreground">已启用</span>
                                                                                ) : (
                                                                                    <span className="text-muted-foreground">已禁用</span>
                                                                                )}
                                                                                {tool.is_enabled && (
                                                                                    <span> • 自动运行: {tool.is_auto_run ? '是' : '否'}</span>
                                                                                )}
                                                                            </div>
                                                                        </div>
                                                                    </div>
                                                                    <div className="flex items-center gap-4">
                                                                        <div className="flex items-center gap-2">
                                                                            <span className="text-sm text-foreground">启用</span>
                                                                            <Switch
                                                                                checked={tool.is_enabled}
                                                                                onCheckedChange={(checked) =>
                                                                                    handleToolConfigChange(tool.id, checked, tool.is_auto_run, server.id)
                                                                                }
                                                                            />
                                                                        </div>
                                                                        <div className="flex items-center gap-2">
                                                                            <span className="text-sm text-foreground">自动运行</span>
                                                                            <Switch
                                                                                checked={tool.is_auto_run}
                                                                                disabled={!tool.is_enabled}
                                                                                onCheckedChange={(checked) =>
                                                                                    handleToolConfigChange(tool.id, tool.is_enabled, checked, server.id)
                                                                                }
                                                                            />
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                            ))}
                                                        </div>
                                                    )}
                                                </div>
                                            </CollapsibleContent>
                                        </div>
                                    </Collapsible>
                                );
                            })}
                        </div>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    );
};

export default AssistantMCPConfigDialog;