import React, { useState, useCallback, useMemo, useEffect } from "react";
import { Play, Maximize2, Loader2, CheckCircle, XCircle, Blocks, ChevronDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { invoke } from '@tauri-apps/api/core';
import { MCPToolCall } from '@/data/MCPToolCall';

interface McpToolCallProps {
    serverName?: string;
    toolName?: string;
    parameters?: string;
    conversationId?: number;
    messageId?: number;
    callId?: number; // If provided, this is an existing call
}

type ExecutionState = "idle" | "pending" | "executing" | "success" | "failed";

const JsonDisplay: React.FC<{ content: string; maxHeight?: string }> = ({ content, maxHeight = "120px" }) => {
    const formattedJson = useMemo(() => {
        try {
            const parsed = JSON.parse(content);
            return JSON.stringify(parsed, null, 2);
        } catch {
            return content;
        }
    }, [content]);

    return (
        <ScrollArea className="w-full" style={{ maxHeight }}>
            <pre className="text-xs font-mono bg-secondary p-2 mt-1 mb-4 rounded text-secondary-foreground whitespace-pre-wrap break-words">
                {formattedJson}
            </pre>
        </ScrollArea>
    );
};

const StatusIndicator: React.FC<{ state: ExecutionState }> = ({ state }) => {
    switch (state) {
        case "idle": return null;
        case "pending":
            return <Badge variant="secondary" className="flex items-center gap-1"><Loader2 className="h-3 w-3 animate-spin" />等待中</Badge>;
        case "executing":
            return <Badge variant="secondary" className="flex items-center gap-1"><Loader2 className="h-3 w-3 animate-spin" />执行中</Badge>;
        case "success":
            return <Badge variant="default" className="flex items-center gap-1 bg-green-100 text-green-800 border-green-200"><CheckCircle className="h-3 w-3" />成功</Badge>;
        case "failed":
            return <Badge variant="destructive" className="flex items-center gap-1"><XCircle className="h-3 w-3" />失败</Badge>;
        default: return null;
    }
};

const McpToolCall: React.FC<McpToolCallProps> = ({
    serverName = "未知服务器",
    toolName = "未知工具",
    parameters = "{}",
    conversationId,
    messageId,
    callId
}) => {
    const [executionState, setExecutionState] = useState<ExecutionState>("idle");
    const [executionResult, setExecutionResult] = useState<string | null>(null);
    const [executionError, setExecutionError] = useState<string | null>(null);
    const [isExpanded, setIsExpanded] = useState<boolean>(false);
    const [toolCallId, setToolCallId] = useState<number | null>(callId || null);

    // 检查是否已经执行过
    const isExecuted = executionState === "success" || executionState === "failed";

    // 如果提供了 callId，尝试获取已有的执行结果
    useEffect(() => {
        if (callId && executionState === "idle") {
            const fetchExistingResult = async () => {
                try {
                    const result = await invoke<MCPToolCall>('get_mcp_tool_call', {
                        callId: callId
                    });
                    
                    if (result.status === 'success' && result.result) {
                        setExecutionResult(result.result);
                        setExecutionState("success");
                    } else if (result.status === 'failed' && result.error) {
                        setExecutionError(result.error);
                        setExecutionState("failed");
                    }
                } catch (error) {
                    console.warn('Failed to fetch existing tool call result:', error);
                }
            };
            
            fetchExistingResult();
        }
    }, [callId, executionState]);

    // 如果没有 callId，尝试根据消息参数查询是否存在相关的工具调用记录
    useEffect(() => {
        if (!callId && conversationId && messageId && executionState === "idle") {
            const findExistingToolCall = async () => {
                try {
                    const allCalls = await invoke<MCPToolCall[]>('get_mcp_tool_calls_by_conversation', {
                        conversationId: conversationId
                    });
                    
                    // 查找匹配的工具调用（相同的消息ID、服务器名和工具名）
                    const matchingCall = allCalls.find(call => 
                        call.message_id === messageId && 
                        call.server_name === serverName && 
                        call.tool_name === toolName &&
                        call.parameters === parameters
                    );
                    
                    if (matchingCall) {
                        setToolCallId(matchingCall.id);
                        
                        if (matchingCall.status === 'success' && matchingCall.result) {
                            setExecutionResult(matchingCall.result);
                            setExecutionState("success");
                        } else if (matchingCall.status === 'failed' && matchingCall.error) {
                            setExecutionError(matchingCall.error);
                            setExecutionState("failed");
                        } else if (matchingCall.status === 'executing') {
                            setExecutionState("executing");
                        }
                    }
                } catch (error) {
                    console.warn('Failed to find existing tool call:', error);
                }
            };
            
            findExistingToolCall();
        }
    }, [callId, conversationId, messageId, serverName, toolName, parameters, executionState]);

    const handleExecute = useCallback(async () => {
        if (!conversationId) {
            console.error('conversation_id is required for execution');
            return;
        }

        try {
            setExecutionState("executing");
            setExecutionResult(null);
            setExecutionError(null);

            let currentCallId = toolCallId;

            // Create tool call if it doesn't exist
            if (!currentCallId) {
                const createdCall = await invoke<MCPToolCall>('create_mcp_tool_call', {
                    conversationId: conversationId,
                    messageId: messageId,
                    serverName: serverName,
                    toolName: toolName,
                    parameters,
                });
                currentCallId = createdCall.id;
                setToolCallId(currentCallId);
            }

            // Execute the tool call
            const result = await invoke<MCPToolCall>('execute_mcp_tool_call', {
                callId: currentCallId
            });

            if (result.status === 'success' && result.result) {
                setExecutionResult(result.result);
                setExecutionState("success");
            } else if (result.status === 'failed' && result.error) {
                setExecutionError(result.error);
                setExecutionState("failed");
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : "执行失败";
            setExecutionError(errorMessage);
            setExecutionState("failed");
        }
    }, [conversationId, messageId, serverName, toolName, parameters, toolCallId]);

    const renderResult = () => {
        if (executionResult) {
            return (
                <div className="mt-2">
                    <span className="text-xs text-muted-foreground">结果:</span>
                    <ScrollArea className="w-full mt-1 max-w-full" style={{ maxHeight: "200px" }}>
                        <div className="text-xs font-mono bg-muted p-2 rounded max-w-full overflow-hidden">
                            <pre className="whitespace-pre-wrap break-words text-muted-foreground max-w-full overflow-hidden">
                                {executionResult}
                            </pre>
                        </div>
                    </ScrollArea>
                </div>
            );
        }

        if (executionError) {
            return (
                <div className="mt-2">
                    <span className="text-xs text-muted-foreground">错误:</span>
                    <ScrollArea className="w-full mt-1 max-w-full" style={{ maxHeight: "200px" }}>
                        <div className="text-xs font-mono bg-muted p-2 rounded max-w-full overflow-hidden">
                            <div className="text-red-600 max-w-full overflow-hidden"><strong>错误:</strong> {executionError}</div>
                        </div>
                    </ScrollArea>
                </div>
            );
        }

        return null;
    };

    const DialogContent_: React.FC = () => (
        <DialogContent className="max-w-4xl max-h-[80vh]">
            <DialogHeader>
                <DialogTitle className="flex items-center gap-2">
                    <Blocks className="h-4 w-4" />
                    {serverName}
                    <span className="text-xs font-bold mb-1 text-muted-foreground"> - </span>
                    {toolName}
                </DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
                <div>
                    <h4 className="text-sm font-medium mb-2">参数:</h4>
                    <JsonDisplay content={parameters} maxHeight="400px" />
                </div>
                {!isExecuted && (
                    <div className="flex items-center gap-2">
                        <Button onClick={handleExecute} disabled={executionState === "executing"} size="sm" className="flex items-center gap-2">
                            {executionState === "executing" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                            执行
                        </Button>
                        <StatusIndicator state={executionState} />
                    </div>
                )}
                {isExecuted && <StatusIndicator state={executionState} />}
                {renderResult()}
            </div>
        </DialogContent>
    );

    return (
        <div className="w-full max-w-full my-1 p-2 border border-border rounded-md bg-card overflow-hidden">
            <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-sm min-w-0 flex-1">
                    <Blocks className="h-4 w-4 flex-shrink-0" />
                    <span className="truncate">{serverName}</span>
                    <span className="text-xs font-bold text-muted-foreground flex-shrink-0"> - </span>
                    <span className="truncate">{toolName}</span>
                </div>
                <div className="flex items-center gap-1 flex-shrink-0">
                    {!isExpanded && <StatusIndicator state={executionState} />}
                    {!isExpanded && !isExecuted && (
                        <Button
                            onClick={handleExecute}
                            disabled={executionState === "executing"}
                            size="sm"
                            variant="ghost"
                            className="h-7 w-7 p-0 flex-shrink-0"
                        >
                            {executionState === "executing" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
                        </Button>
                    )}
                    <Button
                        onClick={() => setIsExpanded(!isExpanded)}
                        size="sm"
                        variant="ghost"
                        className="h-7 w-7 p-0 flex-shrink-0"
                    >
                        {isExpanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
                    </Button>
                </div>
            </div>

            {isExpanded && (
                <div className="mt-2 space-y-2 max-w-full overflow-hidden">
                    <div className="flex items-center justify-end mb-2">
                        <StatusIndicator state={executionState} />
                    </div>
                    <div className="max-w-full overflow-hidden">
                        <span className="text-xs font-medium mb-1 text-muted-foreground">参数:</span>
                        <JsonDisplay content={parameters} />
                    </div>
                    {!isExecuted && (
                        <div className="flex items-center gap-2">
                            <Button onClick={handleExecute} disabled={executionState === "executing"} size="sm" className="flex items-center gap-1 h-7 text-xs">
                                {executionState === "executing" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
                                执行
                            </Button>
                            <Dialog>
                                <DialogTrigger asChild>
                                    <Button variant="outline" size="sm" className="flex items-center gap-1 h-7 text-xs">
                                        <Maximize2 className="h-3 w-3" />
                                        展开
                                    </Button>
                                </DialogTrigger>
                                <DialogContent_ />
                            </Dialog>
                        </div>
                    )}
                    {isExecuted && (
                        <Dialog>
                            <DialogTrigger asChild>
                                <Button variant="outline" size="sm" className="flex items-center gap-1 h-7 text-xs">
                                    <Maximize2 className="h-3 w-3" />
                                    展开查看
                                </Button>
                            </DialogTrigger>
                            <DialogContent_ />
                        </Dialog>
                    )}
                    <div className="max-w-full overflow-hidden">
                        {renderResult()}
                    </div>
                </div>
            )}
        </div>
    );
};

export default McpToolCall;