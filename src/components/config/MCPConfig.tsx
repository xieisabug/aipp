import React, { useCallback, useEffect, useState, useMemo } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";
import { Badge } from "../ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { PlusCircle, Server, Play, Trash2, Edit, RefreshCw } from "lucide-react";
import { toast } from 'sonner';
import ConfirmDialog from "../ConfirmDialog";
import MCPServerDialog from "./MCPServerDialog";

// 导入公共组件
import {
    ConfigPageLayout,
    SidebarList,
    ListItemButton,
    EmptyState,
    SelectOption
} from "../common";

import { MCPServer, MCPServerTool, MCPServerResource, MCPServerPrompt } from "../../data/MCP";

const MCPConfig: React.FC = () => {
    const [mcpServers, setMcpServers] = useState<MCPServer[]>([]);
    const [selectedServer, setSelectedServer] = useState<MCPServer | null>(null);
    const [serverTools, setServerTools] = useState<MCPServerTool[]>([]);
    const [serverResources, setServerResources] = useState<MCPServerResource[]>([]);
    const [serverPrompts, setServerPrompts] = useState<MCPServerPrompt[]>([]);
    
    // Dialog states
    const [serverDialogOpen, setServerDialogOpen] = useState(false);
    const [editingServer, setEditingServer] = useState<MCPServer | null>(null);
    const [confirmDialogOpen, setConfirmDialogOpen] = useState(false);
    const [deletingServerId, setDeletingServerId] = useState<number | null>(null);
    
    // Loading states
    const [isRefreshing, setIsRefreshing] = useState(false);

    // 获取MCP服务器列表
    const getMcpServers = useCallback(() => {
        invoke<MCPServer[]>('get_mcp_servers')
            .then((servers) => {
                setMcpServers(servers);
                // 如果没有选中的服务器，选择第一个
                if (!selectedServer && servers.length > 0) {
                    setSelectedServer(servers[0]);
                }
                // 如果当前选中的服务器已被删除，选择第一个
                if (selectedServer && !servers.find(s => s.id === selectedServer.id)) {
                    setSelectedServer(servers.length > 0 ? servers[0] : null);
                }
            })
            .catch((e) => {
                toast.error('获取MCP服务器失败: ' + e);
            });
    }, [selectedServer]);

    // 获取服务器工具列表
    const getServerTools = useCallback((serverId: number) => {
        invoke<MCPServerTool[]>('get_mcp_server_tools', { serverId })
            .then(setServerTools)
            .catch((e) => {
                toast.error('获取服务器工具失败: ' + e);
            });
    }, []);

    // 获取服务器资源列表
    const getServerResources = useCallback((serverId: number) => {
        invoke<MCPServerResource[]>('get_mcp_server_resources', { serverId })
            .then(setServerResources)
            .catch((e) => {
                toast.error('获取服务器资源失败: ' + e);
            });
    }, []);

    // 获取服务器提示列表
    const getServerPrompts = useCallback((serverId: number) => {
        invoke<MCPServerPrompt[]>('get_mcp_server_prompts', { serverId })
            .then(setServerPrompts)
            .catch((e) => {
                toast.error('获取服务器提示失败: ' + e);
            });
    }, []);

    useEffect(() => {
        getMcpServers();
    }, []);

    useEffect(() => {
        if (selectedServer) {
            getServerTools(selectedServer.id);
            getServerResources(selectedServer.id);
            getServerPrompts(selectedServer.id);
        }
    }, [selectedServer, getServerTools, getServerResources, getServerPrompts]);

    // 切换服务器启用状态
    const handleToggleServer = useCallback(async (serverId: number, isEnabled: boolean) => {
        try {
            await invoke('toggle_mcp_server', { id: serverId, isEnabled });
            setMcpServers(prev => prev.map(server => 
                server.id === serverId ? { ...server, is_enabled: isEnabled } : server
            ));
            if (selectedServer && selectedServer.id === serverId) {
                setSelectedServer(prev => prev ? { ...prev, is_enabled: isEnabled } : null);
            }
            toast.success(`服务器已${isEnabled ? '启用' : '禁用'}`);
        } catch (e) {
            toast.error('切换服务器状态失败: ' + e);
        }
    }, [selectedServer]);

    // 打开新增服务器对话框
    const openAddServerDialog = useCallback(() => {
        setEditingServer(null);
        setServerDialogOpen(true);
    }, []);

    // 打开编辑服务器对话框
    const openEditServerDialog = useCallback((server: MCPServer) => {
        setEditingServer(server);
        setServerDialogOpen(true);
    }, []);

    // 关闭服务器对话框
    const closeServerDialog = useCallback(() => {
        setServerDialogOpen(false);
        setEditingServer(null);
    }, []);

    // 服务器对话框提交
    const handleServerDialogSubmit = useCallback(() => {
        closeServerDialog();
        getMcpServers();
    }, [closeServerDialog, getMcpServers]);

    // 删除服务器
    const handleDeleteServer = useCallback((serverId: number) => {
        setDeletingServerId(serverId);
        setConfirmDialogOpen(true);
    }, []);

    const confirmDeleteServer = useCallback(async () => {
        if (!deletingServerId) return;
        
        try {
            await invoke('delete_mcp_server', { id: deletingServerId });
            toast.success('删除服务器成功');
            getMcpServers();
        } catch (e) {
            toast.error('删除服务器失败: ' + e);
        }
        
        setConfirmDialogOpen(false);
        setDeletingServerId(null);
    }, [deletingServerId, getMcpServers]);

    // 选择服务器
    const handleSelectServer = useCallback((server: MCPServer) => {
        setSelectedServer(server);
    }, []);

    // 刷新服务器能力
    const handleRefreshServerCapabilities = useCallback(async (serverId: number) => {
        setIsRefreshing(true);
        try {
            const [tools, resources, prompts] = await invoke<[MCPServerTool[], MCPServerResource[], MCPServerPrompt[]]>(
                'refresh_mcp_server_capabilities', 
                { serverId }
            );
            setServerTools(tools);
            setServerResources(resources);
            setServerPrompts(prompts);
            toast.success('服务器能力刷新成功');
        } catch (e) {
            toast.error('刷新服务器能力失败: ' + e);
        } finally {
            setIsRefreshing(false);
        }
    }, []);

    // 更新工具配置
    const handleUpdateTool = useCallback(async (toolId: number, isEnabled: boolean, isAutoRun: boolean) => {
        try {
            await invoke('update_mcp_server_tool', { toolId, isEnabled, isAutoRun });
            setServerTools(prev => prev.map(tool => 
                tool.id === toolId ? { ...tool, is_enabled: isEnabled, is_auto_run: isAutoRun } : tool
            ));
        } catch (e) {
            toast.error('更新工具配置失败: ' + e);
        }
    }, []);

    // 更新提示配置
    const handleUpdatePrompt = useCallback(async (promptId: number, isEnabled: boolean) => {
        try {
            await invoke('update_mcp_server_prompt', { promptId, isEnabled });
            setServerPrompts(prev => prev.map(prompt => 
                prompt.id === promptId ? { ...prompt, is_enabled: isEnabled } : prompt
            ));
        } catch (e) {
            toast.error('更新提示配置失败: ' + e);
        }
    }, []);

    // 下拉菜单选项
    const selectOptions: SelectOption[] = useMemo(() =>
        mcpServers.map(server => ({
            id: server.id.toString(),
            label: server.name,
            icon: server.is_enabled ? <Play className="h-4 w-4" /> : <Server className="h-4 w-4" />
        })), [mcpServers]);

    // 下拉菜单选择回调
    const handleSelectFromDropdown = useCallback((serverId: string) => {
        const server = mcpServers.find(s => s.id.toString() === serverId);
        if (server) {
            handleSelectServer(server);
        }
    }, [mcpServers, handleSelectServer]);

    // 新增按钮组件
    const addButton = useMemo(() => (
        <Button
            onClick={openAddServerDialog}
            className="gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-sm hover:shadow-md transition-all"
        >
            <PlusCircle className="h-4 w-4" />
            新增MCP
        </Button>
    ), [openAddServerDialog]);

    // 空状态
    if (mcpServers.length === 0) {
        return (
            <>
                <ConfigPageLayout
                    sidebar={null}
                    content={null}
                    emptyState={
                        <EmptyState
                            icon={<Server className="h-8 w-8 text-gray-500" />}
                            title="还没有配置MCP服务器"
                            description="开始添加你的第一个MCP服务器，扩展AI助手的能力"
                            action={
                                <Button
                                    onClick={openAddServerDialog}
                                    className="gap-2 bg-gray-800 hover:bg-gray-900 text-white shadow-lg hover:shadow-xl transition-all"
                                >
                                    <PlusCircle className="h-4 w-4" />
                                    添加第一个MCP服务器
                                </Button>
                            }
                        />
                    }
                    showEmptyState={true}
                />

                {/* MCP服务器对话框 - 空状态时也需要渲染 */}
                <MCPServerDialog
                    isOpen={serverDialogOpen}
                    onClose={closeServerDialog}
                    onSubmit={handleServerDialogSubmit}
                    editingServer={editingServer}
                />
            </>
        );
    }

    // 侧边栏内容
    const sidebar = (
        <SidebarList
            title="MCP服务器"
            description="选择服务器进行配置"
            icon={<Server className="h-5 w-5" />}
        >
            {mcpServers.map((server) => (
                <ListItemButton
                    key={server.id}
                    isSelected={selectedServer?.id === server.id}
                    onClick={() => handleSelectServer(server)}
                >
                    <div className="flex items-center w-full">
                        <div className="flex-1 truncate">
                            <div className="font-medium truncate">{server.name}</div>
                            <div className="text-xs text-gray-500 truncate">{server.transport_type}</div>
                        </div>
                        {server.is_enabled && (
                            <Play className="h-3 w-3 ml-2 flex-shrink-0 text-green-500" />
                        )}
                    </div>
                </ListItemButton>
            ))}
        </SidebarList>
    );

    // 右侧内容
    const content = selectedServer ? (
        <div className="space-y-6">
            {/* 服务器基本信息 */}
            <div className="bg-white rounded-lg border border-gray-200 p-6">
                <div className="flex items-center justify-between mb-4">
                    <div>
                        <h3 className="text-lg font-semibold text-gray-900">{selectedServer.name}</h3>
                        <p className="text-sm text-gray-500">{selectedServer.description || '暂无描述'}</p>
                    </div>
                    <div className="flex items-center gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleRefreshServerCapabilities(selectedServer.id)}
                            disabled={isRefreshing}
                        >
                            <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`} />
                            {isRefreshing ? '刷新中...' : '刷新能力'}
                        </Button>
                        <Switch
                            checked={selectedServer.is_enabled}
                            onCheckedChange={(checked) => handleToggleServer(selectedServer.id, checked)}
                        />
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => openEditServerDialog(selectedServer)}
                        >
                            <Edit className="h-4 w-4" />
                        </Button>
                        <Button
                            variant="destructive"
                            size="sm"
                            onClick={() => handleDeleteServer(selectedServer.id)}
                        >
                            <Trash2 className="h-4 w-4" />
                        </Button>
                    </div>
                </div>
                
                <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                        <span className="font-medium text-gray-700">传输类型:</span>
                        <Badge variant="secondary" className="ml-2">
                            {selectedServer.transport_type}
                        </Badge>
                    </div>
                    <div>
                        <span className="font-medium text-gray-700">长期运行:</span>
                        <Badge variant={selectedServer.is_long_running ? "default" : "secondary"} className="ml-2">
                            {selectedServer.is_long_running ? "是" : "否"}
                        </Badge>
                    </div>
                    {selectedServer.timeout && (
                        <div>
                            <span className="font-medium text-gray-700">超时时间:</span>
                            <span className="ml-2 text-gray-600">{selectedServer.timeout}ms</span>
                        </div>
                    )}
                </div>
            </div>

            {/* 能力列表 - 使用 Tabs */}
            <div className="bg-white rounded-lg border border-gray-200 p-6">
                <h4 className="text-md font-semibold text-gray-900 mb-4">服务器能力</h4>
                
                {/* 动态计算需要显示的tabs */}
                {(() => {
                    const availableTabs = [];
                    if (serverTools.length > 0) availableTabs.push("tools");
                    if (serverPrompts.length > 0) availableTabs.push("prompts");
                    if (serverResources.length > 0) availableTabs.push("resources");
                    
                    const defaultValue = availableTabs.length > 0 ? availableTabs[0] : "tools";
                    const gridCols = availableTabs.length === 1 ? "grid-cols-1" : 
                                   availableTabs.length === 2 ? "grid-cols-2" : "grid-cols-3";
                    
                    return availableTabs.length > 0 ? (
                        <Tabs defaultValue={defaultValue} className="w-full">
                            <TabsList className={`grid w-full ${gridCols}`}>
                                {serverTools.length > 0 && (
                                    <TabsTrigger value="tools">
                                        工具 ({serverTools.length})
                                    </TabsTrigger>
                                )}
                                {serverPrompts.length > 0 && (
                                    <TabsTrigger value="prompts">
                                        提示 ({serverPrompts.length})
                                    </TabsTrigger>
                                )}
                                {serverResources.length > 0 && (
                                    <TabsTrigger value="resources">
                                        资源 ({serverResources.length})
                                    </TabsTrigger>
                                )}
                            </TabsList>

                    {/* 工具列表 */}
                    {serverTools.length > 0 && (
                        <TabsContent value="tools" className="mt-4">
                            <div className="space-y-3">
                                {serverTools.map((tool) => (
                                    <div key={tool.id} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                                        <div className="flex-1">
                                            <div className="font-medium text-gray-900">{tool.tool_name}</div>
                                            {tool.tool_description && (
                                                <div className="text-sm text-gray-500 mt-1">{tool.tool_description}</div>
                                            )}
                                        </div>
                                        <div className="flex items-center gap-4">
                                            <div className="flex items-center gap-2">
                                                <span className="text-sm text-gray-700">启用</span>
                                                <Switch
                                                    checked={tool.is_enabled}
                                                    onCheckedChange={(checked) => 
                                                        handleUpdateTool(tool.id, checked, tool.is_auto_run)
                                                    }
                                                />
                                            </div>
                                            <div className="flex items-center gap-2">
                                                <span className="text-sm text-gray-700">自动运行</span>
                                                <Switch
                                                    checked={tool.is_auto_run}
                                                    onCheckedChange={(checked) => 
                                                        handleUpdateTool(tool.id, tool.is_enabled, checked)
                                                    }
                                                />
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </TabsContent>
                    )}

                    {/* 提示列表 */}
                    {serverPrompts.length > 0 && (
                        <TabsContent value="prompts" className="mt-4">
                            <div className="space-y-3">
                                {serverPrompts.map((prompt) => (
                                    <div key={prompt.id} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                                        <div className="flex-1">
                                            <div className="font-medium text-gray-900">{prompt.prompt_name}</div>
                                            {prompt.prompt_description && (
                                                <div className="text-sm text-gray-500 mt-1">{prompt.prompt_description}</div>
                                            )}
                                        </div>
                                        <div className="flex items-center gap-2">
                                            <span className="text-sm text-gray-700">启用</span>
                                            <Switch
                                                checked={prompt.is_enabled}
                                                onCheckedChange={(checked) => 
                                                    handleUpdatePrompt(prompt.id, checked)
                                                }
                                            />
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </TabsContent>
                    )}

                    {/* 资源列表 */}
                    {serverResources.length > 0 && (
                        <TabsContent value="resources" className="mt-4">
                            <div className="space-y-3">
                                {serverResources.map((resource) => (
                                    <div key={resource.id} className="p-3 bg-gray-50 rounded-lg">
                                        <div className="font-medium text-gray-900">{resource.resource_name}</div>
                                        <div className="text-sm text-gray-500 mt-1">{resource.resource_uri}</div>
                                        {resource.resource_description && (
                                            <div className="text-sm text-gray-600 mt-1">{resource.resource_description}</div>
                                        )}
                                        <Badge variant="outline" className="mt-2">{resource.resource_type}</Badge>
                                    </div>
                                ))}
                            </div>
                        </TabsContent>
                    )}

                        </Tabs>
                    ) : (
                        <div className="text-center py-8">
                            <Server className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                            <p className="text-sm text-gray-500">暂无能力数据</p>
                            <p className="text-xs text-gray-400 mt-1">点击上方"刷新能力"按钮获取服务器能力</p>
                        </div>
                    );
                })()}
            </div>
        </div>
    ) : (
        <EmptyState
            icon={<Server className="h-8 w-8 text-gray-500" />}
            title="选择一个MCP服务器"
            description="从左侧列表中选择一个服务器开始配置"
        />
    );

    return (
        <>
            <ConfigPageLayout
                sidebar={sidebar}
                content={content}
                selectOptions={selectOptions}
                selectedOptionId={selectedServer?.id.toString()}
                onSelectOption={handleSelectFromDropdown}
                selectPlaceholder="选择MCP服务器"
                addButton={addButton}
            />

            {/* MCP服务器对话框 */}
            <MCPServerDialog
                isOpen={serverDialogOpen}
                onClose={closeServerDialog}
                onSubmit={handleServerDialogSubmit}
                editingServer={editingServer}
            />

            {/* 确认删除对话框 */}
            <ConfirmDialog
                isOpen={confirmDialogOpen}
                title="确认删除"
                confirmText="确定要删除这个MCP服务器吗？删除后相关配置将无法恢复。"
                onConfirm={confirmDeleteServer}
                onCancel={() => setConfirmDialogOpen(false)}
            />
        </>
    );
};

export default MCPConfig;