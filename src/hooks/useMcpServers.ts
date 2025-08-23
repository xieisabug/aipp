import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export interface MCPServerForSelect {
    id: number;
    name: string;
    description?: string;
    is_enabled: boolean;
}

export const useMcpServers = (shouldFetch: boolean = true) => {
    const [mcpServers, setMcpServers] = useState<MCPServerForSelect[]>([]);
    const [loading, setLoading] = useState(shouldFetch);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!shouldFetch) {
            setLoading(false);
            return;
        }

        setLoading(true);
        invoke<Array<MCPServerForSelect>>("get_mcp_servers")
            .then((serverList) => {
                // Filter to only enabled servers for selection
                const enabledServers = serverList.filter(server => server.is_enabled);
                setMcpServers(enabledServers);
                setError(null);
            })
            .catch((err) => {
                const errorMsg = "获取MCP服务器列表失败: " + err;
                setError(errorMsg);
                toast.error(errorMsg);
            })
            .finally(() => {
                setLoading(false);
            });
    }, [shouldFetch]);

    return { mcpServers, loading, error };
};