import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";
import { Badge } from "../ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { Server, Wrench, MoreHorizontal, CheckCircle2, Circle, Settings } from "lucide-react";
import { toast } from 'sonner';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "../ui/dropdown-menu";

interface AssistantMCPConfigSectionProps {
    assistantId: number;
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


const AssistantMCPConfigSection: React.FC<AssistantMCPConfigSectionProps> = ({
    assistantId,
    onConfigChange
}) => {
    const [availableServers, setAvailableServers] = useState<MCPServerInfo[]>([]);
    const [selectedServerId, setSelectedServerId] = useState<number | null>(null);
    const [availableTools, setAvailableTools] = useState<MCPToolInfo[]>([]);
    const [allServerTools, setAllServerTools] = useState<Map<number, MCPToolInfo[]>>(new Map());
    const [loading, setLoading] = useState(false);

    // 获取可用的MCP服务器列表
    const fetchAvailableServers = useCallback(async () => {
        try {
            const serversWithTools = await invoke<(MCPServerInfo & { tools: MCPToolInfo[] })[]>(
                'get_assistant_mcp_servers_with_tools',
                { assistantId }
            );

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
            setAllServerTools(toolsMap);

            // 如果没有选中的服务器且有可用服务器，选择第一个
            if (!selectedServerId && servers.length > 0) {
                setSelectedServerId(servers[0].id);
            }
        } catch (error) {
            console.error('Failed to fetch available servers:', error);
            toast.error('获取MCP服务器列表失败: ' + error);
        }
    }, [assistantId, selectedServerId]);

    // 获取指定服务器的工具列表（从已缓存的数据中获取）
    const fetchAvailableTools = useCallback((serverId: number) => {
        setLoading(true);
        const tools = allServerTools.get(serverId) || [];
        setAvailableTools(tools);
        setLoading(false);
    }, [allServerTools]);

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
        isAutoRun: boolean
    ) => {
        try {
            await invoke('update_assistant_mcp_tool_config', {
                assistantId,
                mcpToolId: toolId,
                isEnabled,
                isAutoRun
            });

            // 更新当前显示的工具列表
            setAvailableTools(prev =>
                prev.map(tool =>
                    tool.id === toolId ? { ...tool, is_enabled: isEnabled, is_auto_run: isAutoRun } : tool
                )
            );

            // 更新所有工具的映射表
            setAllServerTools(prev => {
                const newMap = new Map(prev);
                if (selectedServerId) {
                    const serverTools = newMap.get(selectedServerId) || [];
                    const updatedTools = serverTools.map(tool =>
                        tool.id === toolId ? { ...tool, is_enabled: isEnabled, is_auto_run: isAutoRun } : tool
                    );
                    newMap.set(selectedServerId, updatedTools);
                    
                    // 检查是否所有工具都被禁用了，如果是则自动禁用服务器
                    const hasEnabledTools = updatedTools.some(tool => tool.is_enabled);
                    if (!hasEnabledTools && selectedServerId) {
                        const server = availableServers.find(s => s.id === selectedServerId);
                        if (server && server.is_enabled) {
                            // 自动禁用服务器
                            invoke('update_assistant_mcp_config', {
                                assistantId,
                                mcpServerId: selectedServerId,
                                isEnabled: false
                            }).then(() => {
                                setAvailableServers(prev =>
                                    prev.map(server =>
                                        server.id === selectedServerId ? { ...server, is_enabled: false } : server
                                    )
                                );
                                
                                toast.success('工具全部禁用，服务器已自动禁用');
                            }).catch((error) => {
                                console.error('Failed to auto-disable server:', error);
                            });
                        }
                    }
                }
                return newMap;
            });

            onConfigChange?.();
        } catch (error) {
            console.error('Failed to update tool config:', error);
            toast.error('更新工具配置失败: ' + error);
        }
    }, [assistantId, selectedServerId, availableServers, onConfigChange]);

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

            // 重新获取所有服务器和工具数据以更新缓存
            await fetchAvailableServers();
            
            toast.success(`批量${isEnabled ? '启用' : '禁用'}工具成功`);
            onConfigChange?.();
        } catch (error) {
            console.error('Failed to bulk update tools:', error);
            toast.error('批量更新工具失败: ' + error);
        }
    }, [assistantId, fetchAvailableTools, onConfigChange]);

    useEffect(() => {
        fetchAvailableServers();
    }, [fetchAvailableServers]);

    useEffect(() => {
        if (selectedServerId) {
            fetchAvailableTools(selectedServerId);
        }
    }, [selectedServerId, fetchAvailableTools]);

    const enabledServers = availableServers.filter(server => server.is_enabled);
    
    // 统计有效启用的工具数量：只有服务器启用时，其工具才算有效启用
    const totalEnabledTools = Array.from(allServerTools.entries())
        .filter(([serverId]) => availableServers.find(s => s.id === serverId)?.is_enabled)
        .flatMap(([, tools]) => tools)
        .filter(tool => tool.is_enabled).length;

    if (availableServers.length === 0) {
        return (
            <div className="bg-white rounded-lg border border-gray-200 p-6">
                <div className="text-center py-8">
                    <Server className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                    <p className="text-sm text-gray-500 mb-2">暂无可用的MCP服务器</p>
                    <p className="text-xs text-gray-400">请先在MCP配置中添加服务器</p>
                </div>
            </div>
        );
    }

    return (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h3 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
                        <Settings className="h-5 w-5" />
                        MCP工具配置
                    </h3>
                    <p className="text-sm text-gray-500">
                        为该助手配置可用的MCP服务器和工具 ({enabledServers.length}个服务器，{totalEnabledTools}个工具已启用)
                    </p>
                </div>
                {selectedServerId && (
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button variant="outline" size="sm">
                                <MoreHorizontal className="h-4 w-4 mr-2" />
                                批量操作
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent>
                            <DropdownMenuItem
                                onClick={() => handleBulkUpdateTools(selectedServerId, true)}
                            >
                                启用所有工具
                            </DropdownMenuItem>
                            <DropdownMenuItem
                                onClick={() => handleBulkUpdateTools(selectedServerId, false)}
                            >
                                禁用所有工具
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                )}
            </div>

            <Tabs value="servers" className="w-full">
                <TabsList className="grid w-full grid-cols-2">
                    <TabsTrigger value="servers">
                        服务器配置 ({availableServers.length})
                    </TabsTrigger>
                    <TabsTrigger value="tools">
                        工具配置 ({availableTools.length})
                    </TabsTrigger>
                </TabsList>

                <TabsContent value="servers" className="mt-4">
                    <div className="space-y-3">
                        {availableServers.map(server => (
                            <div
                                key={server.id}
                                className={`flex items-center justify-between p-4 rounded-lg border transition-colors cursor-pointer ${selectedServerId === server.id
                                        ? 'border-blue-200 bg-blue-50'
                                        : server.is_enabled
                                            ? 'border-gray-300 bg-white'
                                            : 'border-gray-200 bg-gray-50'
                                    }`}
                                onClick={() => setSelectedServerId(server.id)}
                            >
                                <div className="flex items-center gap-3">
                                    {server.is_enabled ? (
                                        <CheckCircle2 className="h-5 w-5 text-gray-700" />
                                    ) : (
                                        <Circle className="h-5 w-5 text-gray-400" />
                                    )}
                                    <div>
                                        <div className="font-medium text-gray-900">{server.name}</div>
                                        <div className="text-sm text-gray-500">
                                            {server.is_enabled ? '已启用' : '已禁用'}
                                        </div>
                                    </div>
                                </div>
                                <div className="flex items-center gap-2">
                                    {selectedServerId === server.id && (
                                        <Badge variant="secondary">当前选中</Badge>
                                    )}
                                    <Switch
                                        checked={server.is_enabled}
                                        onCheckedChange={(checked) => handleServerToggle(server.id, checked)}
                                        onClick={(e) => e.stopPropagation()}
                                    />
                                </div>
                            </div>
                        ))}
                    </div>
                </TabsContent>

                <TabsContent value="tools" className="mt-4">
                    {selectedServerId ? (
                        <div className="space-y-2">
                            <div className="flex items-center justify-between mb-4">
                                <p className="text-sm text-gray-600">
                                    {availableServers.find(s => s.id === selectedServerId)?.name} 的工具配置
                                </p>
                            </div>

                            {loading ? (
                                <div className="text-center py-8">
                                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto"></div>
                                    <p className="text-sm text-gray-500 mt-2">加载工具列表...</p>
                                </div>
                            ) : availableTools.length === 0 ? (
                                <div className="text-center py-8">
                                    <Wrench className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                                    <p className="text-sm text-gray-500">该服务器暂无可用工具</p>
                                </div>
                            ) : (
                                <div className="space-y-3">
                                    {availableTools.map(tool => (
                                        <div key={tool.id} className="flex items-center justify-between p-3 border rounded-lg">
                                            <div className="flex items-center gap-3">
                                                <Wrench className="h-4 w-4 text-gray-500" />
                                                <div>
                                                    <div className="font-medium text-gray-900">{tool.name}</div>
                                                    <div className="text-sm text-gray-500">
                                                        状态: {tool.is_enabled ? '启用' : '禁用'} |
                                                        自动运行: {tool.is_auto_run ? '是' : '否'}
                                                    </div>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-4">
                                                <div className="flex items-center gap-2">
                                                    <span className="text-sm text-gray-700">启用</span>
                                                    <Switch
                                                        checked={tool.is_enabled}
                                                        onCheckedChange={(checked) =>
                                                            handleToolConfigChange(tool.id, checked, tool.is_auto_run)
                                                        }
                                                    />
                                                </div>
                                                <div className="flex items-center gap-2">
                                                    <span className="text-sm text-gray-700">自动运行</span>
                                                    <Switch
                                                        checked={tool.is_auto_run}
                                                        disabled={!tool.is_enabled}
                                                        onCheckedChange={(checked) =>
                                                            handleToolConfigChange(tool.id, tool.is_enabled, checked)
                                                        }
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="text-center py-8">
                            <Server className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                            <p className="text-sm text-gray-500">请先选择一个服务器</p>
                        </div>
                    )}
                </TabsContent>
            </Tabs>
        </div>
    );
};

export default AssistantMCPConfigSection;