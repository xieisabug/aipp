import React, { useState, useEffect, useCallback } from 'react';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '../ui/dialog';
import { toast } from 'sonner';
import { MCPServerRequest } from '../../data/MCP';
import { readText } from '@tauri-apps/plugin-clipboard-manager';

interface JSONImportDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onImport: (configs: MCPServerRequest[]) => void;
}


const JSONImportDialog: React.FC<JSONImportDialogProps> = ({
    isOpen,
    onClose,
    onImport
}) => {
    const [jsonText, setJsonText] = useState('');
    const [isLoading, setIsLoading] = useState(false);

    // 如果剪贴板里的是 json 就直接加载到对话框
    useEffect(() => {
        if (isOpen) {
            loadClipboardContent();
        } else {
            setJsonText('');
        }
    }, [isOpen]);

    const loadClipboardContent = useCallback(async () => {
        setIsLoading(true);
        try {
            const clipboardText = await readText();
            if (clipboardText && isValidJSON(clipboardText)) {
                setJsonText(clipboardText);
                toast.success('已自动从剪切板加载JSON内容');
            }
        } catch (error) {
            console.warn('Failed to read clipboard:', error);
        } finally {
            setIsLoading(false);
        }
    }, []);

    const isValidJSON = (text: string): boolean => {
        try {
            JSON.parse(text);
            return true;
        } catch {
            return false;
        }
    };

    const validateMCPConfig = (config: any): MCPServerRequest[] => {
        // 检查是否是标准MCP格式，包含mcpServers对象
        if (config.mcpServers && typeof config.mcpServers === 'object') {
            return parseMCPServersFormat(config.mcpServers);
        }
        
        // 检查是否是单个服务器配置（旧格式）
        if (config.name || config.transport_type || config.command || config.url) {
            return [parseSingleServerConfig(config)];
        }
        
        throw new Error('无效的JSON格式，请使用标准MCP配置格式或单个服务器配置格式');
    };

    const parseMCPServersFormat = (mcpServers: any): MCPServerRequest[] => {
        const servers: MCPServerRequest[] = [];
        
        for (const [serverName, serverConfig] of Object.entries(mcpServers)) {
            if (!serverConfig || typeof serverConfig !== 'object') {
                throw new Error(`服务器 "${serverName}" 配置无效`);
            }
            
            const config = serverConfig as any;
            let transport_type: string;
            let command = '';
            let url = '';
            let environment_variables = '';
            
            // 根据配置确定传输类型
            if (config.url) {
                transport_type = config.url.startsWith('http') ? 'http' : 'sse';
                url = config.url;
            } else if (config.command || config.args) {
                transport_type = 'stdio';
                if (config.command && config.args) {
                    command = `${config.command} ${config.args.join(' ')}`;
                } else if (config.command) {
                    command = config.command;
                } else {
                    throw new Error(`服务器 "${serverName}" 的stdio配置必须包含command字段`);
                }
            } else {
                throw new Error(`服务器 "${serverName}" 必须包含url或command/args字段`);
            }
            
            // 处理环境变量
            if (config.env && typeof config.env === 'object') {
                environment_variables = Object.entries(config.env)
                    .map(([key, value]) => `${key}=${value}`)
                    .join('\n');
            }
            
            servers.push({
                name: serverName,
                description: config.description || `${serverName} MCP Server`,
                transport_type,
                command,
                environment_variables,
                url,
                timeout: config.timeout || 30000,
                is_long_running: config.is_long_running || false,
                is_enabled: config.is_enabled !== false, // Default to true
            });
        }
        
        return servers;
    };

    const parseSingleServerConfig = (config: any): MCPServerRequest => {
        // 必需字段
        if (!config.name || typeof config.name !== 'string') {
            throw new Error('缺少必需的字段: name');
        }

        if (!config.transport_type || !['stdio', 'sse', 'http'].includes(config.transport_type)) {
            throw new Error('transport_type必须是stdio、sse或http之一');
        }

        // 类型特定验证
        if (config.transport_type === 'stdio' && !config.command) {
            throw new Error('stdio类型必须提供command字段');
        }

        if ((config.transport_type === 'sse' || config.transport_type === 'http') && !config.url) {
            throw new Error(`${config.transport_type}类型必须提供url字段`);
        }

        // 如果环境变量是对象，则转换为字符串
        let envVars = config.environment_variables || '';
        if (typeof envVars === 'object' && envVars !== null) {
            envVars = Object.entries(envVars)
                .map(([key, value]) => `${key}=${value}`)
                .join('\n');
        }

        return {
            name: config.name,
            description: config.description || '',
            transport_type: config.transport_type,
            command: config.command || '',
            environment_variables: envVars,
            url: config.url || '',
            timeout: config.timeout || 30000,
            is_long_running: config.is_long_running || false,
            is_enabled: config.is_enabled !== false, // Default to true
        };
    };

    const handleImport = useCallback(() => {
        if (!jsonText.trim()) {
            toast.error('请输入JSON配置');
            return;
        }

        try {
            const config = JSON.parse(jsonText);
            const mcpConfigs = validateMCPConfig(config);
            onImport(mcpConfigs);
            const count = mcpConfigs.length;
            toast.success(`成功导入 ${count} 个MCP服务器配置`);
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'JSON格式无效';
            toast.error(`导入失败: ${errorMessage}`);
        }
    }, [jsonText, onImport]);

    const handleCancel = useCallback(() => {
        onClose();
    }, [onClose]);

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && handleCancel()}>
            <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
                <DialogHeader>
                    <DialogTitle>JSON导入MCP服务器配置</DialogTitle>
                </DialogHeader>

                <div className="space-y-4 py-4">
                    <div className="space-y-2">
                        <Label htmlFor="json-content">JSON配置</Label>
                        <Textarea
                            id="json-content"
                            placeholder={`标准MCP配置格式：
{
  "mcpServers": {
    "context7": {
      "url": "https://mcp.context7.com/mcp"
    },
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/directory"]
    }
  }
}

或单个服务器配置：
{
  "name": "my-server",
  "transport_type": "stdio",
  "command": "npx server-command",
  "description": "My MCP server"
}`}
                            rows={15}
                            value={jsonText}
                            onChange={(e) => setJsonText(e.target.value)}
                            className="font-mono text-sm"
                        />
                    </div>

                    <div className="flex gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={loadClipboardContent}
                            disabled={isLoading}
                        >
                            {isLoading ? '读取中...' : '从剪切板读取'}
                        </Button>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setJsonText('')}
                        >
                            清空
                        </Button>
                    </div>
                </div>

                <DialogFooter>
                    <Button
                        variant="outline"
                        onClick={handleCancel}
                    >
                        取消
                    </Button>
                    <Button
                        onClick={handleImport}
                        disabled={!jsonText.trim()}
                    >
                        导入
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
};

export default JSONImportDialog;