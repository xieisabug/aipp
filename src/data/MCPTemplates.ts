import { MCPServerRequest, MCPTransportType } from './MCP';

export interface MCPTemplate {
    id: string;
    name: string;
    description: string;
    category: 'Quick Start' | 'Import' | 'Builtin Tools';
    template: Partial<MCPServerRequest>;
}

export const MCP_QUICK_TEMPLATES: MCPTemplate[] = [
    {
        id: 'stdio',
        name: 'Stdio MCP',
        description: '本地命令行MCP服务器',
        category: 'Quick Start',
        template: {
            transport_type: 'stdio',
            command: 'npx @modelcontextprotocol/server-filesystem /path/to/directory',
            is_long_running: false,
            is_enabled: true,
            timeout: 30000,
        }
    },
    {
        id: 'sse',
        name: 'SSE MCP',
        description: '服务器发送事件MCP服务器',
        category: 'Quick Start',
        template: {
            transport_type: 'sse',
            url: 'http://localhost:3000/mcp',
            is_long_running: true,
            is_enabled: true,
            timeout: 30000,
        }
    },
    {
        id: 'http',
        name: 'HTTP MCP',
        description: 'HTTP协议MCP服务器',
        category: 'Quick Start',
        template: {
            transport_type: 'http',
            url: 'http://localhost:8080/mcp',
            is_long_running: false,
            is_enabled: true,
            timeout: 30000,
        }
    },
    {
        id: 'builtin-search',
        name: '内置工具',
        description: '通过内置协议（aipp:*）快速添加官方内置工具',
        category: 'Import',
        template: {
            transport_type: 'stdio',
            command: 'aipp:search',
            is_long_running: false,
            is_enabled: true,
            timeout: 10000,
            is_builtin: true,
        }
    },
    {
        id: 'json-import',
        name: 'JSON导入',
        description: '从JSON配置导入MCP服务器',
        category: 'Import',
        template: {}
    }
];

export const getTemplateByType = (transportType: MCPTransportType): MCPTemplate | undefined => {
    return MCP_QUICK_TEMPLATES.find(template => template.id === transportType);
};

export const getQuickStartTemplates = (): MCPTemplate[] => {
    return MCP_QUICK_TEMPLATES.filter(template => template.category === 'Quick Start');
};

export const getImportTemplates = (): MCPTemplate[] => {
    return MCP_QUICK_TEMPLATES.filter(template => template.category === 'Import');
};

export const getBuiltinToolsTemplates = (): MCPTemplate[] => {
    return MCP_QUICK_TEMPLATES.filter(template => template.category === 'Builtin Tools');
};