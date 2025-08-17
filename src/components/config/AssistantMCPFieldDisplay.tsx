import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/button";
import { Settings } from "lucide-react";
import AssistantMCPConfigDialog from './AssistantMCPConfigDialog';

interface AssistantMCPFieldDisplayProps {
    assistantId: number;
    onConfigChange?: () => void;
    navigateTo: (menuKey: string) => void;
}

interface MCPSummary {
    totalServers: number;
    enabledServers: number;
    totalTools: number;
    enabledTools: number;
    useNativeToolCall: boolean;
}

const AssistantMCPFieldDisplay: React.FC<AssistantMCPFieldDisplayProps> = ({
    assistantId,
    onConfigChange,
    navigateTo
}) => {
    const [mcpSummary, setMcpSummary] = useState<MCPSummary>({
        totalServers: 0,
        enabledServers: 0,
        totalTools: 0,
        enabledTools: 0,
        useNativeToolCall: false
    });
    const [configDialogOpen, setConfigDialogOpen] = useState(false);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // 获取 MCP 配置摘要
    const fetchMCPSummary = useCallback(async () => {
        try {
            setLoading(true);
            setError(null);

            // 使用新的接口一次性获取所有服务器和工具信息
            const serversWithTools = await invoke<{id: number, name: string, is_enabled: boolean, tools: {id: number, name: string, is_enabled: boolean, is_auto_run: boolean}[]}[]>(
                'get_assistant_mcp_servers_with_tools',
                { assistantId }
            );

            // 获取原生ToolCall配置
            let useNativeToolCall = false;
            try {
                const nativeToolCallValue = await invoke<string>('get_assistant_field_value', {
                    assistantId,
                    fieldName: 'use_native_toolcall'
                });
                useNativeToolCall = nativeToolCallValue === 'true';
            } catch (error) {
                // 如果没有找到配置，默认为false
                useNativeToolCall = false;
            }

            const totalServers = serversWithTools.length;
            const enabledServers = serversWithTools.filter(server => server.is_enabled).length;

            let totalTools = 0;
            let enabledTools = 0;

            for (const server of serversWithTools) {
                totalTools += server.tools.length;
                // 只有服务器启用时，工具才算有效启用
                if (server.is_enabled) {
                    enabledTools += server.tools.filter(tool => tool.is_enabled).length;
                }
            }

            setMcpSummary({
                totalServers,
                enabledServers,
                totalTools,
                enabledTools,
                useNativeToolCall
            });

        } catch (error) {
            console.error('Failed to fetch MCP summary:', error);
            setError(error as string);
        } finally {
            setLoading(false);
        }
    }, [assistantId]);

    useEffect(() => {
        fetchMCPSummary();
    }, [fetchMCPSummary]);

    const handleOpenConfig = useCallback(() => {
        if (mcpSummary.totalServers === 0) {
            // 跳转到mcp的配置页面
            navigateTo("mcp-config");
        } else {
            setConfigDialogOpen(true);
        }
    }, [mcpSummary.totalServers]);

    const handleCloseConfig = useCallback(() => {
        setConfigDialogOpen(false);
    }, []);

    const handleConfigChanged = useCallback(() => {
        fetchMCPSummary(); // 刷新摘要数据
        onConfigChange?.(); // 通知父组件配置已更改
    }, [fetchMCPSummary, onConfigChange]);

    const getSummaryText = () => {
        if (loading) {
            return "加载中...";
        }

        if (error) {
            return "加载失败";
        }

        if (mcpSummary.totalServers === 0) {
            return "暂无可用的MCP服务器";
        }

        const toolCallMethod = mcpSummary.useNativeToolCall ? "原生ToolCall" : "Prompt调用";
        return [<div>启用 {mcpSummary.enabledServers} 个服务器</div>, <div>启用 {mcpSummary.enabledTools} 个工具</div>,<div>{toolCallMethod}</div>];
    };

    return (
        <>
            <div className="flex items-center justify-between">
                <div className="flex items-start gap-3">
                    
                    <div>
                        <div className="text-sm font-medium text-foreground">{getSummaryText()}</div>
                    </div>
                </div>

                <Button
                    variant={mcpSummary.totalServers === 0 ? "default" : "outline"}
                    size="sm"
                    onClick={handleOpenConfig}
                    disabled={loading}
                >
                    <Settings className="h-4 w-4 mr-1" />
                    {mcpSummary.totalServers === 0 ? "配置MCP" : "配置"}
                </Button>
            </div>

            <AssistantMCPConfigDialog
                assistantId={assistantId}
                isOpen={configDialogOpen}
                onClose={handleCloseConfig}
                onConfigChange={handleConfigChanged}
            />
        </>
    );
};

export default AssistantMCPFieldDisplay;